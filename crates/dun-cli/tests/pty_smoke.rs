#![cfg(unix)]
#![forbid(unsafe_code)]

use std::ffi::OsStr;
use std::fs;
use std::io;
use std::os::unix::fs::PermissionsExt;
#[cfg(feature = "test-panic-hook")]
use std::os::unix::process::ExitStatusExt;
use std::time::Duration;

mod support;

use support::pty::{
    CTRL_Q, TerminalCase, assert_output_contains, assert_output_not_contains, command_on_path,
    pty_test_guard, run_dun_in_pty, run_dun_in_pty_with_env, run_dun_osc52_in_pty, temp_path,
};
use support::terminal_grid::assert_line_contains;

fn terminal_profile_cases() -> [TerminalCase; 9] {
    [
        TerminalCase::new(
            "xterm-256color utf8",
            "xterm-256color",
            "en_US.UTF-8",
            "en_US.UTF-8",
            false,
            "UTF-8/256",
        ),
        TerminalCase::new(
            "screen-256color utf8",
            "screen-256color",
            "en_US.UTF-8",
            "en_US.UTF-8",
            false,
            "UTF-8/256",
        ),
        TerminalCase::new(
            "tmux-256color utf8",
            "tmux-256color",
            "en_US.UTF-8",
            "en_US.UTF-8",
            false,
            "UTF-8/256",
        ),
        TerminalCase::new(
            "screen utf8",
            "screen",
            "en_US.UTF-8",
            "en_US.UTF-8",
            false,
            "UTF-8/16",
        ),
        TerminalCase::new(
            "xterm-color c locale",
            "xterm-color",
            "C",
            "C",
            false,
            "ASCII/16",
        ),
        TerminalCase::new("vt100 ascii", "vt100", "C", "C", false, "ASCII/16"),
        TerminalCase::new("ansi ascii", "ansi", "C", "C", false, "ASCII/16"),
        TerminalCase::new("dumb ascii mono", "dumb", "C", "C", false, "ASCII/mono"),
        TerminalCase::new(
            "xterm-256color no color",
            "xterm-256color",
            "en_US.UTF-8",
            "en_US.UTF-8",
            true,
            "UTF-8/mono",
        ),
    ]
}

#[test]
fn pty_smoke_quits_cleanly_for_common_terminal_profiles() -> io::Result<()> {
    let _guard = pty_test_guard();
    let Some(expect) = command_on_path("expect") else {
        eprintln!("skipping PTY smoke test: expect(1) is not on PATH");
        return Ok(());
    };

    for case in terminal_profile_cases() {
        let run = run_dun_in_pty(&expect, case, &[], "Untitled", CTRL_Q)?;
        assert!(
            run.status.success(),
            "{} failed with status {:?}\n{}",
            case.name,
            run.status,
            run.output
        );
        assert_output_contains(&run.output, "Untitled", case.name);
        assert_output_contains(&run.output, "[Plain Text]", case.name);
        assert_output_contains(&run.output, "1:1", case.name);
        if let Some(profile) = case.expected_profile {
            assert_output_contains(&run.output, profile, case.name);
            if profile.ends_with("/16") {
                assert_legacy_16_color_output(&run.output, case.name);
            }
        }
    }

    Ok(())
}

#[test]
fn pty_smoke_opens_utf8_file_and_renders_initial_content() -> io::Result<()> {
    let _guard = pty_test_guard();
    let Some(expect) = command_on_path("expect") else {
        eprintln!("skipping PTY smoke test: expect(1) is not on PATH");
        return Ok(());
    };

    let file_path = temp_path("dun-pty-open", "txt");
    let file_name = file_path
        .file_name()
        .and_then(OsStr::to_str)
        .expect("temp file name should be valid UTF-8")
        .to_string();
    fs::write(&file_path, "alpha\nbeta\n")?;

    let case = TerminalCase::new(
        "xterm-256color file open",
        "xterm-256color",
        "en_US.UTF-8",
        "en_US.UTF-8",
        false,
        "UTF-8/256",
    );

    let run = run_dun_in_pty(&expect, case, &[file_path.as_os_str()], "alpha", CTRL_Q);
    let _ = fs::remove_file(&file_path);
    let run = run?;

    assert!(
        run.status.success(),
        "{} failed with status {:?}\n{}",
        case.name,
        run.status,
        run.output
    );
    assert_output_contains(&run.output, &file_name, case.name);
    assert_output_contains(&run.output, "alpha", case.name);
    assert_output_contains(&run.output, "beta", case.name);
    assert_output_contains(&run.output, "[Plain Text]", case.name);
    assert_output_contains(&run.output, "[UTF-8]", case.name);

    let grid = run.terminal_grid_for_case(case);
    assert_eq!(grid.width, case.cols);
    assert_eq!(grid.height, case.rows);
    assert_line_contains(&grid, 2, "alpha");
    assert_line_contains(&grid, 3, "beta");
    assert_line_contains(&grid, 23, "[UTF-8]");

    Ok(())
}

