//! Child-process plugin host client: launch, handshake, one-role requests,
//! timeout/cancel, crash handling, and shutdown.
//!
//! Hosts are launched directly (no shell) with a cleared environment and
//! only stdin/stdout/stderr passed through. stdout carries framed protocol
//! messages; stderr is captured as bounded human-readable diagnostics.

use std::fmt;
use std::io::Read;
#[cfg(unix)]
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

#[cfg(unix)]
use rustix::process::{Pid, Signal, getpgrp, kill_process_group};

use crate::capability::{Capability, GrantedCapabilities};
use crate::frame::{FrameError, read_frame, write_frame};
use crate::json::{self, Json};
use crate::keybinding::PluginKeybinding;
use crate::menu::PluginMenu;
use crate::proto::{Envelope, MessageKind, Policy, ProtocolError, Role, TrustClass};
use crate::validate::{
    InputSnapshot, StreamChunk, StyleSpan, validate_spans, validate_stream_verdict,
    validate_surface,
};

const SHUTDOWN_GRACE: Duration = Duration::from_millis(500);
const MAX_HOST_ERROR_CHARS: usize = 200;

#[derive(Debug)]
pub enum PluginError {
    Spawn(std::io::Error),
    Io(std::io::Error),
    Frame(FrameError),
    Protocol(ProtocolError),
    Handshake(&'static str),
    Timeout,
    HostClosed,
    HostError(String),
    StaleRevision {
        expected: u64,
        received: Option<u64>,
    },
    PolicyViolation(&'static str),
}

impl fmt::Display for PluginError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Spawn(error) => write!(formatter, "failed to launch plugin host: {error}"),
            Self::Io(error) => write!(formatter, "plugin host I/O failed: {error}"),
            Self::Frame(error) => write!(formatter, "plugin frame error: {error}"),
            Self::Protocol(error) => write!(formatter, "plugin protocol error: {error}"),
            Self::Handshake(message) => write!(formatter, "plugin handshake failed: {message}"),
            Self::Timeout => write!(formatter, "plugin host timed out"),
            Self::HostClosed => write!(formatter, "plugin host closed its protocol stream"),
            Self::HostError(message) => {
                write!(formatter, "plugin host reported an error: {message}")
            }
            Self::StaleRevision { expected, received } => match received {
                Some(received) => write!(
                    formatter,
                    "stale plugin response: expected revision {expected}, received {received}"
                ),
                None => write!(
                    formatter,
                    "stale plugin response: expected revision {expected}, received none"
                ),
            },
            Self::PolicyViolation(message) => {
                write!(formatter, "plugin output rejected: {message}")
            }
        }
    }
}

pub struct HostClient {
    plugin_id: String,
    child: Child,
    process_group: Arc<AtomicU32>,
    stdin: ChildStdin,
    frames: Receiver<Result<Vec<u8>, FrameError>>,
    stderr_tail: Arc<Mutex<Vec<u8>>>,
    policy: Policy,
    next_request_id: u64,
    host_id: String,
    trust: TrustClass,
    granted: GrantedCapabilities,
    menu: Option<PluginMenu>,
    keybinding: Option<PluginKeybinding>,
}

/// The raw UI contributions a host advertises in its `HelloAck`, parsed and
/// grant-gated after the handshake completes.
struct AdvertisedUi {
    menu: Option<Json>,
    keybinding: Option<Json>,
}

impl HostClient {
    /// Launch a configured host. `roles` are the host's declared roles and
    /// `config_trust` is the trust class the user granted it in config; the
    /// two together decide the capability grant, computed after a successful
    /// handshake. The host's self-declared trust may not exceed `config_trust`.
    pub fn launch(
        command_path: &Path,
        plugin_id: &str,
        policy: Policy,
        roles: &[Role],
        config_trust: TrustClass,
    ) -> Result<Self, PluginError> {
        Self::launch_with_process_group(
            command_path,
            plugin_id,
            policy,
            roles,
            config_trust,
            Arc::new(AtomicU32::new(0)),
        )
    }

