#![allow(unused_imports)]

use super::support::*;

struct CommandTempDir {
    path: PathBuf,
}

impl CommandTempDir {
    fn new(name: &str) -> Self {
        let path = temp_file_path(name);
        std::fs::create_dir_all(&path).expect("creates command test directory");
        Self { path }
    }
}

impl Drop for CommandTempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

fn shell_quote(path: &Path) -> String {
    format!("'{}'", path.to_string_lossy().replace('\'', "'\\''"))
}

#[test]
fn run_command_prompt_opens_read_only_output_window() {
    let mut app = AppState::new();

    app.handle_command(&EditorCommand::App(AppCommand::RunCommand));
    assert_eq!(app.prompt_status_text(), Some("Run Command: ".to_string()));
    send_text(&mut app, "printf dun-run");
    handle_key_event(
        &mut app,
        TerminalKeyEvent::new(TerminalKeyCode::Enter, TerminalKeyModifiers::NONE),
    );

    let window = app.workspace.focused_window().unwrap();
    assert_eq!(window.kind, WindowKind::CommandOutput);
    let buffer = app.buffer_state(window.buffer_id).unwrap();
    assert!(buffer.buffer.is_read_only());
    let text = buffer.buffer.to_text();
    assert!(text.contains("Command: printf dun-run"));
    assert!(text.contains("Stdout: 7 bytes, complete"));
    assert!(text.contains("Truncated: no"));
    assert!(text.contains("--- stdout (7 bytes, complete) ---\ndun-run\n"));
    assert!(
        app.status_message
            .as_deref()
            .is_some_and(|status| status.contains("Command returned exit 0"))
    );
}

#[test]
fn run_command_history_navigates_separately_from_command_line_history() {
    let mut app = AppState::new();

    app.handle_command(&EditorCommand::App(AppCommand::RunCommand));
    send_text(&mut app, "printf first");
    handle_key_event(
        &mut app,
        TerminalKeyEvent::new(TerminalKeyCode::Enter, TerminalKeyModifiers::NONE),
    );

    app.handle_command(&EditorCommand::App(AppCommand::RunCommand));
    send_text(&mut app, "printf second");
    handle_key_event(
        &mut app,
        TerminalKeyEvent::new(TerminalKeyCode::Enter, TerminalKeyModifiers::NONE),
    );

    app.handle_command(&EditorCommand::App(AppCommand::RunCommand));
    send_text(&mut app, "draft");

    handle_key_event(
        &mut app,
        TerminalKeyEvent::new(TerminalKeyCode::Up, TerminalKeyModifiers::NONE),
    );
    assert_eq!(
        app.prompt_status_text(),
        Some("Run Command: printf second".to_string())
    );

    handle_key_event(
        &mut app,
        TerminalKeyEvent::new(TerminalKeyCode::Up, TerminalKeyModifiers::NONE),
    );
    assert_eq!(
        app.prompt_status_text(),
        Some("Run Command: printf first".to_string())
    );

    handle_key_event(
        &mut app,
        TerminalKeyEvent::new(TerminalKeyCode::Down, TerminalKeyModifiers::NONE),
    );
    assert_eq!(
        app.prompt_status_text(),
        Some("Run Command: printf second".to_string())
    );

    handle_key_event(
        &mut app,
        TerminalKeyEvent::new(TerminalKeyCode::Down, TerminalKeyModifiers::NONE),
    );
    assert_eq!(
        app.prompt_status_text(),
        Some("Run Command: draft".to_string())
    );
    assert!(app.command_history.is_empty());
    assert_eq!(
        app.run_command_history,
        vec!["printf first".to_string(), "printf second".to_string()]
    );
}

#[test]
fn run_command_reuses_output_window_for_new_results() {
    let mut app = AppState::new();

    app.run_external_command_to_buffer("printf one");
    let first_window = app.workspace.focused_window().unwrap().clone();
    let window_count = app.workspace.windows.len();

    app.run_external_command_to_buffer("printf two");

    let second_window = app.workspace.focused_window().unwrap();
    assert_eq!(app.workspace.windows.len(), window_count);
    assert_eq!(second_window.id, first_window.id);
    assert_eq!(second_window.kind, WindowKind::CommandOutput);
    assert_eq!(second_window.buffer_kind, BufferKind::ReadOnly);
    assert!(!second_window.collapsed);
    let text = app
        .buffer_state(second_window.buffer_id)
        .unwrap()
        .buffer
        .to_text();
    assert!(text.contains("Command: printf two"));
    assert!(text.contains("two"));
    assert!(!text.contains("one"));
}

