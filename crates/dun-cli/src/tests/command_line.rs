#![allow(unused_imports)]

use super::support::*;

#[test]
fn command_line_runs_window_command_ids() {
    let mut app = AppState::new();
    app.sync_view_for_area(Rect::new(0, 0, 80, 20));

    submit_command_line(&mut app, "window.split_horizontal");
    assert_eq!(app.workspace.window_count(), 2);
    assert_eq!(app.status_message, Some("Split horizontally".to_string()));

    let right = app.workspace.focused;
    submit_command_line(&mut app, "window.focus_left");
    assert_ne!(app.workspace.focused, right);
    assert_eq!(app.status_message, Some("Focused left".to_string()));

    submit_command_line(&mut app, "window.resize_down extra");
    assert_eq!(
        app.status_message,
        Some("Command failed: window.resize_down expects no arguments".to_string())
    );
}

#[test]
fn command_line_prompt_dispatches_app_commands() {
    let mut app = AppState::new();

    handle_key_event(
        &mut app,
        CrosstermKeyEvent::new(CrosstermKeyCode::Char('p'), CrosstermKeyModifiers::CONTROL),
    );
    assert_eq!(app.prompt_status_text(), Some("Command: ".to_string()));

    send_text(&mut app, "help");
    handle_key_event(
        &mut app,
        CrosstermKeyEvent::new(CrosstermKeyCode::Enter, CrosstermKeyModifiers::NONE),
    );

    assert_eq!(
        app.workspace.focused_window().unwrap().kind,
        WindowKind::Help
    );

    submit_command_line(&mut app, "config");
    assert_eq!(
        app.workspace.focused_window().unwrap().kind,
        WindowKind::ConfigDiagnostics
    );
}

#[test]
fn command_line_prompt_completes_output_commands() {
    let mut app = AppState::new();

    app.handle_command(&EditorCommand::App(AppCommand::CommandLine));
    send_text(&mut app, "outp");
    handle_key_event(
        &mut app,
        CrosstermKeyEvent::new(CrosstermKeyCode::Tab, CrosstermKeyModifiers::NONE),
    );
    assert_eq!(
        app.prompt_status_text(),
        Some("Command: output ".to_string())
    );

    send_text(&mut app, "stdout-b");
    handle_key_event(
        &mut app,
        CrosstermKeyEvent::new(CrosstermKeyCode::Tab, CrosstermKeyModifiers::NONE),
    );
    assert_eq!(
        app.prompt_status_text(),
        Some("Command: output stdout-body".to_string())
    );
}

#[test]
fn command_line_prompt_completes_config_sections_and_themes() {
    let mut app = AppState::new();

    app.handle_command(&EditorCommand::App(AppCommand::CommandLine));
    send_text(&mut app, "config file-d");
    handle_key_event(
        &mut app,
        CrosstermKeyEvent::new(CrosstermKeyCode::Tab, CrosstermKeyModifiers::NONE),
    );
    assert_eq!(
        app.prompt_status_text(),
        Some("Command: config file-dialog-keymap".to_string())
    );

    handle_key_event(
        &mut app,
        CrosstermKeyEvent::new(CrosstermKeyCode::Esc, CrosstermKeyModifiers::NONE),
    );
    app.handle_command(&EditorCommand::App(AppCommand::CommandLine));
    send_text(&mut app, "theme ms");
    handle_key_event(
        &mut app,
        CrosstermKeyEvent::new(CrosstermKeyCode::Tab, CrosstermKeyModifiers::NONE),
    );
    assert_eq!(
        app.prompt_status_text(),
        Some("Command: theme msedit".to_string())
    );
}

#[test]
fn command_line_prompt_lists_and_cycles_ambiguous_completions() {
    let mut app = AppState::new();

    app.handle_command(&EditorCommand::App(AppCommand::CommandLine));
    send_text(&mut app, "re");
    handle_key_event(
        &mut app,
        CrosstermKeyEvent::new(CrosstermKeyCode::Tab, CrosstermKeyModifiers::NONE),
    );
    assert_eq!(app.prompt_status_text(), Some("Command: re".to_string()));
    assert!(
        app.status_message
            .as_deref()
            .is_some_and(|status| status.contains("reload-config")
                && status.contains("replace")
                && status.contains("results"))
    );

    handle_key_event(
        &mut app,
        CrosstermKeyEvent::new(CrosstermKeyCode::Tab, CrosstermKeyModifiers::NONE),
    );
    assert_eq!(
        app.prompt_status_text(),
        Some("Command: reload-config ".to_string())
    );

    handle_key_event(
        &mut app,
        CrosstermKeyEvent::new(CrosstermKeyCode::Tab, CrosstermKeyModifiers::NONE),
    );
    assert_eq!(
        app.prompt_status_text(),
        Some("Command: reloadfile ".to_string())
    );

    handle_key_event(
        &mut app,
        CrosstermKeyEvent::new(CrosstermKeyCode::BackTab, CrosstermKeyModifiers::SHIFT),
    );
    assert_eq!(
        app.prompt_status_text(),
        Some("Command: reload-config ".to_string())
    );
}