    /// Launch a configured host and publish its process-group id as soon as
    /// the child exists, before the protocol handshake can block. The owner
    /// may use the shared cell to sweep a worker that cannot reach `Drop`.
    pub fn launch_with_process_group(
        command_path: &Path,
        plugin_id: &str,
        policy: Policy,
        roles: &[Role],
        config_trust: TrustClass,
        process_group: Arc<AtomicU32>,
    ) -> Result<Self, PluginError> {
        let mut command = Command::new(command_path);
        command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .env_clear();
        // Unix hosts own a process group so cleanup can include helpers. On
        // non-Unix, `kill` retains the previous direct-child fallback.
        #[cfg(unix)]
        command.process_group(0);
        let mut child = command.spawn().map_err(PluginError::Spawn)?;
        #[cfg(unix)]
        process_group.store(child.id(), Ordering::Release);

        let stdin = child.stdin.take().expect("child stdin is piped");
        let mut stdout = child.stdout.take().expect("child stdout is piped");
        let mut stderr = child.stderr.take().expect("child stderr is piped");

        let (frame_sender, frames) = mpsc::channel();
        let max_frame_bytes = policy.max_frame_bytes;
        thread::spawn(move || {
            loop {
                let result = read_frame(&mut stdout, max_frame_bytes);
                let stop = result.is_err();
                if frame_sender.send(result).is_err() || stop {
                    return;
                }
            }
        });

        let stderr_tail = Arc::new(Mutex::new(Vec::new()));
        let stderr_capture = Arc::clone(&stderr_tail);
        let max_stderr_bytes = policy.max_stderr_bytes;
        thread::spawn(move || {
            let mut chunk = [0u8; 1024];
            loop {
                match stderr.read(&mut chunk) {
                    Ok(0) | Err(_) => return,
                    Ok(count) => {
                        if let Ok(mut tail) = stderr_capture.lock() {
                            let space = max_stderr_bytes.saturating_sub(tail.len());
                            tail.extend_from_slice(&chunk[..count.min(space)]);
                        }
                    }
                }
            }
        });

        let mut client = Self {
            plugin_id: plugin_id.to_string(),
            child,
            process_group,
            stdin,
            frames,
            stderr_tail,
            policy,
            next_request_id: 1,
            host_id: String::new(),
            trust: TrustClass::UserTrustedExternal,
            granted: GrantedCapabilities::default(),
            menu: None,
            keybinding: None,
        };

        let advertised = match client.handshake() {
            Ok(advertised) => advertised,
            Err(error) => {
                client.kill();
                return Err(error);
            }
        };
        // A host may under-claim its trust, never over-claim it: the grant is
        // computed from the user's configured trust, and a host asserting more
        // authority than that is rejected rather than silently clamped.
        if client.trust.authority_rank() > config_trust.authority_rank() {
            client.kill();
            return Err(PluginError::Handshake(
                "host trust class exceeds configured trust",
            ));
        }
        client.granted = GrantedCapabilities::for_roles(roles, config_trust);
        // A UI contribution is honored only from a host granted the matching
        // capability; an ungranted host that advertises one is simply ignored.
        // A malformed contribution from a granted host is a protocol violation.
        if client.granted.holds(Capability::Menu) {
            if let Some(payload) = advertised.menu {
                match PluginMenu::from_payload(&payload) {
                    Ok(menu) => client.menu = Some(menu),
                    Err(_) => {
                        client.kill();
                        return Err(PluginError::Handshake("invalid menu contribution"));
                    }
                }
            }
        }
        if client.granted.holds(Capability::Keybinding) {
            if let Some(payload) = advertised.keybinding {
                match PluginKeybinding::from_payload(&payload) {
                    Ok(keybinding) => client.keybinding = Some(keybinding),
                    Err(_) => {
                        client.kill();
                        return Err(PluginError::Handshake("invalid keybinding contribution"));
                    }
                }
            }
        }
        Ok(client)
    }

