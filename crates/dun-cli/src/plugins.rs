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

use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use dun_config::{
    KeyBinding, KeySequence, KeyStroke, Keymap, PluginEntry, PluginRole, PluginTrust,
};
use dun_core::{BufferId, EditorCommand, PluginActionKind};
use dun_plugin::{
    Capability, GrantedCapabilities, HostClient, InputSnapshot, PluginActionKind as WireActionKind,
    PluginKeybinding, PluginMenu, Policy, Role, StreamChunk, StyleSpan, TrustClass,
};
use dun_ui::{
    MenuEntry, MenuItem, built_in_menu_mnemonics, compose_translated_menu_label,
    english_menu_mnemonic, menu_label_mnemonic,
};

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
    /// A plugin action was invoked; ask the host for its surface content
    /// (`surface-write`). Carries the invoked `action_id`.
    Surface(String),
    /// Feed an output-stream chunk to the host and collect its keep/drop verdict
    /// (`stream-read`).
    Stream(StreamChunk),
    /// Submit scratch buffer text to the host (`execute`); the result comes back
    /// as a `Surface` event.
    Execute(String),
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
    /// A launch completed its handshake; carries the host's validated UI
    /// contributions when the grant allowed them.
    Started {
        menu: Option<PluginMenu>,
        keybinding: Option<PluginKeybinding>,
    },
    /// A launch failed with no job owed an answer (the eager path); job-tied
    /// launch failures surface as failed `Highlight` outcomes instead.
    StartFailed {
        error: String,
    },
    Highlight(HighlightOutcome),
    /// A surface-write action's result: the lines the host returned for its own
    /// window, or an error string. Paired with the plugin id by the poller.
    Surface {
        action_id: String,
        result: Result<Vec<String>, String>,
    },
    /// A stream-read verdict: one keep/drop boolean per fed line, or an error.
    /// The poller pairs it with the plugin id and its remembered fed lines.
    StreamVerdict {
        result: Result<Vec<bool>, String>,
    },
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
    keybinding: Option<PluginKeybinding>,
    jobs: mpsc::Sender<WorkerMessage>,
    events: mpsc::Receiver<HostEvent>,
    last_request: Option<HighlightRequestKey>,
    last_activity: Instant,
    failed: bool,
    unloaded: bool,
    /// The chunk size dun splits a stream into before feeding it (the host's
    /// `max_snapshot_lines`); a chunk over this is rejected by the client.
    stream_chunk_lines: usize,
    /// Stream chunks fed to this host (`stream-read`) awaiting a verdict, in
    /// send order. Each verdict is matched to the front chunk and applied
    /// positionally; verdicts arrive in the same order (one worker, FIFO
    /// channels).
    pending_stream: VecDeque<StreamPending>,
    /// Kept lines accumulated across the chunks of the current stream, so the
    /// surface grows chunk by chunk instead of showing only the last one.
    stream_kept: Vec<String>,
}

/// One stream chunk fed to a host, remembered until its verdict returns.
struct StreamPending {
    stream_id: String,
    chunk_index: u64,
    lines: Vec<String>,
}

