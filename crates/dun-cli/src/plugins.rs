//! Editor-side plugin wiring: maps configured plugin hosts onto the
//! `dun-plugin` protocol client and runs each host on a worker thread so a
//! slow or hostile host can never block the event loop.
//!
//! v1 scope: one `SyntaxHighlight` host (the first configured entry with
//! that role), focused-buffer visible-window snapshots, and stale-revision
//! discard at both the client and the application layer.

use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use dun_config::{PluginEntry, PluginRole};
use dun_core::BufferId;
use dun_plugin::{HostClient, InputSnapshot, Policy, StyleSpan};

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

#[derive(Debug)]
pub(crate) struct HighlightOutcome {
    pub(crate) buffer_id: BufferId,
    pub(crate) revision: u64,
    pub(crate) result: Result<Vec<StyleSpan>, String>,
}

/// A request key identifying work already in flight or applied; used to
/// avoid re-sending the same snapshot every tick.
pub(crate) type HighlightRequestKey = (BufferId, u64, usize, usize);

pub(crate) struct PluginHighlighter {
    plugin_id: String,
    jobs: mpsc::Sender<HighlightJob>,
    outcomes: mpsc::Receiver<HighlightOutcome>,
    last_request: Option<HighlightRequestKey>,
}

impl PluginHighlighter {
    /// Builds the highlighter for the first configured entry advertising
    /// the `syntax-highlight` role, if any.
    pub(crate) fn from_entries(entries: &[PluginEntry]) -> Option<Self> {
        let entry = entries
            .iter()
            .find(|entry| entry.roles.contains(&PluginRole::SyntaxHighlight))?;
        let policy = Policy {
            timeout: Duration::from_millis(entry.timeout_ms),
            max_frame_bytes: entry.max_frame_bytes,
            ..Policy::default()
        };
        let (job_sender, job_receiver) = mpsc::channel::<HighlightJob>();
        let (outcome_sender, outcome_receiver) = mpsc::channel::<HighlightOutcome>();
        let command = entry.command.clone();
        let plugin_id = entry.id.clone();
        let worker_plugin_id = plugin_id.clone();
        thread::spawn(move || {
            highlight_worker(
                &command,
                &worker_plugin_id,
                policy,
                &job_receiver,
                &outcome_sender,
            );
        });

        Some(Self {
            plugin_id,
            jobs: job_sender,
            outcomes: outcome_receiver,
            last_request: None,
        })
    }

    pub(crate) fn plugin_id(&self) -> &str {
        &self.plugin_id
    }

    /// Sends a job unless an identical snapshot (buffer, revision, window)
    /// was already requested. Returns whether a job was sent.
    pub(crate) fn schedule(&mut self, job: HighlightJob) -> bool {
        let key = (job.buffer_id, job.revision, job.first_line, job.lines.len());
        if self.last_request == Some(key) {
            return false;
        }
        self.last_request = Some(key);
        // A send error means the worker died; the next poll simply yields
        // nothing and the error was already reported as an outcome.
        self.jobs.send(job).is_ok()
    }

    pub(crate) fn poll(&mut self) -> Vec<HighlightOutcome> {
        self.outcomes.try_iter().collect()
    }
}

fn highlight_worker(
    command: &Path,
    plugin_id: &str,
    policy: Policy,
    jobs: &mpsc::Receiver<HighlightJob>,
    outcomes: &mpsc::Sender<HighlightOutcome>,
) {
    let mut client: Option<HostClient> = None;
    let mut last_failure: Option<Instant> = None;

    while let Ok(mut job) = jobs.recv() {
        // Coalesce to the newest pending job; intermediate snapshots are
        // stale by construction.
        while let Ok(newer) = jobs.try_recv() {
            job = newer;
        }

        if client.is_none() {
            if last_failure.is_some_and(|failed| failed.elapsed() < RELAUNCH_COOLDOWN) {
                continue;
            }
            match HostClient::launch(command, plugin_id, policy.clone()) {
                Ok(launched) => client = Some(launched),
                Err(error) => {
                    last_failure = Some(Instant::now());
                    let _ = outcomes.send(failure_outcome(&job, &error.to_string()));
                    continue;
                }
            }
        }
        let Some(active) = client.as_mut() else {
            continue;
        };

        let Ok(first_line) = u32::try_from(job.first_line) else {
            let _ = outcomes.send(failure_outcome(&job, "snapshot start exceeds u32 lines"));
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
                let _ = outcomes.send(HighlightOutcome {
                    buffer_id: job.buffer_id,
                    revision: job.revision,
                    result: Ok(spans),
                });
            }
            Err(error) => {
                // The client kills the host on protocol violations; drop it
                // so the next job relaunches after the cooldown.
                client = None;
                last_failure = Some(Instant::now());
                let _ = outcomes.send(failure_outcome(&job, &error.to_string()));
            }
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
impl PluginHighlighter {
    /// Test constructor without a worker thread; returns the job receiver
    /// so tests can observe what `schedule` actually sends.
    pub(crate) fn for_tests() -> (Self, mpsc::Receiver<HighlightJob>) {
        let (job_sender, job_receiver) = mpsc::channel::<HighlightJob>();
        let (_outcome_sender, outcome_receiver) = mpsc::channel::<HighlightOutcome>();
        (
            Self {
                plugin_id: "test-plugin".to_string(),
                jobs: job_sender,
                outcomes: outcome_receiver,
                last_request: None,
            },
            job_receiver,
        )
    }
}