    fn handshake(&mut self) -> Result<AdvertisedUi, PluginError> {
        self.send(&Envelope {
            kind: MessageKind::Hello,
            request_id: 0,
            plugin_id: String::new(),
            role: None,
            revision: None,
            payload: json::obj([("host", json::str("dun"))]),
        })?;
        let timeout = self.policy.timeout;
        let ack = self.recv(timeout)?;
        if ack.kind != MessageKind::HelloAck {
            return Err(PluginError::Handshake("expected hello-ack"));
        }
        let host_id = ack
            .payload
            .get("host_id")
            .and_then(Json::as_str)
            .ok_or(PluginError::Handshake("missing host id"))?;
        let trust_id = ack
            .payload
            .get("trust")
            .and_then(Json::as_str)
            .ok_or(PluginError::Handshake("missing trust class"))?;
        let trust = TrustClass::from_id(trust_id)
            .ok_or(PluginError::Handshake("unsupported trust class"))?;
        self.host_id = host_id.to_string();
        self.trust = trust;
        Ok(AdvertisedUi {
            menu: ack.payload.get("menu").cloned(),
            keybinding: ack.payload.get("keybinding").cloned(),
        })
    }

    pub fn host_id(&self) -> &str {
        &self.host_id
    }

    pub const fn trust(&self) -> TrustClass {
        self.trust
    }

    /// The host's validated menu contribution, present only when the host was
    /// granted the `menu` capability and advertised a valid menu at handshake.
    pub fn menu(&self) -> Option<&PluginMenu> {
        self.menu.as_ref()
    }

    /// The host's validated keybinding contribution, present only when the host
    /// was granted the `keybinding` capability and advertised a valid leader +
    /// chords at handshake.
    pub fn keybinding(&self) -> Option<&PluginKeybinding> {
        self.keybinding.as_ref()
    }