#[test]
fn pty_smoke_handles_small_low_capability_terminal() -> io::Result<()> {
    let _guard = pty_test_guard();
    let Some(expect) = command_on_path("expect") else {
        eprintln!("skipping PTY smoke test: expect(1) is not on PATH");
        return Ok(());
    };

    let case = TerminalCase::new("small vt100 ascii", "vt100", "C", "C", false, "ASCII/16")
        .sized(12, 40)
        .without_profile_marker();
    let run = run_dun_in_pty(&expect, case, &[], "Untitled", CTRL_Q)?;

    assert!(
        run.status.success(),
        "{} failed with status {:?}\n{}",
        case.name,
        run.status,
        run.output
    );
    assert_output_contains(&run.output, "Untitled", case.name);
    assert_legacy_16_color_output(&run.output, case.name);

    Ok(())
}

fn assert_legacy_16_color_output(output: &str, case: &str) {
    assert_output_not_contains(output, "\x1b[38;5;", case);
    assert_output_not_contains(output, "\x1b[48;5;", case);
    assert_output_not_contains(output, ";38;5;", case);
    assert_output_not_contains(output, ";48;5;", case);
}

#[test]
fn pty_smoke_quits_cleanly_with_mouse_capture_enabled() -> io::Result<()> {
    let _guard = pty_test_guard();
    let Some(expect) = command_on_path("expect") else {
        eprintln!("skipping PTY smoke test: expect(1) is not on PATH");
        return Ok(());
    };

    let config_path = temp_path("dun-pty-mouse-config", "conf");
    fs::write(&config_path, "mouse.enabled = true\n")?;
    let case = TerminalCase::new(
        "xterm-256color mouse enabled",
        "xterm-256color",
        "en_US.UTF-8",
        "en_US.UTF-8",
        false,
        "UTF-8/256",
    );
    let run = run_dun_in_pty(
        &expect,
        case,
        &[OsStr::new("--config"), config_path.as_os_str()],
        "Untitled",
        CTRL_Q,
    );
    let _ = fs::remove_file(&config_path);
    let run = run?;

    assert!(
        run.status.success(),
        "{} failed with status {:?}\n{}",
        case.name,
        run.status,
        run.output
    );
    assert_output_contains(&run.output, "Untitled", case.name);

    Ok(())
}

#[test]
fn pty_smoke_pastes_matched_osc52_response_and_sanitizes_controls() -> io::Result<()> {
    let _guard = pty_test_guard();
    let Some(expect) = command_on_path("expect") else {
        eprintln!("skipping PTY smoke test: expect(1) is not on PATH");
        return Ok(());
    };

    const PASTE_EXTERNAL: &[u8] = b"\x18\x16";
    const RESPONSE: &[u8] = b"\x1b]52;c;T1NDNTItc2FmZRtdMDtvd25lZAc=\x07";
    const RAW_CONTROL_PAYLOAD: &str = "\x1b]0;owned\x07";

    let config_path = temp_path("dun-pty-osc52-read-config", "conf");
    fs::write(
        &config_path,
        "clipboard.osc52.allow_read = true\nclipboard.osc52.max_bytes = 1024\n",
    )?;
    let case = TerminalCase::new(
        "xterm-256color OSC 52 read",
        "xterm-256color",
        "en_US.UTF-8",
        "en_US.UTF-8",
        false,
        "UTF-8/256",
    );
    let run = run_dun_osc52_in_pty(
        &expect,
        case,
        &[OsStr::new("--config"), config_path.as_os_str()],
        "Untitled",
        PASTE_EXTERNAL,
        Some(RESPONSE),
        "OSC52-safe",
    );
    let _ = fs::remove_file(&config_path);
    let run = run?;

    assert!(
        run.status.success(),
        "{} failed with status {:?}\n{}",
        case.name,
        run.status,
        run.output
    );
    assert!(
        run.osc52_query_observed,
        "{} did not observe the exact OSC 52 query\n{}",
        case.name, run.output
    );
    assert_output_contains(&run.output, "\x1b]52;c;?\x07", case.name);
    assert_output_not_contains(&run.output, RAW_CONTROL_PAYLOAD, case.name);

    let grid = run.terminal_grid_for_case(case);
    assert_line_contains(&grid, 2, "OSC52-safe␛]0;owned␇");

    Ok(())
}