#[test]
fn command_line_prompt_overlay_shows_ambiguous_completion_candidates() {
    let mut app = AppState::new();

    app.handle_command(&EditorCommand::App(AppCommand::CommandLine));
    send_text(&mut app, "re");
    handle_key_event(
        &mut app,
        CrosstermKeyEvent::new(CrosstermKeyCode::Tab, CrosstermKeyModifiers::NONE),
    );

    let overlay = app.active_overlay().expect("command prompt overlay");
    assert_eq!(overlay.title, "Command");
    assert!(overlay.lines.iter().any(|line| {
        line.contains("Command completion")
            && line.contains("reload-config")
            && line.contains("replace")
            && line.contains("results")
    }));
}

#[test]
fn command_line_prompt_completes_path_arguments() {
    let directory = temp_file_path("command-line-complete");
    let nested = directory.join("nested");
    let file = nested.join("alpha file.txt");
    std::fs::create_dir(&directory).unwrap();
    std::fs::create_dir(&nested).unwrap();
    std::fs::write(&file, "alpha").unwrap();
    let mut app = AppState::new();

    app.handle_command(&EditorCommand::App(AppCommand::CommandLine));
    send_text(&mut app, &format!("open {}/n", directory.display()));
    handle_key_event(
        &mut app,
        CrosstermKeyEvent::new(CrosstermKeyCode::Tab, CrosstermKeyModifiers::NONE),
    );
    assert_eq!(
        app.prompt_status_text(),
        Some(format!("Command: open {}/nested/", directory.display()))
    );
    send_text(&mut app, "a");
    handle_key_event(
        &mut app,
        CrosstermKeyEvent::new(CrosstermKeyCode::Tab, CrosstermKeyModifiers::NONE),
    );
    assert_eq!(
        app.prompt_status_text(),
        Some(format!(
            "Command: open \"{}/nested/alpha file.txt\"",
            directory.display()
        ))
    );

    let _ = std::fs::remove_file(file);
    let _ = std::fs::remove_dir(nested);
    let _ = std::fs::remove_dir(directory);
}

#[test]
fn command_line_run_executes_quoted_command() {
    let mut app = AppState::new();

    submit_command_line(&mut app, "run \"printf quoted-run\"");

    let window = app.workspace.focused_window().unwrap();
    assert_eq!(window.kind, WindowKind::CommandOutput);
    assert!(
        app.buffer_state(window.buffer_id)
            .unwrap()
            .buffer
            .to_text()
            .contains("quoted-run")
    );
}

#[test]
fn command_line_history_navigates_recent_commands_and_restores_draft() {
    let mut app = AppState::new();

    submit_command_line(&mut app, "commands");
    submit_command_line(&mut app, "theme");

    app.handle_command(&EditorCommand::App(AppCommand::CommandLine));
    send_text(&mut app, "draft");

    handle_key_event(
        &mut app,
        CrosstermKeyEvent::new(CrosstermKeyCode::Up, CrosstermKeyModifiers::NONE),
    );
    assert_eq!(app.prompt_status_text(), Some("Command: theme".to_string()));

    handle_key_event(
        &mut app,
        CrosstermKeyEvent::new(CrosstermKeyCode::Up, CrosstermKeyModifiers::NONE),
    );
    assert_eq!(
        app.prompt_status_text(),
        Some("Command: commands".to_string())
    );

    handle_key_event(
        &mut app,
        CrosstermKeyEvent::new(CrosstermKeyCode::Up, CrosstermKeyModifiers::NONE),
    );
    assert_eq!(
        app.prompt_status_text(),
        Some("Command: commands".to_string())
    );

    handle_key_event(
        &mut app,
        CrosstermKeyEvent::new(CrosstermKeyCode::Down, CrosstermKeyModifiers::NONE),
    );
    assert_eq!(app.prompt_status_text(), Some("Command: theme".to_string()));

    handle_key_event(
        &mut app,
        CrosstermKeyEvent::new(CrosstermKeyCode::Down, CrosstermKeyModifiers::NONE),
    );
    assert_eq!(app.prompt_status_text(), Some("Command: draft".to_string()));
}

