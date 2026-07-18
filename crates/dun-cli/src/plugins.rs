//! Editor-side plugin wiring: maps configured plugin hosts onto the
//! `dun-plugin` protocol client and runs each host on a worker thread so a
//! slow or hostile host can never block the event loop.
//!
//! Every configured entry gets a [`PluginHost`], collected in [`PluginHosts`].
//! Syntax highlighting is one facet, routed to the first host declaring the
//! `syntax-highlight` role (the pre-generalization selection rule); a menu
//! contribution is captured from any host the grant allows one. Launch timing
//! is hybrid: a host granted `menu` or `window` launches eagerly at startup,
//! because only its handshake can advertise the UI it contributes, while
//! highlight-only hosts keep the memory-saving lazy launch on their first job.

use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use dun_config::{PluginEntry, PluginRole, PluginTrust};
use dun_core::{BufferId, EditorCommand};
use dun_plugin::{
    Capability, GrantedCapabilities, HostClient, InputSnapshot, PluginMenu, Policy, Role,
    StyleSpan, TrustClass,
};
use dun_ui::{MenuEntry, MenuItem};

/// Do not relaunch a failed host more often than this; failures otherwise
/// turn every editor tick into a spawn attempt.
const RELAUNCH_COOLDOWN: Duration = Duration::from_secs(5);

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct HighlightJob {
    pub(crate) buffer_id: BufferId,
    pub(crate) revision: u64,
    pub(crate) language: String,
    pub(crate) first_line: usize,
    pub(crate) lines: Vec<String>,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum WorkerMessage {
    Job(HighlightJob),
    /// Launch the host now if it is not running: the eager path for hosts
    /// whose handshake carries UI contributions.
    Launch,
    Load,
    Unload,
}

#[derive(Debug)]
pub(crate) struct HighlightOutcome {
    pub(crate) buffer_id: BufferId,
    pub(crate) revision: u64,
    pub(crate) result: Result<Vec<StyleSpan>, String>,
}

/// What a worker reports back to the main thread.
#[derive(Debug)]
pub(crate) enum HostEvent {
    /// A launch completed its handshake; carries the host's validated menu
    /// contribution when the grant allowed one.
    Started {
        menu: Option<PluginMenu>,
    },
    /// A launch failed with no job owed an answer (the eager path); job-tied
    /// launch failures surface as failed `Highlight` outcomes instead.
    StartFailed {
        error: String,
    },
    Highlight(HighlightOutcome),
}

/// A request key identifying work already in flight or applied; used to
/// avoid re-sending the same snapshot every tick.
pub(crate) type HighlightRequestKey = (BufferId, u64, usize, usize);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PluginActivity {
    Off,
    Active,
    Idle,
    Error,
}

/// One configured host entry: its worker channel, granted capabilities, and
/// the UI state the handshake delivered. Host process I/O lives on the worker
/// thread; this side never blocks.
pub(crate) struct PluginHost {
    plugin_id: String,
    roles: Vec<Role>,
    granted: GrantedCapabilities,
    menu: Option<PluginMenu>,
    jobs: mpsc::Sender<WorkerMessage>,
    events: mpsc::Receiver<HostEvent>,
    last_request: Option<HighlightRequestKey>,
    last_activity: Instant,
    failed: bool,
    unloaded: bool,
}

impl PluginHost {
    fn from_entry(entry: &PluginEntry) -> Self {
        let policy = Policy {
            timeout: Duration::from_millis(entry.timeout_ms),
            max_frame_bytes: entry.max_frame_bytes,
            ..Policy::default()
        };
        let roles: Vec<Role> = entry
            .roles
            .iter()
            .copied()
            .filter_map(plugin_role)
            .collect();
        let trust = plugin_trust(entry.trust);
        let granted = GrantedCapabilities::for_roles(&roles, trust);
        let (job_sender, job_receiver) = mpsc::channel::<WorkerMessage>();
        let (event_sender, event_receiver) = mpsc::channel::<HostEvent>();
        let command = entry.command.clone();
        let plugin_id = entry.id.clone();
        let worker_plugin_id = plugin_id.clone();
        let worker_roles = roles.clone();
        thread::spawn(move || {
            host_worker(
                &command,
                &worker_plugin_id,
                policy,
                &worker_roles,
                trust,
                &job_receiver,
                &event_sender,
            );
        });

        let host = Self {
            plugin_id,
            roles,
            granted,
            menu: None,
            jobs: job_sender,
            events: event_receiver,
            last_request: None,
            last_activity: Instant::now(),
            failed: false,
            unloaded: false,
        };
        if host.launches_eagerly() {
            let _ = host.jobs.send(WorkerMessage::Launch);
        }
        host
    }

    pub(crate) fn plugin_id(&self) -> &str {
        &self.plugin_id
    }

    /// Whether the host launches without waiting for work: a host granted a
    /// UI-contributing capability must handshake at startup to advertise it.
    pub(crate) fn launches_eagerly(&self) -> bool {
        self.granted.holds(Capability::Menu) || self.granted.holds(Capability::Window)
    }

    /// Whether the host may open and own `dun` windows. The `menu` grant lets
    /// its menu items appear; opening a surface is separately gated on this.
    pub(crate) fn holds_window(&self) -> bool {
        self.granted.holds(Capability::Window)
    }

    fn highlights(&self) -> bool {
        self.roles.contains(&Role::SyntaxHighlight)
    }

    pub(crate) fn is_loaded(&self) -> bool {
        !self.unloaded
    }

    pub(crate) fn unload(&mut self) {
        let _ = self.jobs.send(WorkerMessage::Unload);
        self.unloaded = true;
        // An unloaded host contributes no UI; the menu returns with the
        // relaunch handshake.
        self.menu = None;
        self.last_request = None;
        self.last_activity = Instant::now();
        self.failed = false;
    }

    pub(crate) fn load(&mut self) {
        let _ = self.jobs.send(WorkerMessage::Load);
        if self.launches_eagerly() {
            let _ = self.jobs.send(WorkerMessage::Launch);
        }
        self.unloaded = false;
        self.last_request = None;
        self.last_activity = Instant::now();
        self.failed = false;
    }

    /// Sends a job unless an identical snapshot (buffer, revision, window)
    /// was already requested. Returns whether a job was sent.
    pub(crate) fn schedule(&mut self, job: HighlightJob) -> bool {
        let key = (job.buffer_id, job.revision, job.first_line, job.lines.len());
        if self.last_request == Some(key) {
            return false;
        }
        self.last_request = Some(key);
        self.last_activity = Instant::now();
        // A send error means the worker died; the next poll simply yields
        // nothing and the error was already reported as an event.
        self.jobs.send(WorkerMessage::Job(job)).is_ok()
    }

    /// Drains worker events. Handshake results are absorbed here (the menu
    /// contribution installs or reinstalls); events the application layer
    /// must act on — launch failures, highlight outcomes — are returned.
    pub(crate) fn poll(&mut self) -> Vec<HostEvent> {
        let mut kept = Vec::new();
        let mut any = false;
        for event in self.events.try_iter() {
            any = true;
            match event {
                HostEvent::Started { menu } => {
                    self.menu = menu;
                    self.failed = false;
                }
                HostEvent::StartFailed { .. } => {
                    self.failed = true;
                    kept.push(event);
                }
                HostEvent::Highlight(outcome) => {
                    self.failed = outcome.result.is_err();
                    kept.push(HostEvent::Highlight(outcome));
                }
            }
        }
        if any {
            self.last_activity = Instant::now();
        }
        kept
    }

    pub(crate) fn activity_at(&self, now: Instant, idle_after: Option<Duration>) -> PluginActivity {
        if !self.is_loaded() {
            return PluginActivity::Off;
        }
        if self.failed {
            return PluginActivity::Error;
        }
        match idle_after {
            Some(threshold) if now.saturating_duration_since(self.last_activity) >= threshold => {
                PluginActivity::Idle
            }
            _ => PluginActivity::Active,
        }
    }
}

/// Every configured host, in configuration order.
pub(crate) struct PluginHosts {
    hosts: Vec<PluginHost>,
}

impl PluginHosts {
    pub(crate) fn from_entries(entries: &[PluginEntry]) -> Self {
        Self {
            hosts: entries.iter().map(PluginHost::from_entry).collect(),
        }
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.hosts.is_empty()
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = &PluginHost> {
        self.hosts.iter()
    }

    pub(crate) fn iter_mut(&mut self) -> impl Iterator<Item = &mut PluginHost> {
        self.hosts.iter_mut()
    }

    pub(crate) fn get(&self, plugin_id: &str) -> Option<&PluginHost> {
        self.hosts.iter().find(|host| host.plugin_id == plugin_id)
    }

    pub(crate) fn get_mut(&mut self, plugin_id: &str) -> Option<&mut PluginHost> {
        self.hosts
            .iter_mut()
            .find(|host| host.plugin_id == plugin_id)
    }

    /// The addressee of a bare `plugin load`/`unload`: unambiguous only when
    /// exactly one host is configured.
    pub(crate) fn only_host_mut(&mut self) -> Option<&mut PluginHost> {
        match self.hosts.as_mut_slice() {
            [host] => Some(host),
            _ => None,
        }
    }

    /// The host highlight snapshots are routed to: the first configured
    /// entry declaring the `syntax-highlight` role.
    pub(crate) fn highlighter(&self) -> Option<&PluginHost> {
        self.hosts.iter().find(|host| host.highlights())
    }

    pub(crate) fn highlighter_mut(&mut self) -> Option<&mut PluginHost> {
        self.hosts.iter_mut().find(|host| host.highlights())
    }

    /// Menu contributions gathered from every host that advertised one under
    /// the `menu` grant, in configuration order.
    pub(crate) fn menus(&self) -> impl Iterator<Item = (&str, &PluginMenu)> {
        self.hosts.iter().filter_map(|host| {
            host.menu
                .as_ref()
                .map(|menu| (host.plugin_id.as_str(), menu))
        })
    }

    /// Every host's menu contribution resolved into a dun-ui menu item, in
    /// configuration order, ready to append after the built-in menus. `tags`
    /// is the active locale chain used to pick each label (empty falls back to
    /// the required `en_US`).
    pub(crate) fn resolved_menu_items(&self, tags: &[String]) -> Vec<MenuItem> {
        self.menus()
            .map(|(plugin_id, menu)| resolve_plugin_menu(plugin_id, menu, tags))
            .collect()
    }
}

/// Resolve a validated plugin menu contribution into a dun-ui menu item. Each
/// entry carries a [`EditorCommand::PluginMenuAction`] tagged by `plugin_id`
/// and the item's `action_id`, so dispatch can route the invocation back to
/// the owning host. Labels are resolved against the active locale `tags`; the
/// display sanitizer still runs at render time.
fn resolve_plugin_menu(plugin_id: &str, menu: &PluginMenu, tags: &[String]) -> MenuItem {
    let entries = menu
        .items
        .iter()
        .map(|item| {
            MenuEntry::new(
                item.label.resolve(tags).to_string(),
                EditorCommand::PluginMenuAction {
                    plugin_id: plugin_id.to_string(),
                    action_id: item.action_id.clone(),
                },
            )
        })
        .collect();
    MenuItem::new(menu.top_label.resolve(tags).to_string(), entries)
}

/// Map a configured role name to the protocol `Role`, if the protocol models
/// it yet. Config accepts role names ahead of the protocol client
/// (`TextTransform`, `ConfigHelper` have no `Role` variant yet); those grant
/// no capabilities until their slice lands, so they map to `None`.
fn plugin_role(role: PluginRole) -> Option<Role> {
    match role {
        PluginRole::SyntaxHighlight => Some(Role::SyntaxHighlight),
        PluginRole::LogFilter => Some(Role::LogFilter),
        PluginRole::TextTransform | PluginRole::ConfigHelper => None,
    }
}

fn plugin_trust(trust: PluginTrust) -> TrustClass {
    match trust {
        PluginTrust::PureSandbox => TrustClass::PureSandbox,
        PluginTrust::UserTrustedExternal => TrustClass::UserTrustedExternal,
    }
}

/// One gathered round of worker input: the newest job wins, plus whether an
/// eager launch was requested.
struct WorkerAction {
    launch: bool,
    job: Option<HighlightJob>,
}

fn host_worker(
    command: &Path,
    plugin_id: &str,
    policy: Policy,
    roles: &[Role],
    trust: TrustClass,
    messages: &mpsc::Receiver<WorkerMessage>,
    events: &mpsc::Sender<HostEvent>,
) {
    let mut client: Option<HostClient> = None;
    let mut last_failure: Option<Instant> = None;
    let mut unloaded = false;

    while let Ok(action) = next_worker_action(messages, &mut client, &mut unloaded) {
        if !action.launch && action.job.is_none() {
            continue;
        }

        if client.is_none() {
            if last_failure.is_some_and(|failed| failed.elapsed() < RELAUNCH_COOLDOWN) {
                continue;
            }
            match HostClient::launch(command, plugin_id, policy.clone(), roles, trust) {
                Ok(launched) => {
                    let _ = events.send(HostEvent::Started {
                        menu: launched.menu().cloned(),
                    });
                    client = Some(launched);
                    last_failure = None;
                }
                Err(error) => {
                    last_failure = Some(Instant::now());
                    let event = match &action.job {
                        Some(job) => HostEvent::Highlight(failure_outcome(job, &error.to_string())),
                        None => HostEvent::StartFailed {
                            error: error.to_string(),
                        },
                    };
                    let _ = events.send(event);
                    continue;
                }
            }
        }
        let Some(active) = client.as_mut() else {
            continue;
        };
        let Some(job) = action.job else {
            continue;
        };

        let Ok(first_line) = u32::try_from(job.first_line) else {
            let _ = events.send(HostEvent::Highlight(failure_outcome(
                &job,
                "snapshot start exceeds u32 lines",
            )));
            continue;
        };
        let snapshot = InputSnapshot {
            buffer_revision: job.revision,
            language: job.language.clone(),
            first_line,
            lines: job.lines.clone(),
        };
        match active.request_highlight(&snapshot) {
            Ok(spans) => {
                last_failure = None;
                let _ = events.send(HostEvent::Highlight(HighlightOutcome {
                    buffer_id: job.buffer_id,
                    revision: job.revision,
                    result: Ok(spans),
                }));
            }
            Err(error) => {
                // The client kills the host on protocol violations; drop it
                // so the next job relaunches after the cooldown.
                client = None;
                last_failure = Some(Instant::now());
                let _ = events.send(HostEvent::Highlight(failure_outcome(
                    &job,
                    &error.to_string(),
                )));
            }
        }
    }
}

fn next_worker_action(
    messages: &mpsc::Receiver<WorkerMessage>,
    client: &mut Option<HostClient>,
    unloaded: &mut bool,
) -> Result<WorkerAction, mpsc::RecvError> {
    let mut action = WorkerAction {
        launch: false,
        job: None,
    };
    apply_worker_message(messages.recv()?, client, unloaded, &mut action);
    while let Ok(message) = messages.try_recv() {
        apply_worker_message(message, client, unloaded, &mut action);
    }

    if *unloaded {
        Ok(WorkerAction {
            launch: false,
            job: None,
        })
    } else {
        Ok(action)
    }
}

fn apply_worker_message(
    message: WorkerMessage,
    client: &mut Option<HostClient>,
    unloaded: &mut bool,
    action: &mut WorkerAction,
) {
    match message {
        WorkerMessage::Job(job) => action.job = Some(job),
        WorkerMessage::Launch => action.launch = true,
        WorkerMessage::Load => *unloaded = false,
        WorkerMessage::Unload => {
            if let Some(active) = client.take() {
                let _ = active.shutdown();
            }
            *unloaded = true;
            action.launch = false;
            action.job = None;
        }
    }
}

fn failure_outcome(job: &HighlightJob, message: &str) -> HighlightOutcome {
    HighlightOutcome {
        buffer_id: job.buffer_id,
        revision: job.revision,
        result: Err(message.to_string()),
    }
}

/// Language hint passed to highlight hosts: the lowercased file extension,
/// or an empty string for untitled/extension-less buffers. Interpretation
/// is the host's job; the editor does not maintain a language table.
pub(crate) fn language_hint(path: Option<&PathBuf>) -> String {
    path.and_then(|path| path.extension())
        .and_then(|extension| extension.to_str())
        .map(str::to_ascii_lowercase)
        .unwrap_or_default()
}

#[cfg(test)]
impl PluginHost {
    /// Test constructor without a worker thread; returns the message receiver
    /// and event sender so tests can observe worker traffic and inject
    /// events. The host carries the `syntax-highlight` role and no grants.
    pub(crate) fn for_tests() -> (Self, mpsc::Receiver<WorkerMessage>, mpsc::Sender<HostEvent>) {
        Self::for_tests_granted("test-plugin", GrantedCapabilities::default())
    }

    pub(crate) fn for_tests_granted(
        plugin_id: &str,
        granted: GrantedCapabilities,
    ) -> (Self, mpsc::Receiver<WorkerMessage>, mpsc::Sender<HostEvent>) {
        let (job_sender, job_receiver) = mpsc::channel::<WorkerMessage>();
        let (event_sender, event_receiver) = mpsc::channel::<HostEvent>();
        (
            Self {
                plugin_id: plugin_id.to_string(),
                roles: vec![Role::SyntaxHighlight],
                granted,
                menu: None,
                jobs: job_sender,
                events: event_receiver,
                last_request: None,
                last_activity: Instant::now(),
                failed: false,
                unloaded: false,
            },
            job_receiver,
            event_sender,
        )
    }

    pub(crate) fn set_last_activity_for_tests(&mut self, last_activity: Instant) {
        self.last_activity = last_activity;
    }
}

#[cfg(test)]
impl PluginHosts {
    pub(crate) fn for_tests(hosts: Vec<PluginHost>) -> Self {
        Self { hosts }
    }
}

#[cfg(test)]
pub(crate) fn next_worker_action_for_tests(
    messages: &mpsc::Receiver<WorkerMessage>,
    unloaded: &mut bool,
) -> Result<(bool, Option<HighlightJob>), mpsc::RecvError> {
    let mut client = None;
    next_worker_action(messages, &mut client, unloaded).map(|action| (action.launch, action.job))
}
