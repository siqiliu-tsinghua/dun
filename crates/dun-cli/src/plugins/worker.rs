//! The worker-thread side of a plugin host.
//!
//! Everything here runs off the event loop: process launch, the framed-stdio
//! client, and the per-role request/response serving. Split out of
//! `plugins.rs` when that file passed the 35k architecture-debt threshold in
//! docs/code-organization-guidelines.md. Behaviour-preserving: the code moved
//! verbatim, only its `use` list is new.

use std::path::Path;
use std::sync::mpsc;
use std::time::Instant;

use dun_plugin::{HostClient, InputSnapshot, Policy, Role, StreamChunk, TrustClass};

use super::{HighlightJob, HighlightOutcome, HostEvent, RELAUNCH_COOLDOWN, WorkerMessage};

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

pub(super) fn host_worker(
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

#[cfg(test)]
pub(crate) fn next_worker_action_for_tests(
    messages: &mpsc::Receiver<WorkerMessage>,
    unloaded: &mut bool,
) -> Result<(bool, Option<HighlightJob>), mpsc::RecvError> {
    let mut client = None;
    next_worker_action(messages, &mut client, unloaded).map(|action| (action.launch, action.job))
}