#[test]
fn command_line_history_repeats_previous_command() {
    let mut app = AppState::new();

    submit_command_line(&mut app, "commands");
    app.set_status("cleared");

    app.handle_command(&EditorCommand::App(AppCommand::CommandLine));
    handle_key_event(
        &mut app,
        CrosstermKeyEvent::new(CrosstermKeyCode::Up, CrosstermKeyModifiers::NONE),
    );
    handle_key_event(
        &mut app,
        CrosstermKeyEvent::new(CrosstermKeyCode::Enter, CrosstermKeyModifiers::NONE),
    );

    assert_eq!(app.status_message, Some(COMMAND_LINE_HELP.to_string()));
    assert_eq!(app.command_history, vec!["commands".to_string()]);
}

#[test]
fn command_line_history_is_capped_and_skips_consecutive_duplicates() {
    let mut app = AppState::new();

    for index in 0..(COMMAND_HISTORY_LIMIT + 2) {
        app.record_command_history(format!("cmd-{index}"));
    }

    assert_eq!(app.command_history.len(), COMMAND_HISTORY_LIMIT);
    assert_eq!(app.command_history.first(), Some(&"cmd-2".to_string()));
    assert_eq!(
        app.command_history.last(),
        Some(&format!("cmd-{}", COMMAND_HISTORY_LIMIT + 1))
    );

    let last = app.command_history.last().cloned().unwrap();
    app.record_command_history(last);
    assert_eq!(app.command_history.len(), COMMAND_HISTORY_LIMIT);
}

#[test]
fn command_line_history_does_not_affect_other_prompts() {
    let mut app = AppState::new();
    app.record_command_history("theme".to_string());

    app.handle_command(&EditorCommand::File(FileCommand::Open));
    send_text(&mut app, "path");
    handle_key_event(
        &mut app,
        CrosstermKeyEvent::new(CrosstermKeyCode::Up, CrosstermKeyModifiers::NONE),
    );
    handle_key_event(
        &mut app,
        CrosstermKeyEvent::new(CrosstermKeyCode::Down, CrosstermKeyModifiers::NONE),
    );

    assert_eq!(app.prompt_status_text(), Some("Open: path".to_string()));
}

#[test]
fn command_line_prompt_cursor_edits_middle_of_input() {
    let mut app = AppState::new();

    app.handle_command(&EditorCommand::App(AppCommand::CommandLine));
    send_text(&mut app, "ac");
    handle_key_event(
        &mut app,
        CrosstermKeyEvent::new(CrosstermKeyCode::Left, CrosstermKeyModifiers::NONE),
    );
    send_text(&mut app, "b");
    assert_eq!(app.prompt_status_text(), Some("Command: abc".to_string()));

    handle_key_event(
        &mut app,
        CrosstermKeyEvent::new(CrosstermKeyCode::Home, CrosstermKeyModifiers::NONE),
    );
    send_text(&mut app, ">");
    assert_eq!(app.prompt_status_text(), Some("Command: >abc".to_string()));

    handle_key_event(
        &mut app,
        CrosstermKeyEvent::new(CrosstermKeyCode::End, CrosstermKeyModifiers::NONE),
    );
    send_text(&mut app, "<");
    handle_key_event(
        &mut app,
        CrosstermKeyEvent::new(CrosstermKeyCode::Left, CrosstermKeyModifiers::NONE),
    );
    handle_key_event(
        &mut app,
        CrosstermKeyEvent::new(CrosstermKeyCode::Backspace, CrosstermKeyModifiers::NONE),
    );

    assert_eq!(app.prompt_status_text(), Some("Command: >ab<".to_string()));
}