#[test]
fn pty_smoke_osc52_timeout_restores_internal_clipboard_after_500ms() -> io::Result<()> {
    let _guard = pty_test_guard();
    let Some(expect) = command_on_path("expect") else {
        eprintln!("skipping PTY smoke test: expect(1) is not on PATH");
        return Ok(());
    };

    // Type the seed, select it, copy it through the Edit-menu mnemonic, delete
    // the active selection, then issue Ctrl+X,Ctrl+V.
    const SEED_CLEAR_AND_PASTE_EXTERNAL: &[u8] = b"internal-fallback\x01\x1bec\x1b[3~\x18\x16";
    const FALLBACK_STATUS: &str = "Terminal clipboard unavailable; pasted internal clipboard";

    let config_path = temp_path("dun-pty-osc52-timeout-config", "conf");
    fs::write(&config_path, "clipboard.osc52.allow_read = true\n")?;
    let case = TerminalCase::new(
        "xterm-256color OSC 52 timeout",
        "xterm-256color",
        "en_US.UTF-8",
        "en_US.UTF-8",
        false,
        "UTF-8/256",
    );
    let run = run_dun_osc52_in_pty(
        &expect,
        case,
        &[OsStr::new("--config"), config_path.as_os_str()],
        "Untitled",
        SEED_CLEAR_AND_PASTE_EXTERNAL,
        None,
        "Termi",
    );
    let _ = fs::remove_file(&config_path);
    let run = run?;

    assert!(
        run.status.success(),
        "{} failed with status {:?}\n{}",
        case.name,
        run.status,
        run.output
    );
    assert!(
        run.osc52_query_observed,
        "{} did not observe the exact OSC 52 query\n{}",
        case.name, run.output
    );
    assert_output_contains(&run.output, "\x1b]52;c;?\x07", case.name);

    let elapsed = run
        .osc52_elapsed
        .expect("OSC 52 fallback marker should record elapsed time");
    assert!(
        elapsed >= Duration::from_millis(500),
        "fallback completed before 500 ms: {elapsed:?}\n{}",
        run.output
    );
    assert!(
        elapsed < Duration::from_secs(3),
        "fallback exceeded the bounded PTY ceiling: {elapsed:?}\n{}",
        run.output
    );

    let grid = run
        .terminal_grid_at_osc52_completion(case)
        .expect("OSC 52 harness should retain the completion grid");
    assert_line_contains(&grid, 23, FALLBACK_STATUS);
    let editor_line = grid.line_text(2);
    assert_eq!(
        editor_line.matches("internal-fallback").count(),
        1,
        "internal fallback was not restored exactly once: {editor_line:?}"
    );

    Ok(())
}

#[test]
fn pty_smoke_restores_the_terminal_when_it_panics() -> io::Result<()> {
    let _guard = pty_test_guard();
    let Some(expect) = command_on_path("expect") else {
        eprintln!("skipping PTY smoke test: expect(1) is not on PATH");
        return Ok(());
    };

    // CARGO_BIN_EXE_dun is a debug build, where panics unwind; the release build
    // aborts. That difference matters more than it looks: under unwind,
    // `TerminalGuard::drop` runs and restores the terminal all by itself, so
    // merely asserting the restore sequences are present proves nothing about
    // the hook -- the test passes with the hook removed entirely.
    //
    // What separates them is ORDER. The hook restores and *then* chains to the
    // default hook, which prints the panic message; `Drop` only runs afterwards,
    // as the stack unwinds. So:
    //
    //   hook installed:  ...[?1049l... panicked at ...
    //   hook removed:    ...panicked at ... [?1049l...
    //
    // Asserting the restore precedes the message is therefore the only thing
    // here that actually tests the hook. Verified by removing the
    // `install_panic_terminal_restore()` call: this assertion fails, the rest
    // still pass. The release abort path -- where the hook is not merely first
    // but the *only* restorer -- is still not covered by any test.
    let case = TerminalCase::new(
        "xterm-256color panic restore",
        "xterm-256color",
        "en_US.UTF-8",
        "en_US.UTF-8",
        false,
        "UTF-8/256",
    );
    let run = run_dun_in_pty_with_env(
        &expect,
        case,
        &[],
        "Untitled",
        b"",
        &[("DUN_TEST_PANIC", OsStr::new("1"))],
    )?;

    assert!(
        !run.status.success(),
        "{} unexpectedly succeeded\n{}",
        case.name,
        run.output
    );
    assert_output_contains(&run.output, "Untitled", case.name);
    assert_output_contains(&run.output, "[?1049l", case.name);
    assert_output_contains(&run.output, "[?2004l", case.name);
    assert_output_contains(&run.output, "DUN_TEST_PANIC", case.name);
    assert_restore_precedes_panic_message(&run.output, case.name);

    Ok(())
}