#[test]
fn run_command_kills_non_terminating_commands_after_timeout() {
    let mut app = AppState::new();
    app.limits.run_command_timeout_ms = 100;

    app.run_external_command_to_buffer("sleep 5");

    assert!(
        app.status_message
            .as_deref()
            .is_some_and(|status| status.contains("Command timed out after")),
        "status: {:?}",
        app.status_message
    );
    let window = app.workspace.focused_window().unwrap();
    assert_eq!(window.kind, WindowKind::CommandOutput);
    let text = app.buffer_state(window.buffer_id).unwrap().buffer.to_text();
    assert!(text.contains("Status: timed out; process killed"));
}

#[test]
fn command_capture_deadline_returns_from_background_descendant() {
    let mut app = AppState::new();
    app.limits.run_command_timeout_ms = 100;
    let started = Instant::now();

    app.run_external_command_to_buffer("sleep 3 & echo started");

    let elapsed = started.elapsed();
    eprintln!("background command capture returned in {elapsed:?}");
    assert!(
        elapsed < Duration::from_secs(1),
        "capture took {elapsed:?}, too close to the descendant's three-second lifetime"
    );
    let window = app.workspace.focused_window().unwrap();
    assert_eq!(window.kind, WindowKind::CommandOutput);
    let text = app.buffer_state(window.buffer_id).unwrap().buffer.to_text();
    assert!(text.contains("started"), "command output:\n{text}");
    assert!(text.contains("Truncated: yes"), "command output:\n{text}");
}

#[test]
fn run_command_kills_background_descendant_before_it_can_survive() {
    let directory = CommandTempDir::new("command-background-descendant");
    let helper = directory.path.join("helper.sh");
    let ready = directory.path.join("ready");
    let survived = directory.path.join("survived");
    std::fs::write(
        &helper,
        "ready=$1\nsurvived=$2\n: > \"$ready\"\nsleep 2\n: > \"$survived\"\n",
    )
    .expect("writes background helper");
    let command = format!(
        "/bin/sh {} {} {} & while [ ! -f {} ]; do :; done",
        shell_quote(&helper),
        shell_quote(&ready),
        shell_quote(&survived),
        shell_quote(&ready),
    );
    let mut app = AppState::new();
    app.limits.run_command_timeout_ms = 4_000;
    let started = Instant::now();

    app.run_external_command_to_buffer(&command);

    let elapsed = started.elapsed();
    assert!(
        ready.exists(),
        "background helper never reached its ready point"
    );
    let observation_period = Duration::from_millis(2_500);
    std::thread::sleep((started + observation_period).saturating_duration_since(Instant::now()));
    assert!(
        !survived.exists(),
        "background helper survived process-group cleanup"
    );
    assert!(
        elapsed < Duration::from_secs(1),
        "background capture returned too late: {elapsed:?}"
    );
    assert!(
        app.status_message
            .as_deref()
            .is_some_and(|status| status.contains("background processes killed")),
        "status: {:?}",
        app.status_message
    );
    let window = app.workspace.focused_window().unwrap();
    let text = app.buffer_state(window.buffer_id).unwrap().buffer.to_text();
    assert!(
        text.contains("Status: exit 0; background processes killed"),
        "command output:\n{text}"
    );
}

#[test]
fn backgrounding_command_returns_well_before_its_deadline() {
    let timeout = Duration::from_secs(2);
    let started = Instant::now();

    let result = run_command_capture(
        "sleep 2 & echo started",
        COMMAND_OUTPUT_STREAM_SOFT_LIMIT_BYTES,
        timeout,
    )
    .expect("backgrounding command should run");

    let elapsed = started.elapsed();
    eprintln!("backgrounding command returned in {elapsed:?}");
    assert!(
        elapsed < Duration::from_secs(1),
        "background capture used too much of its {timeout:?} timeout: {elapsed:?}"
    );
    assert!(
        result.background_processes_killed,
        "successful group cleanup should be recorded"
    );
    assert_eq!(result.stdout.bytes, b"started\n");
}

#[test]
fn timed_out_command_with_complete_output_is_not_reported_truncated() {
    let mut app = AppState::new();
    app.limits.run_command_timeout_ms = 200;

    // Output reaches EOF immediately; the command then outlives the timeout.
    // Killing its process group must not be mistaken for losing output.
    app.run_external_command_to_buffer("exec >/dev/null 2>&1; sleep 3");

    let status = app.status_message.as_deref().unwrap_or_default();
    assert!(
        status.contains("Command timed out after"),
        "status: {status:?}"
    );
    assert!(
        !status.contains("truncated"),
        "nothing was truncated, but the status claims it was: {status:?}"
    );
    let window = app.workspace.focused_window().unwrap();
    let text = app.buffer_state(window.buffer_id).unwrap().buffer.to_text();
    assert!(text.contains("Truncated: no"), "command output:\n{text}");
}
