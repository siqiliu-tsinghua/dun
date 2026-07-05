#![cfg(unix)]
#![forbid(unsafe_code)]

use std::ffi::OsStr;
use std::fs;
use std::io;

mod support;

use support::pty::{
    CTRL_Q, TerminalCase, assert_output_contains, assert_output_not_contains, command_on_path,
    pty_test_guard, run_dun_in_pty, temp_path,
};

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
        assert_output_contains(&run.output, "Ln 1/1, Col 1", case.name);
        if let Some(profile) = case.expected_profile {
            assert_output_contains(&run.output, profile, case.name);
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
    assert_output_contains(&run.output, "Opened ", case.name);
    assert_output_contains(&run.output, "Text UTF-8", case.name);

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

    Ok(())
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
        "Escaped bytes",
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
    assert_output_contains(&run.output, "Escaped bytes", case.name);

    Ok(())
}