#[cfg(feature = "test-panic-hook")]
#[test]
fn pty_smoke_restores_the_terminal_when_a_release_build_aborts() -> io::Result<()> {
    let _guard = pty_test_guard();
    let Some(expect) = command_on_path("expect") else {
        eprintln!("skipping PTY smoke test: expect(1) is not on PATH");
        return Ok(());
    };

    let case = TerminalCase::new(
        "xterm-256color release abort restore",
        "xterm-256color",
        "en_US.UTF-8",
        "en_US.UTF-8",
        false,
        "UTF-8/256",
    );
    let run = run_dun_in_pty_with_env(
        &expect,
        case,
        &[],
        "Untitled",
        b"",
        &[("DUN_TEST_PANIC", OsStr::new("1"))],
    )?;

    assert_eq!(
        run.status.signal(),
        Some(6),
        "{} did not abort with SIGABRT\n{}",
        case.name,
        run.output
    );
    assert_output_contains(&run.output, "[?1049l", case.name);
    assert_output_contains(&run.output, "[?2004l", case.name);
    assert_output_contains(&run.output, "DUN_TEST_PANIC", case.name);
    // An abort cannot run TerminalGuard::drop, so the restore sequences are
    // present only if the panic hook ran; ordering adds nothing to this test.

    Ok(())
}

/// The panic hook restores the terminal and only then lets the default hook
/// print the message. If the terminal is restored *after* the message, the hook
/// did not run and `TerminalGuard::drop` picked up the pieces on the way out --
/// which the release profile (`panic = "abort"`) would never do.
fn assert_restore_precedes_panic_message(output: &str, case: &str) {
    let restore = output
        .find("[?1049l")
        .unwrap_or_else(|| panic!("{case}: the terminal was never restored\n{output}"));
    let message = output
        .find("panicked")
        .unwrap_or_else(|| panic!("{case}: the panic message never reached the user\n{output}"));

    assert!(
        restore < message,
        "{case}: the terminal was restored at {restore}, after the panic message at {message} -- \
         the panic hook did not run, only TerminalGuard::drop did, and a release build would have \
         aborted before reaching it\n{output}"
    );
}

#[test]
fn pty_smoke_restores_mouse_capture_when_it_panics() -> io::Result<()> {
    let _guard = pty_test_guard();
    let Some(expect) = command_on_path("expect") else {
        eprintln!("skipping PTY smoke test: expect(1) is not on PATH");
        return Ok(());
    };

    let config_path = temp_path("dun-pty-panic-mouse-config", "conf");
    fs::write(&config_path, "mouse.enabled = true\n")?;
    let case = TerminalCase::new(
        "xterm-256color mouse panic restore",
        "xterm-256color",
        "en_US.UTF-8",
        "en_US.UTF-8",
        false,
        "UTF-8/256",
    );
    let run = run_dun_in_pty_with_env(
        &expect,
        case,
        &[OsStr::new("--config"), config_path.as_os_str()],
        "Untitled",
        b"",
        &[("DUN_TEST_PANIC", OsStr::new("1"))],
    );
    let _ = fs::remove_file(&config_path);
    let run = run?;

    assert!(
        !run.status.success(),
        "{} unexpectedly succeeded\n{}",
        case.name,
        run.output
    );
    assert_output_contains(&run.output, "Untitled", case.name);
    assert_output_contains(&run.output, "[?1049l", case.name);
    assert_output_contains(&run.output, "[?2004l", case.name);
    assert_output_contains(&run.output, "DUN_TEST_PANIC", case.name);
    assert_output_contains(&run.output, "\x1b[?1000h\x1b[?1002h\x1b[?1006h", case.name);
    assert_output_contains(&run.output, "\x1b[?1006l\x1b[?1002l\x1b[?1000l", case.name);
    for sequence in ["[?1003h", "[?1003l", "[?1015h", "[?1015l"] {
        assert_output_not_contains(&run.output, sequence, case.name);
    }

    Ok(())
}