    pub fn request_highlight(
        &mut self,
        snapshot: &InputSnapshot,
    ) -> Result<Vec<StyleSpan>, PluginError> {
        // Overlay spans are the only channel this request applies, so a host
        // that was not granted `overlay-write` is never asked for them.
        if !self.granted.holds(Capability::OverlayWrite) {
            return Err(PluginError::PolicyViolation(
                "overlay-write capability not granted",
            ));
        }
        if snapshot.lines.len() > self.policy.max_snapshot_lines {
            return Err(PluginError::PolicyViolation("snapshot exceeds line budget"));
        }
        let request_id = self.next_request_id;
        self.next_request_id += 1;
        let lines = snapshot.lines.iter().map(|line| json::str(line)).collect();
        self.send(&Envelope {
            kind: MessageKind::Request,
            request_id,
            plugin_id: self.plugin_id.clone(),
            role: Some(Role::SyntaxHighlight),
            revision: Some(snapshot.buffer_revision),
            payload: Json::Obj(vec![
                ("language".to_string(), json::str(&snapshot.language)),
                (
                    "first_line".to_string(),
                    json::num(u64::from(snapshot.first_line)),
                ),
                ("lines".to_string(), Json::Arr(lines)),
            ]),
        })?;

        let mut diagnostics_seen = 0usize;
        let deadline = Instant::now() + self.policy.timeout;
        loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            let envelope = match self.recv(remaining) {
                Ok(envelope) => envelope,
                Err(PluginError::Timeout) => {
                    let _ = self.send(&Envelope {
                        kind: MessageKind::CancelRequest,
                        request_id,
                        plugin_id: self.plugin_id.clone(),
                        role: Some(Role::SyntaxHighlight),
                        revision: None,
                        payload: Json::Null,
                    });
                    self.kill();
                    return Err(PluginError::Timeout);
                }
                Err(error) => {
                    self.kill();
                    return Err(error);
                }
            };
            match envelope.kind {
                MessageKind::Diagnostic => {
                    diagnostics_seen += 1;
                    if diagnostics_seen > self.policy.max_diagnostics {
                        self.kill();
                        return Err(PluginError::PolicyViolation("diagnostic flood"));
                    }
                }
                MessageKind::Response => {
                    if envelope.request_id != request_id {
                        self.kill();
                        return Err(PluginError::PolicyViolation("response for unknown request"));
                    }
                    if envelope.role != Some(Role::SyntaxHighlight) {
                        self.kill();
                        return Err(PluginError::PolicyViolation("response role mismatch"));
                    }
                    if envelope.revision != Some(snapshot.buffer_revision) {
                        return Err(PluginError::StaleRevision {
                            expected: snapshot.buffer_revision,
                            received: envelope.revision,
                        });
                    }
                    return validate_spans(snapshot, &envelope.payload, &self.policy)
                        .map_err(PluginError::PolicyViolation);
                }
                MessageKind::Error => {
                    let message = envelope
                        .payload
                        .get("message")
                        .and_then(Json::as_str)
                        .unwrap_or("unspecified");
                    return Err(PluginError::HostError(bounded_message(message)));
                }
                _ => {
                    self.kill();
                    return Err(PluginError::PolicyViolation("unexpected message kind"));
                }
            }
        }
    }

    /// Invoke a plugin action and collect the surface content the host returns
    /// for its own window (`surface-write`). Mirrors `request_highlight`'s
    /// transport (bounded diagnostics, timeout+cancel, kill on violation) but
    /// carries no role or revision: the action id is the whole request, and the
    /// response is a bounded list of validated text lines.
    pub fn request_surface(&mut self, action_id: &str) -> Result<Vec<String>, PluginError> {
        if !self.granted.holds(Capability::SurfaceWrite) {
            return Err(PluginError::PolicyViolation(
                "surface-write capability not granted",
            ));
        }
        let request_id = self.next_request_id;
        self.next_request_id += 1;
        self.send(&Envelope {
            kind: MessageKind::Request,
            request_id,
            plugin_id: self.plugin_id.clone(),
            role: None,
            revision: None,
            payload: json::obj([("action_id", json::str(action_id))]),
        })?;

        let mut diagnostics_seen = 0usize;
        let deadline = Instant::now() + self.policy.timeout;
        loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            let envelope = match self.recv(remaining) {
                Ok(envelope) => envelope,
                Err(PluginError::Timeout) => {
                    let _ = self.send(&Envelope {
                        kind: MessageKind::CancelRequest,
                        request_id,
                        plugin_id: self.plugin_id.clone(),
                        role: None,
                        revision: None,
                        payload: Json::Null,
                    });
                    self.kill();
                    return Err(PluginError::Timeout);
                }
                Err(error) => {
                    self.kill();
                    return Err(error);
                }
            };
            match envelope.kind {
                MessageKind::Diagnostic => {
                    diagnostics_seen += 1;
                    if diagnostics_seen > self.policy.max_diagnostics {
                        self.kill();
                        return Err(PluginError::PolicyViolation("diagnostic flood"));
                    }
                }
                MessageKind::Response => {
                    if envelope.request_id != request_id {
                        self.kill();
                        return Err(PluginError::PolicyViolation("response for unknown request"));
                    }
                    return validate_surface(&envelope.payload, &self.policy)
                        .map_err(PluginError::PolicyViolation);
                }
                MessageKind::Error => {
                    let message = envelope
                        .payload
                        .get("message")
                        .and_then(Json::as_str)
                        .unwrap_or("unspecified");
                    return Err(PluginError::HostError(bounded_message(message)));
                }
                _ => {
                    self.kill();
                    return Err(PluginError::PolicyViolation("unexpected message kind"));
                }
            }
        }
    }

    /// Feed a bounded output-stream chunk to a host granted `stream-read` and
    /// collect its per-line keep/drop verdict. Mirrors `request_surface`'s
    /// transport; the response must carry exactly one boolean per input line
    /// (enforced by `validate_stream_verdict`).
    pub fn request_stream_filter(&mut self, chunk: &StreamChunk) -> Result<Vec<bool>, PluginError> {
        if !self.granted.holds(Capability::StreamRead) {
            return Err(PluginError::PolicyViolation(
                "stream-read capability not granted",
            ));
        }
        if chunk.lines.len() > self.policy.max_snapshot_lines {
            return Err(PluginError::PolicyViolation(
                "stream chunk exceeds line budget",
            ));
        }
        let line_count = chunk.lines.len();
        let request_id = self.next_request_id;
        self.next_request_id += 1;
        let lines = chunk.lines.iter().map(|line| json::str(line)).collect();
        self.send(&Envelope {
            kind: MessageKind::Request,
            request_id,
            plugin_id: self.plugin_id.clone(),
            role: None,
            revision: None,
            payload: Json::Obj(vec![
                ("stream_id".to_string(), json::str(&chunk.stream_id)),
                ("chunk_index".to_string(), json::num(chunk.chunk_index)),
                ("lines".to_string(), Json::Arr(lines)),
                ("final".to_string(), json::bool(chunk.final_chunk)),
            ]),
        })?;

        let mut diagnostics_seen = 0usize;
        let deadline = Instant::now() + self.policy.timeout;
        loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            let envelope = match self.recv(remaining) {
                Ok(envelope) => envelope,
                Err(PluginError::Timeout) => {
                    let _ = self.send(&Envelope {
                        kind: MessageKind::CancelRequest,
                        request_id,
                        plugin_id: self.plugin_id.clone(),
                        role: None,
                        revision: None,
                        payload: Json::Null,
                    });
                    self.kill();
                    return Err(PluginError::Timeout);
                }
                Err(error) => {
                    self.kill();
                    return Err(error);
                }
            };
            match envelope.kind {
                MessageKind::Diagnostic => {
                    diagnostics_seen += 1;
                    if diagnostics_seen > self.policy.max_diagnostics {
                        self.kill();
                        return Err(PluginError::PolicyViolation("diagnostic flood"));
                    }
                }
                MessageKind::Response => {
                    if envelope.request_id != request_id {
                        self.kill();
                        return Err(PluginError::PolicyViolation("response for unknown request"));
                    }
                    return validate_stream_verdict(&envelope.payload, line_count)
                        .map_err(PluginError::PolicyViolation);
                }
                MessageKind::Error => {
                    let message = envelope
                        .payload
                        .get("message")
                        .and_then(Json::as_str)
                        .unwrap_or("unspecified");
                    return Err(PluginError::HostError(bounded_message(message)));
                }
                _ => {
                    self.kill();
                    return Err(PluginError::PolicyViolation("unexpected message kind"));
                }
            }
        }
    }

    /// Submit the whole scratch buffer text to a host granted `scratch-input`
    /// as one blob (`execute`) and collect the host's result lines. The snippet
    /// runs in the host's interpreter, never in `dun`; `dun` only sends the
    /// text and validates the bounded lines that come back (via
    /// `validate_surface`).
    pub fn request_execute(&mut self, snippet: &str) -> Result<Vec<String>, PluginError> {
        if !self.granted.holds(Capability::ScratchInput) {
            return Err(PluginError::PolicyViolation(
                "scratch-input capability not granted",
            ));
        }
        let request_id = self.next_request_id;
        self.next_request_id += 1;
        self.send(&Envelope {
            kind: MessageKind::Request,
            request_id,
            plugin_id: self.plugin_id.clone(),
            role: None,
            revision: None,
            payload: json::obj([("snippet", json::str(snippet))]),
        })?;

        let mut diagnostics_seen = 0usize;
        let deadline = Instant::now() + self.policy.timeout;
        loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            let envelope = match self.recv(remaining) {
                Ok(envelope) => envelope,
                Err(PluginError::Timeout) => {
                    let _ = self.send(&Envelope {
                        kind: MessageKind::CancelRequest,
                        request_id,
                        plugin_id: self.plugin_id.clone(),
                        role: None,
                        revision: None,
                        payload: Json::Null,
                    });
                    self.kill();
                    return Err(PluginError::Timeout);
                }
                Err(error) => {
                    self.kill();
                    return Err(error);
                }
            };
            match envelope.kind {
                MessageKind::Diagnostic => {
                    diagnostics_seen += 1;
                    if diagnostics_seen > self.policy.max_diagnostics {
                        self.kill();
                        return Err(PluginError::PolicyViolation("diagnostic flood"));
                    }
                }
                MessageKind::Response => {
                    if envelope.request_id != request_id {
                        self.kill();
                        return Err(PluginError::PolicyViolation("response for unknown request"));
                    }
                    return validate_surface(&envelope.payload, &self.policy)
                        .map_err(PluginError::PolicyViolation);
                }
                MessageKind::Error => {
                    let message = envelope
                        .payload
                        .get("message")
                        .and_then(Json::as_str)
                        .unwrap_or("unspecified");
                    return Err(PluginError::HostError(bounded_message(message)));
                }
                _ => {
                    self.kill();
                    return Err(PluginError::PolicyViolation("unexpected message kind"));
                }
            }
        }
    }

    pub fn shutdown(mut self) -> Result<(), PluginError> {
        let _ = self.send(&Envelope {
            kind: MessageKind::Shutdown,
            request_id: 0,
            plugin_id: String::new(),
            role: None,
            revision: None,
            payload: Json::Null,
        });
        let deadline = Instant::now() + SHUTDOWN_GRACE;
        loop {
            match self.child.try_wait() {
                Ok(Some(status)) => {
                    self.sweep_process_group();
                    if status.success() {
                        return Ok(());
                    }
                    let mut message = format!("host exited with {status}");
                    let excerpt = self.stderr_excerpt();
                    if !excerpt.is_empty() {
                        message.push_str(": ");
                        message.push_str(&excerpt);
                    }
                    return Err(PluginError::HostError(bounded_message(&message)));
                }
                Ok(None) => {
                    if Instant::now() >= deadline {
                        self.kill();
                        return Err(PluginError::Timeout);
                    }
                    thread::sleep(Duration::from_millis(10));
                }
                Err(error) => return Err(PluginError::Io(error)),
            }
        }
    }

    fn send(&mut self, envelope: &Envelope) -> Result<(), PluginError> {
        write_frame(&mut self.stdin, &envelope.to_json_bytes()).map_err(PluginError::Io)
    }

    fn recv(&mut self, timeout: Duration) -> Result<Envelope, PluginError> {
        match self.frames.recv_timeout(timeout) {
            Ok(Ok(payload)) => Envelope::from_json_bytes(&payload).map_err(PluginError::Protocol),
            Ok(Err(FrameError::Eof)) => Err(PluginError::HostClosed),
            Ok(Err(error)) => Err(PluginError::Frame(error)),
            Err(RecvTimeoutError::Timeout) => Err(PluginError::Timeout),
            Err(RecvTimeoutError::Disconnected) => Err(PluginError::HostClosed),
        }
    }

    fn stderr_excerpt(&self) -> String {
        match self.stderr_tail.lock() {
            Ok(tail) => String::from_utf8_lossy(tail.as_slice()).into_owned(),
            Err(_) => String::new(),
        }
    }

    fn kill(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        self.sweep_process_group();
    }

    #[cfg(unix)]
    fn sweep_process_group(&mut self) {
        let child_pid = self.process_group.swap(0, Ordering::AcqRel);
        if child_pid == 0 {
            return;
        }
        let own_pgid = getpgrp().as_raw_nonzero().get() as u32;
        let Some(target) = group_kill_target(child_pid, own_pgid) else {
            return;
        };
        let _ = kill_process_group(target, Signal::Kill);
    }

    #[cfg(not(unix))]
    fn sweep_process_group(&mut self) {
        // There is no portable process-group primitive here. The direct host
        // is still killed and reaped by `kill`, matching the prior behavior.
        self.process_group.store(0, Ordering::Release);
    }
}

impl Drop for HostClient {
    fn drop(&mut self) {
        self.kill();
    }
}

fn bounded_message(message: &str) -> String {
    if message.len() <= MAX_HOST_ERROR_CHARS {
        return message.to_string();
    }
    let mut cut = MAX_HOST_ERROR_CHARS;
    while !message.is_char_boundary(cut) {
        cut -= 1;
    }
    format!("{}...", &message[..cut])
}

/// Derive a positive process-group target from a child pid while refusing ids
/// that could address dun's own group or system-special process groups.
#[cfg(unix)]
pub fn group_kill_target(child_pid: u32, own_pgid: u32) -> Option<Pid> {
    if child_pid <= 1 || child_pid == own_pgid {
        return None;
    }
    i32::try_from(child_pid).ok().and_then(Pid::from_raw)
}

#[cfg(all(test, unix))]
mod tests {
    use super::{Pid, group_kill_target};

    #[test]
    fn group_kill_target_refuses_unsafe_process_groups() {
        assert_eq!(group_kill_target(42, 7), Pid::from_raw(42));
        assert_eq!(group_kill_target(42, 42), None);
        assert_eq!(group_kill_target(0, 7), None);
        assert_eq!(group_kill_target(1, 7), None);
    }
}