#[test]
fn command_line_prompt_cursor_respects_utf8_boundaries() {
    let mut app = AppState::new();

    app.handle_command(&EditorCommand::App(AppCommand::CommandLine));
    send_text(&mut app, "中b");
    handle_key_event(
        &mut app,
        CrosstermKeyEvent::new(CrosstermKeyCode::Left, CrosstermKeyModifiers::NONE),
    );
    handle_key_event(
        &mut app,
        CrosstermKeyEvent::new(CrosstermKeyCode::Backspace, CrosstermKeyModifiers::NONE),
    );
    assert_eq!(app.prompt_status_text(), Some("Command: b".to_string()));

    handle_key_event(
        &mut app,
        CrosstermKeyEvent::new(CrosstermKeyCode::Delete, CrosstermKeyModifiers::NONE),
    );
    assert_eq!(app.prompt_status_text(), Some("Command: ".to_string()));
}

#[test]
fn command_line_runs_file_commands_with_quoted_paths() {
    let save_path = temp_file_path("command save.txt");
    let open_path = temp_file_path("command open.txt");
    std::fs::write(&open_path, "opened").unwrap();
    let mut app = AppState::new();

    app.handle_text_input('x');
    submit_command_line(&mut app, &format!("save-as \"{}\"", save_path.display()));

    assert_eq!(std::fs::read_to_string(&save_path).unwrap(), "x");
    assert_eq!(
        app.status_message,
        Some(format!("Saved {}", save_path.display()))
    );

    submit_command_line(&mut app, "new");
    submit_command_line(&mut app, &format!("open \"{}\"", open_path.display()));

    let state = app.buffer_state(BufferId(1)).unwrap();
    assert_eq!(state.buffer.to_text(), "opened");
    assert_eq!(state.path.as_ref(), Some(&open_path));

    let _ = std::fs::remove_file(save_path);
    let _ = std::fs::remove_file(open_path);
}

#[test]
fn command_line_open_path_refuses_dirty_focused_buffer() {
    let path = temp_file_path("command-open-dirty.txt");
    std::fs::write(&path, "opened").unwrap();
    let mut app = AppState::new();
    app.handle_text_input('x');

    submit_command_line(&mut app, &format!("open {}", path.display()));

    assert_eq!(
        app.status_message,
        Some("Open failed: focused buffer has unsaved changes".to_string())
    );
    assert_eq!(app.buffer_state(BufferId(1)).unwrap().buffer.to_text(), "x");

    let _ = std::fs::remove_file(path);
}

#[test]
fn command_line_reports_unknown_and_parse_errors() {
    let mut app = AppState::new();

    submit_command_line(&mut app, "wat");
    assert_eq!(app.status_message, Some("Unknown command: wat".to_string()));

    submit_command_line(&mut app, "open \"unterminated");
    assert_eq!(
        app.status_message,
        Some("Command failed: unclosed quote".to_string())
    );
}

#[test]
fn command_line_parser_handles_quotes_and_escapes() {
    assert_eq!(
        parse_command_line("open \"a b\" save\\ path").unwrap(),
        vec![
            "open".to_string(),
            "a b".to_string(),
            "save path".to_string()
        ]
    );
    assert_eq!(
        parse_command_line("replace one \"\"").unwrap(),
        vec!["replace".to_string(), "one".to_string(), String::new()]
    );
    assert_eq!(
        parse_command_line("open \"unterminated"),
        Err(CommandLineParseError::UnclosedQuote)
    );
    assert_eq!(
        parse_command_line("open path\\"),
        Err(CommandLineParseError::TrailingEscape)
    );
}

#[test]
fn command_line_replace_all_is_single_undo_step() {
    let mut app = app_with_text("one two one");

    app.run_command_line("replace all one uno");

    let state = app.buffer_state(BufferId(1)).unwrap();
    assert_eq!(state.buffer.to_text(), "uno two uno");
    assert_eq!(
        app.status_message,
        Some("Replace All: 2 one -> uno".to_string())
    );

    app.handle_command(&EditorCommand::Edit(EditCommand::Undo));
    assert_eq!(
        app.buffer_state(BufferId(1)).unwrap().buffer.to_text(),
        "one two one"
    );
}

#[test]
fn command_line_replace_all_honors_search_flags() {
    let mut app = app_with_text("ERROR errors error_error error");

    app.run_command_line("replace all \"/iw error\" ok");

    assert_eq!(
        app.buffer_state(BufferId(1)).unwrap().buffer.to_text(),
        "ok errors error_error ok"
    );
    assert_eq!(
        app.status_message,
        Some("Replace All: 2 error (ignore-case, whole-word) -> ok".to_string())
    );
}