#[test]
fn pty_smoke_shell_escape_suspends_and_resumes_terminal() -> io::Result<()> {
    let _guard = pty_test_guard();
    let Some(expect) = command_on_path("expect") else {
        eprintln!("skipping PTY smoke test: expect(1) is not on PATH");
        return Ok(());
    };

    let shell_path = temp_path("dun-pty-shell-escape", "sh");
    fs::write(
        &shell_path,
        "#!/bin/sh\nprintf 'dun-shell-escape-smoke\\n'\nexit 0\n",
    )?;
    let mut permissions = fs::metadata(&shell_path)?.permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&shell_path, permissions)?;

    let case = TerminalCase::new(
        "xterm-256color shell escape",
        "xterm-256color",
        "en_US.UTF-8",
        "en_US.UTF-8",
        false,
        "UTF-8/256",
    );
    let run = run_dun_in_pty_with_env(
        &expect,
        case,
        &[],
        "Untitled",
        b"\x18s\x11",
        &[("SHELL", shell_path.as_os_str())],
    );
    let _ = fs::remove_file(&shell_path);
    let run = run?;

    assert!(
        run.status.success(),
        "{} failed with status {:?}\n{}",
        case.name,
        run.status,
        run.output
    );
    assert_output_contains(&run.output, "dun-shell-escape-smoke", case.name);
    assert_output_contains(&run.output, "Untitled", case.name);

    Ok(())
}

#[test]
fn pty_smoke_renders_escape_payloads_as_text() -> io::Result<()> {
    let _guard = pty_test_guard();
    let Some(expect) = command_on_path("expect") else {
        eprintln!("skipping PTY smoke test: expect(1) is not on PATH");
        return Ok(());
    };

    let file_path = temp_path("dun-pty-escape", "txt");
    fs::write(&file_path, b"safe\x1b]0;owned\x07\n\x1b[31mred?\x1b[0m\n")?;
    let case = TerminalCase::new(
        "xterm-256color escape payload",
        "xterm-256color",
        "en_US.UTF-8",
        "en_US.UTF-8",
        false,
        "UTF-8/256",
    );

    let run = run_dun_in_pty(&expect, case, &[file_path.as_os_str()], "safe", CTRL_Q);
    let _ = fs::remove_file(&file_path);
    let run = run?;

    assert!(
        run.status.success(),
        "{} failed with status {:?}\n{}",
        case.name,
        run.status,
        run.output
    );
    assert_output_contains(&run.output, "safe", case.name);
    assert_output_contains(&run.output, "red?", case.name);
    assert_output_not_contains(&run.output, "\x1b]0;owned\x07", case.name);
    assert_output_not_contains(&run.output, "\x1b[31mred?\x1b[0m", case.name);

    Ok(())
}

#[test]
fn pty_smoke_opens_invalid_bytes_as_read_only_escapes() -> io::Result<()> {
    let _guard = pty_test_guard();
    let Some(expect) = command_on_path("expect") else {
        eprintln!("skipping PTY smoke test: expect(1) is not on PATH");
        return Ok(());
    };

    let file_path = temp_path("dun-pty-invalid", "bin");
    fs::write(&file_path, [b'o', b'k', 0xff, b'\n'])?;
    let case = TerminalCase::new(
        "xterm-256color invalid bytes",
        "xterm-256color",
        "en_US.UTF-8",
        "en_US.UTF-8",
        false,
        "UTF-8/256",
    );

    let run = run_dun_in_pty(
        &expect,
        case,
        &[file_path.as_os_str()],
        "dun-pty-invalid",
        CTRL_Q,
    );
    let _ = fs::remove_file(&file_path);
    let run = run?;

    assert!(
        run.status.success(),
        "{} failed with status {:?}\n{}",
        case.name,
        run.status,
        run.output
    );
    assert_output_contains(&run.output, "ok\\xFF", case.name);
    assert_output_contains(&run.output, "[Escaped Bytes]", case.name);

    Ok(())
}