impl PluginHost {
    fn from_entry(entry: &PluginEntry) -> Self {
        let policy = Policy {
            timeout: Duration::from_millis(entry.timeout_ms),
            max_frame_bytes: entry.max_frame_bytes,
            ..Policy::default()
        };
        let stream_chunk_lines = policy.max_snapshot_lines;
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
            keybinding: None,
            jobs: job_sender,
            events: event_receiver,
            last_request: None,
            last_activity: Instant::now(),
            failed: false,
            unloaded: false,
            stream_chunk_lines,
            pending_stream: VecDeque::new(),
            stream_kept: Vec::new(),
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

    /// Whether the host may fill its own surface window (`surface-write`).
    pub(crate) fn holds_surface_write(&self) -> bool {
        self.granted.holds(Capability::SurfaceWrite)
    }

    /// Whether the host may receive output-stream chunks (`stream-read`).
    pub(crate) fn holds_stream_read(&self) -> bool {
        self.granted.holds(Capability::StreamRead)
    }

    /// Whether the host owns an editable scratch window and accepts `execute`
    /// submissions (`scratch-input`).
    pub(crate) fn holds_scratch_input(&self) -> bool {
        self.granted.holds(Capability::ScratchInput)
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
        // An unloaded host contributes no UI; the contributions return with the
        // relaunch handshake.
        self.menu = None;
        self.keybinding = None;
        self.last_request = None;
        self.last_activity = Instant::now();
        self.failed = false;
        // Unload drops any in-flight stream chunks worker-side, so their
        // verdicts never arrive; clear the pending queue so a later feed's
        // verdicts are not matched against stale chunks.
        self.pending_stream.clear();
        self.stream_kept.clear();
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

    /// Ask the worker to fetch this host's surface content for an invoked
    /// action (`surface-write`). Returns whether the request was sent.
    pub(crate) fn send_surface_request(&mut self, action_id: &str) -> bool {
        self.last_activity = Instant::now();
        self.jobs
            .send(WorkerMessage::Surface(action_id.to_string()))
            .is_ok()
    }

    /// Submit the scratch buffer text to the host (`execute`). The result
    /// arrives as a `Surface` event and fills the host's surface window.
    /// Returns whether the request was sent.
    pub(crate) fn send_execute_request(&mut self, snippet: &str) -> bool {
        self.last_activity = Instant::now();
        self.jobs
            .send(WorkerMessage::Execute(snippet.to_string()))
            .is_ok()
    }

    /// Feed an output-stream chunk to the host (`stream-read`), remembering the
    /// lines so the verdict can be applied positionally. Returns whether the
    /// request was sent.
    pub(crate) fn send_stream_chunks(&mut self, stream_id: &str, lines: &[String]) -> bool {
        self.last_activity = Instant::now();
        if lines.is_empty() {
            return true;
        }
        // Split into chunks no larger than the client's line budget; the last
        // one is flagged `final_chunk`. Sending one oversized chunk is what the
        // client rejects, so a large command output must be chunked here.
        let batches: Vec<&[String]> = lines.chunks(self.stream_chunk_lines.max(1)).collect();
        let last = batches.len() - 1;
        let mut sent = true;
        for (index, batch) in batches.into_iter().enumerate() {
            self.pending_stream.push_back(StreamPending {
                stream_id: stream_id.to_string(),
                chunk_index: index as u64,
                lines: batch.to_vec(),
            });
            let chunk = StreamChunk {
                stream_id: stream_id.to_string(),
                chunk_index: index as u64,
                lines: batch.to_vec(),
                final_chunk: index == last,
            };
            sent &= self.jobs.send(WorkerMessage::Stream(chunk)).is_ok();
        }
        sent
    }

    /// Apply one stream verdict to the front pending chunk: accumulate the kept
    /// lines (resetting at `chunk_index == 0`, the start of a new stream) and
    /// return the stream id plus the lines kept so far. A verdict whose length
    /// no longer matches the chunk (a stale or racing feed) drops the chunk and
    /// returns `None`.
    pub(crate) fn apply_stream_chunk_verdict(
        &mut self,
        keep: &[bool],
    ) -> Option<(String, Vec<String>)> {
        let pending = self.pending_stream.pop_front()?;
        if keep.len() != pending.lines.len() {
            return None;
        }
        if pending.chunk_index == 0 {
            self.stream_kept.clear();
        }
        for (line, &keep) in pending.lines.into_iter().zip(keep) {
            if keep {
                self.stream_kept.push(line);
            }
        }
        Some((pending.stream_id, self.stream_kept.clone()))
    }

    /// Discard the front pending chunk without applying it — for a failed
    /// verdict, which still answers exactly one sent chunk, so the queue stays
    /// aligned with the verdict stream.
    pub(crate) fn discard_pending_stream_chunk(&mut self) {
        self.pending_stream.pop_front();
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
                HostEvent::Started { menu, keybinding } => {
                    self.menu = menu;
                    self.keybinding = keybinding;
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
                HostEvent::Surface { action_id, result } => {
                    self.failed = result.is_err();
                    kept.push(HostEvent::Surface { action_id, result });
                }
                HostEvent::StreamVerdict { result } => {
                    self.failed = result.is_err();
                    kept.push(HostEvent::StreamVerdict { result });
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
    menu_rejections: Vec<PluginMenuRejection>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ResolvedPluginMenus {
    pub(crate) items: Vec<MenuItem>,
    pub(crate) rejections: Vec<PluginMenuRejection>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PluginMenuRejection {
    pub(crate) plugin_id: String,
    pub(crate) reason: PluginMenuRejectionReason,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PluginMenuRejectionReason {
    InvalidEnglishMnemonic,
    MnemonicConflict(char),
}

impl PluginHosts {
    pub(crate) fn from_entries(entries: &[PluginEntry]) -> Self {
        Self {
            hosts: entries.iter().map(PluginHost::from_entry).collect(),
            menu_rejections: Vec::new(),
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

    /// Every accepted host menu, plus a typed rejection for each subtree whose
    /// English mnemonic is invalid or already claimed. Built-ins seed the
    /// claimed set; plugins then claim in configuration order.
    pub(crate) fn resolved_menu_items(&self, tags: &[String]) -> ResolvedPluginMenus {
        let mut resolved = ResolvedPluginMenus {
            items: Vec::new(),
            rejections: Vec::new(),
        };
        let mut claimed = built_in_menu_mnemonics().collect::<Vec<_>>();
        for (plugin_id, menu) in self.menus() {
            let english_label = menu.top_label.fallback();
            let mnemonic = match top_level_mnemonic(menu, english_label) {
                Some(mnemonic) => mnemonic,
                None => {
                    resolved.rejections.push(PluginMenuRejection {
                        plugin_id: plugin_id.to_string(),
                        reason: PluginMenuRejectionReason::InvalidEnglishMnemonic,
                    });
                    continue;
                }
            };
            if claimed.contains(&mnemonic) {
                resolved.rejections.push(PluginMenuRejection {
                    plugin_id: plugin_id.to_string(),
                    reason: PluginMenuRejectionReason::MnemonicConflict(mnemonic),
                });
                continue;
            }
            claimed.push(mnemonic);
            resolved
                .items
                .push(resolve_plugin_menu(plugin_id, menu, tags, mnemonic));
        }
        resolved
    }

    pub(crate) fn replace_menu_rejections(
        &mut self,
        rejections: Vec<PluginMenuRejection>,
    ) -> Vec<PluginMenuRejection> {
        let newly_rejected = rejections
            .iter()
            .filter(|rejection| !self.menu_rejections.contains(*rejection))
            .cloned()
            .collect();
        self.menu_rejections = rejections;
        newly_rejected
    }

    #[cfg(test)]
    pub(crate) fn menu_rejections(&self) -> &[PluginMenuRejection] {
        &self.menu_rejections
    }

    /// Keybinding contributions gathered from every host that advertised one
    /// under the `keybinding` grant, in configuration order.
    fn keybindings(&self) -> impl Iterator<Item = (&str, &PluginKeybinding)> {
        self.hosts.iter().filter_map(|host| {
            host.keybinding
                .as_ref()
                .map(|keybinding| (host.plugin_id.as_str(), keybinding))
        })
    }

    /// Every host's keybinding contribution resolved into a plugin keymap of
    /// `[leader, chord] -> PluginAction` bindings, consulted after the built-in
    /// keymap. `base` is the editor's live keymap: a plugin leader that fails to
    /// parse, is already a binding or the prefix of one in `base`, or was
    /// already claimed by an earlier plugin this pass is rejected — its whole
    /// contribution is dropped — so a plugin can never shadow an existing
    /// binding or another plugin's leader.
    pub(crate) fn resolved_keybindings(&self, base: &Keymap) -> (Keymap, Vec<String>, bool) {
        let mut bindings: Vec<KeyBinding> = Vec::new();
        let mut claimed_chords: Vec<KeyStroke> = Vec::new();
        let mut rejected: Vec<String> = Vec::new();
        let Some(leader) = plugin_leader(base) else {
            // The reserved leader is unusable because the editor's own keymap
            // took it. Report every contribution rather than binding some of
            // them somewhere unexpected.
            return (
                Keymap {
                    bindings: Vec::new(),
                },
                self.keybindings()
                    .map(|(plugin_id, _)| plugin_id.to_string())
                    .collect(),
                true,
            );
        };
        for (plugin_id, keybinding) in self.keybindings() {
            match resolve_plugin_keybinding(plugin_id, keybinding, leader, &claimed_chords) {
                Some(resolved) => {
                    claimed_chords.extend(resolved.chords);
                    bindings.extend(resolved.bindings);
                }
                // A host that advertised a keybinding but was rejected (leader
                // collision, already claimed, or unparseable) is reported so it
                // is not a silent no-op.
                None => rejected.push(plugin_id.to_string()),
            }
        }
        (Keymap { bindings }, rejected, false)
    }
}

/// The one leader prefix every plugin binds under.
///
/// Reserving a single editor-owned prefix is what makes a plugin binding
/// structurally unable to shadow an editor key — the property per-plugin
/// leaders could only ever check for, never guarantee. `Ctrl+T` is the choice
/// because the free Ctrl-letters are I, J, M, T and U, and three of those are
/// unreachable in a terminal: Ctrl+I is byte 0x09 (Tab), Ctrl+M is 0x0D
/// (Enter) and Ctrl+J is 0x0A, all matched before the Ctrl-letter branch in
/// `terminal/vt/parser`. T was already the reference host's own pick.
pub(crate) const PLUGIN_LEADER: &str = "Ctrl+T";

/// The reserved leader, unless the editor's own keymap has claimed it — a
/// user may rebind anything, so this is checked rather than assumed.
pub(crate) fn plugin_leader(base: &Keymap) -> Option<KeyStroke> {
    let leader: KeyStroke = PLUGIN_LEADER.parse().ok()?;
    let sequence = KeySequence::single(leader);
    let free =
        base.command_for_sequence(&sequence).is_none() && !base.has_sequence_prefix(&sequence);
    free.then_some(leader)
}

struct ResolvedKeybinding {
    chords: Vec<KeyStroke>,
    bindings: Vec<KeyBinding>,
}

/// Parse and collision-check one host's keybinding contribution. Returns `None`
/// — dropping the whole contribution — when the leader is unparseable, collides
/// with the base keymap, was already claimed this pass, or any chord key fails
/// to parse.
fn resolve_plugin_keybinding(
    plugin_id: &str,
    keybinding: &PluginKeybinding,
    leader: KeyStroke,
    claimed_chords: &[KeyStroke],
) -> Option<ResolvedKeybinding> {
    // Every chord must be free under the shared leader. A plugin whose chord
    // an earlier plugin already claimed loses its whole contribution rather
    // than landing half-bound: a host that believes it owns `<leader> f` and
    // silently does not is worse than one that is told it was rejected. The
    // user resolves it by editing that plugin's own config, which is where a
    // plugin's settings live.
    let mut bindings = Vec::with_capacity(keybinding.chords.len());
    let mut taken = Vec::with_capacity(keybinding.chords.len());
    for chord in &keybinding.chords {
        let key: KeyStroke = chord.key.parse().ok()?;
        if claimed_chords.contains(&key) || taken.contains(&key) {
            return None;
        }
        taken.push(key);
        bindings.push(KeyBinding {
            sequence: KeySequence {
                strokes: vec![leader, key],
            },
            command: EditorCommand::PluginAction {
                plugin_id: plugin_id.to_string(),
                action_id: chord.action_id.clone(),
                kind: action_kind(chord.kind),
            },
        });
    }
    Some(ResolvedKeybinding {
        chords: taken,
        bindings,
    })
}

/// Map the wire action kind to the dun-core kind (two identical enums kept in
/// separate crates because `dun-plugin` has no `dun-core` dependency).
fn action_kind(kind: WireActionKind) -> PluginActionKind {
    match kind {
        WireActionKind::Surface => PluginActionKind::Surface,
        WireActionKind::Scratch => PluginActionKind::Scratch,
        WireActionKind::Execute => PluginActionKind::Execute,
    }
}

/// Resolve a validated plugin menu contribution into a dun-ui menu item. Each
/// entry carries a [`EditorCommand::PluginAction`] tagged by `plugin_id`
/// and the item's `action_id`, so dispatch can route the invocation back to
/// the owning host. Labels are resolved against the active locale `tags`; the
/// display sanitizer still runs at render time.
fn resolve_plugin_menu(
    plugin_id: &str,
    menu: &PluginMenu,
    tags: &[String],
    mnemonic: char,
) -> MenuItem {
    // Entry mnemonics are author-chosen or absent — never derived. Duplicates
    // within one menu drop only the *later* entry's shortcut, not the entry
    // and not its siblings: a dropdown item stays reachable by arrows, Enter
    // and the mouse, so silently removing it would be a worse trade than
    // losing one letter. (A top-level collision is different and rejects the
    // whole subtree, because there the menu becomes unreachable entirely.)
    let mut claimed_entry_mnemonics: Vec<char> = Vec::new();
    let entries = menu
        .items
        .iter()
        .map(|item| {
            let base = item.label.resolve(tags);
            let label = match item.mnemonic {
                // Always composed, never conditionally: `entry_mnemonic` reads
                // ONLY a trailing `(M)` and has no first-character fallback,
                // so an entry whose suffix is omitted has no working key even
                // when its text starts with that very letter.
                Some(mnemonic) if !claimed_entry_mnemonics.contains(&mnemonic) => {
                    claimed_entry_mnemonics.push(mnemonic);
                    compose_translated_menu_label(base, mnemonic)
                }
                _ => base.to_string(),
            };
            MenuEntry::new(
                label,
                EditorCommand::PluginAction {
                    plugin_id: plugin_id.to_string(),
                    action_id: item.action_id.clone(),
                    kind: action_kind(item.kind),
                },
            )
        })
        .collect();
    let translation = menu.top_label.resolve_translation(tags);
    let base = translation.unwrap_or_else(|| menu.top_label.fallback());
    MenuItem::new(
        top_level_label(base, translation.is_some(), mnemonic),
        entries,
    )
}

/// Render a **top-level** label so it and the matcher agree about the mnemonic.
///
/// Two reasons to append `(M)`, and they are not the same reason:
///
/// - a translation is actively selected. Then the suffix goes on even when the
///   translated text happens to equal the English, because every other menu in
///   a translated UI carries one and a bare plugin label would read as having
///   no key at all;
/// - the rendered text would not resolve to this mnemonic anyway — a
///   translated label (`日志过滤`), or an author-declared letter that is not
///   the first one (`Log Filter` asking for `G`). Without the suffix the
///   declared key would simply not work, since the top-level matcher falls
///   back to the label's first character.
///
/// Plain English whose first letter already *is* the mnemonic gets nothing,
/// so `Log Filter` stays `Log Filter` exactly as `File` stays `File`.
fn top_level_label(base: &str, translated: bool, mnemonic: char) -> String {
    let already_matches =
        menu_label_mnemonic(base).is_some_and(|derived| derived.eq_ignore_ascii_case(&mnemonic));
    if translated || !already_matches {
        compose_translated_menu_label(base, mnemonic)
    } else {
        base.to_string()
    }
}

/// The top-level mnemonic: the host's choice if it declared one, else derived.
///
/// Unlike dropdown entries this one still derives when absent, because a
/// top-level menu without a mnemonic cannot be opened from the keyboard at
/// all. An author-declared mnemonic is taken as-is — it is already validated
/// as a single non-parenthesis ASCII graphic by the protocol layer — and only
/// the collision check in the caller can reject it.
fn top_level_mnemonic(menu: &PluginMenu, english_label: &str) -> Option<char> {
    menu.top_mnemonic
        .or_else(|| valid_plugin_menu_mnemonic(english_label))
}

/// A plugin menu that declared no mnemonic falls back to its first English
/// ASCII letter. A raw English label may already carry a parenthesized
/// mnemonic; accept it only when that suffix agrees with the first-letter
/// rule, because dun-ui's matcher prefers it.
fn valid_plugin_menu_mnemonic(label: &str) -> Option<char> {
    let mnemonic = english_menu_mnemonic(label)?;
    if trailing_parenthesized_mnemonic(label)
        .is_some_and(|embedded| !embedded.eq_ignore_ascii_case(&mnemonic))
    {
        return None;
    }
    Some(mnemonic)
}

fn trailing_parenthesized_mnemonic(label: &str) -> Option<char> {
    let without_close = label.trim_end().strip_suffix(')')?;
    let (_, contents) = without_close.rsplit_once('(')?;
    let mut chars = contents.chars();
    let mnemonic = chars.next()?;
    chars.next().is_none().then_some(mnemonic)
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

/// One gathered round of worker input: the newest highlight job wins, every
/// surface action and stream chunk is kept in order, plus whether an eager
/// launch was requested.
struct WorkerAction {
    launch: bool,
    job: Option<HighlightJob>,
    surface: Vec<String>,
    stream: Vec<StreamChunk>,
    execute: Vec<String>,
}

impl WorkerAction {
    fn is_empty(&self) -> bool {
        !self.launch
            && self.job.is_none()
            && self.surface.is_empty()
            && self.stream.is_empty()
            && self.execute.is_empty()
    }
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
        if action.is_empty() {
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
                        keybinding: launched.keybinding().cloned(),
                    });
                    client = Some(launched);
                    last_failure = None;
                }
                Err(error) => {
                    last_failure = Some(Instant::now());
                    report_launch_failure(&action, &error.to_string(), events);
                    continue;
                }
            }
        }

        if let Some(job) = action.job {
            serve_job(&mut client, job, events, &mut last_failure);
        }
        for action_id in action.surface {
            serve_surface(&mut client, action_id, events, &mut last_failure);
        }
        for chunk in action.stream {
            serve_stream(&mut client, chunk, events, &mut last_failure);
        }
        for snippet in action.execute {
            serve_execute(&mut client, snippet, events, &mut last_failure);
        }
    }
}

/// Report a launch failure to every piece of work the round owed an answer: a
/// failed highlight job, each surface action, or — for a bare eager launch —
/// a single `StartFailed`.
fn report_launch_failure(action: &WorkerAction, error: &str, events: &mpsc::Sender<HostEvent>) {
    let mut reported = false;
    if let Some(job) = &action.job {
        let _ = events.send(HostEvent::Highlight(failure_outcome(job, error)));
        reported = true;
    }
    for action_id in &action.surface {
        let _ = events.send(HostEvent::Surface {
            action_id: action_id.clone(),
            result: Err(error.to_string()),
        });
        reported = true;
    }
    for _ in &action.stream {
        let _ = events.send(HostEvent::StreamVerdict {
            result: Err(error.to_string()),
        });
        reported = true;
    }
    for _ in &action.execute {
        let _ = events.send(HostEvent::Surface {
            action_id: "execute".to_string(),
            result: Err(error.to_string()),
        });
        reported = true;
    }
    if !reported {
        let _ = events.send(HostEvent::StartFailed {
            error: error.to_string(),
        });
    }
}

/// Run one highlight job against the launched client. A protocol violation
/// kills the host (dropping the client) so the next job relaunches after the
/// cooldown.
fn serve_job(
    client: &mut Option<HostClient>,
    job: HighlightJob,
    events: &mpsc::Sender<HostEvent>,
    last_failure: &mut Option<Instant>,
) {
    let Some(active) = client.as_mut() else {
        return;
    };
    let Ok(first_line) = u32::try_from(job.first_line) else {
        let _ = events.send(HostEvent::Highlight(failure_outcome(
            &job,
            "snapshot start exceeds u32 lines",
        )));
        return;
    };
    let snapshot = InputSnapshot {
        buffer_revision: job.revision,
        language: job.language.clone(),
        first_line,
        lines: job.lines.clone(),
    };
    match active.request_highlight(&snapshot) {
        Ok(spans) => {
            *last_failure = None;
            let _ = events.send(HostEvent::Highlight(HighlightOutcome {
                buffer_id: job.buffer_id,
                revision: job.revision,
                result: Ok(spans),
            }));
        }
        Err(error) => {
            *client = None;
            *last_failure = Some(Instant::now());
            let _ = events.send(HostEvent::Highlight(failure_outcome(
                &job,
                &error.to_string(),
            )));
        }
    }
}

/// Run one surface-write action request against the launched client. Like
/// `serve_job`, a violation kills the host so the next request relaunches.
fn serve_surface(
    client: &mut Option<HostClient>,
    action_id: String,
    events: &mpsc::Sender<HostEvent>,
    last_failure: &mut Option<Instant>,
) {
    let Some(active) = client.as_mut() else {
        let _ = events.send(HostEvent::Surface {
            action_id,
            result: Err("plugin host unavailable".to_string()),
        });
        return;
    };
    match active.request_surface(&action_id) {
        Ok(lines) => {
            *last_failure = None;
            let _ = events.send(HostEvent::Surface {
                action_id,
                result: Ok(lines),
            });
        }
        Err(error) => {
            *client = None;
            *last_failure = Some(Instant::now());
            let _ = events.send(HostEvent::Surface {
                action_id,
                result: Err(error.to_string()),
            });
        }
    }
}

/// Run one stream-read chunk against the launched client. Like `serve_job`, a
/// violation kills the host so the next request relaunches.
fn serve_stream(
    client: &mut Option<HostClient>,
    chunk: StreamChunk,
    events: &mpsc::Sender<HostEvent>,
    last_failure: &mut Option<Instant>,
) {
    let Some(active) = client.as_mut() else {
        let _ = events.send(HostEvent::StreamVerdict {
            result: Err("plugin host unavailable".to_string()),
        });
        return;
    };
    match active.request_stream_filter(&chunk) {
        Ok(keep) => {
            *last_failure = None;
            let _ = events.send(HostEvent::StreamVerdict { result: Ok(keep) });
        }
        Err(error) => {
            *client = None;
            *last_failure = Some(Instant::now());
            let _ = events.send(HostEvent::StreamVerdict {
                result: Err(error.to_string()),
            });
        }
    }
}

/// Submit one scratch snippet to the launched client (`execute`) and report the
/// host's result lines as a `Surface` event so they fill the surface window.
/// Like `serve_job`, a violation kills the host so the next request relaunches.
fn serve_execute(
    client: &mut Option<HostClient>,
    snippet: String,
    events: &mpsc::Sender<HostEvent>,
    last_failure: &mut Option<Instant>,
) {
    let Some(active) = client.as_mut() else {
        let _ = events.send(HostEvent::Surface {
            action_id: "execute".to_string(),
            result: Err("plugin host unavailable".to_string()),
        });
        return;
    };
    match active.request_execute(&snippet) {
        Ok(lines) => {
            *last_failure = None;
            let _ = events.send(HostEvent::Surface {
                action_id: "execute".to_string(),
                result: Ok(lines),
            });
        }
        Err(error) => {
            *client = None;
            *last_failure = Some(Instant::now());
            let _ = events.send(HostEvent::Surface {
                action_id: "execute".to_string(),
                result: Err(error.to_string()),
            });
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
        surface: Vec::new(),
        stream: Vec::new(),
        execute: Vec::new(),
    };
    apply_worker_message(messages.recv()?, client, unloaded, &mut action);
    while let Ok(message) = messages.try_recv() {
        apply_worker_message(message, client, unloaded, &mut action);
    }

    if *unloaded {
        Ok(WorkerAction {
            launch: false,
            job: None,
            surface: Vec::new(),
            stream: Vec::new(),
            execute: Vec::new(),
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
        WorkerMessage::Surface(action_id) => action.surface.push(action_id),
        WorkerMessage::Stream(chunk) => action.stream.push(chunk),
        WorkerMessage::Execute(snippet) => action.execute.push(snippet),
        WorkerMessage::Launch => action.launch = true,
        WorkerMessage::Load => *unloaded = false,
        WorkerMessage::Unload => {
            if let Some(active) = client.take() {
                let _ = active.shutdown();
            }
            *unloaded = true;
            action.launch = false;
            action.job = None;
            action.surface.clear();
            action.stream.clear();
            action.execute.clear();
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
                keybinding: None,
                jobs: job_sender,
                events: event_receiver,
                last_request: None,
                last_activity: Instant::now(),
                failed: false,
                unloaded: false,
                stream_chunk_lines: Policy::default().max_snapshot_lines,
                pending_stream: VecDeque::new(),
                stream_kept: Vec::new(),
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
        Self {
            hosts,
            menu_rejections: Vec::new(),
        }
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
