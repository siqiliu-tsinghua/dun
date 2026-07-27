#![cfg(unix)]
#![forbid(unsafe_code)]

//! Automated live tests for the log-filter reference host: drive a real `dun`
//! in tmux, configured to load `hosts/python-logfilter`, and check the whole
//! capability path the unit tests can only reach with a mock host — the host
//! actually launches, the handshake's menu/keybinding contributions install,
//! and an invoked action opens the plugin's window.
//!
//! Skipped cleanly when tmux or `/usr/bin/python3` is unavailable (the host is
//! stdlib-only, launched through an absolute-path wrapper because `dun` clears
//! the environment).

use std::ffi::OsStr;
use std::fs;
use std::io;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

mod support;

use support::tmux::{TmuxSession, tmux_test_guard};

const STARTUP_TIMEOUT: Duration = Duration::from_secs(5);
const INTERACTION_TIMEOUT: Duration = Duration::from_secs(5);

fn python3() -> Option<PathBuf> {
    let path = PathBuf::from("/usr/bin/python3");
    path.exists().then_some(path)
}

fn host_script() -> Option<PathBuf> {
    fs::canonicalize(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../hosts/python-logfilter/dun-python-logfilter-host.py"),
    )
    .ok()
}

/// Write an absolute-path wrapper and a dun config that loads the host, and
/// return the config path. `dun` launches the command directly with a cleared
/// environment, so the wrapper spells out both interpreter and script.
fn write_config(python: &Path, host: &Path) -> io::Result<PathBuf> {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("dun-logfilter-{nanos}"));
    fs::create_dir_all(&dir)?;

    let wrapper = dir.join("wrapper.sh");
    fs::write(
        &wrapper,
        format!("#!/bin/sh\nexec {} {}\n", python.display(), host.display()),
    )?;
    fs::set_permissions(&wrapper, fs::Permissions::from_mode(0o755))?;

    let config = dir.join("dun.conf");
    fs::write(
        &config,
        format!(
            "plugin.logfilter.command = {}\n\
             plugin.logfilter.trust = user-trusted-external\n\
             plugin.logfilter.roles = log-filter\n",
            wrapper.display()
        ),
    )?;
    Ok(config)
}

/// Start `dun` with the log-filter host configured, or `None` to skip. `cols`
/// lets a test that ends up with several tiled plugin windows use a wider pane
/// so their content is not truncated.
fn start_with_host(label: &str, cols: u16) -> io::Result<Option<TmuxSession>> {
    let (Some(python), Some(host)) = (python3(), host_script()) else {
        eprintln!("skipping tmux log-filter test: python3 or the host is unavailable");
        return Ok(None);
    };
    let config = write_config(&python, &host)?;
    TmuxSession::start_dun(
        label,
        cols,
        24,
        &[OsStr::new("--config"), config.as_os_str()],
    )
}

fn start_with_host_locale(label: &str, cols: u16, locale: &str) -> io::Result<Option<TmuxSession>> {
    let (Some(python), Some(host)) = (python3(), host_script()) else {
        eprintln!("skipping tmux log-filter test: python3 or the host is unavailable");
        return Ok(None);
    };
    let config = write_config(&python, &host)?;
    TmuxSession::start_dun_with_locale(
        label,
        cols,
        24,
        locale,
        &[OsStr::new("--config"), config.as_os_str()],
    )
}

#[test]
fn tmux_logfilter_menu_is_injected_after_handshake() -> io::Result<()> {
    let _guard = tmux_test_guard();
    let Some(session) = start_with_host("logfilter-menu", 100)? else {
        return Ok(());
    };

    // The host holds menu + window, so it launches eagerly; after the handshake
    // its top-level "Log Filter" menu is appended to the menu bar. Reaching this
    // proves the real editor launched the host and installed its contribution.
    let screen = session.capture_until_contains("Log Filter", STARTUP_TIMEOUT)?;
    assert!(
        screen.line(0).contains("Log Filter"),
        "menu bar (row 0) should carry the plugin menu: {:?}\n{}",
        screen.line(0),
        screen.text
    );
    Ok(())
}

#[test]
fn tmux_translated_logfilter_menu_opens_with_english_alt_l() -> io::Result<()> {
    let _guard = tmux_test_guard();

    let Some(english) = start_with_host("logfilter-mnemonic-en", 100)? else {
        return Ok(());
    };
    english.capture_until_contains("Log Filter", STARTUP_TIMEOUT)?;
    english.send_keys(&["M-l"])?;
    let screen = english.capture_until_contains("Edit Pattern", INTERACTION_TIMEOUT)?;
    assert!(
        screen.text.contains("Edit Pattern"),
        "Alt+L should open the English plugin menu\n{}",
        screen.text
    );
    drop(english);

    let Some(translated) = start_with_host_locale("logfilter-mnemonic-zh", 100, "zh_CN.UTF-8")?
    else {
        return Ok(());
    };
    translated.capture_until_contains("日志过滤 (L)", STARTUP_TIMEOUT)?;
    translated.send_keys(&["M-l"])?;
    let screen = translated.capture_until_contains("Edit Pattern", INTERACTION_TIMEOUT)?;
    assert!(
        screen.text.contains("Edit Pattern"),
        "Alt+L should open the translated plugin menu\n{}",
        screen.text
    );
    Ok(())
}

#[test]
fn tmux_logfilter_keybinding_opens_the_scratch_window() -> io::Result<()> {
    let _guard = tmux_test_guard();
    let Some(session) = start_with_host("logfilter-scratch", 100)? else {
        return Ok(());
    };

    // Wait for the host to be live (its menu is injected), then trigger the
    // host's own leader chord Ctrl+T e (Edit Pattern, kind=scratch).
    session.capture_until_contains("Log Filter", STARTUP_TIMEOUT)?;
    session.send_keys(&["C-t", "e"])?;

    // A plugin-owned scratch window opens, titled "logfilter: edit".
    let screen = session.capture_until_contains("logfilter: edit", INTERACTION_TIMEOUT)?;
    assert!(
        screen.text.contains("logfilter: edit"),
        "the scratch window should open on the leader chord\n{}",
        screen.text
    );
    Ok(())
}

#[test]
fn tmux_logfilter_execute_shows_the_result_in_the_surface() -> io::Result<()> {
    let _guard = tmux_test_guard();
    let Some(session) = start_with_host("logfilter-execute", 150)? else {
        return Ok(());
    };

    // Open the scratch window (Ctrl+T e), type a pattern into it with dun's own
    // editing engine, then submit it with Ctrl+T a (Apply, kind=execute). The
    // host adopts the text as its pattern and echoes a summary, which fills its
    // surface window — the full scratch-input -> execute -> surface loop.
    session.capture_until_contains("Log Filter", STARTUP_TIMEOUT)?;
    session.send_keys(&["C-t", "e"])?;
    session.capture_until_contains("logfilter: edit", INTERACTION_TIMEOUT)?;
    session.send_keys(&["needle"])?;
    session.send_keys(&["C-t", "a"])?;

    let screen = session.capture_until_contains("Filter pattern set to", INTERACTION_TIMEOUT)?;
    assert!(
        screen.text.contains("needle"),
        "the execute result should carry the submitted pattern into the surface\n{}",
        screen.text
    );
    Ok(())
}

#[test]
fn tmux_logfilter_filters_command_output_into_the_surface() -> io::Result<()> {
    let _guard = tmux_test_guard();
    let Some(session) = start_with_host("logfilter-stream", 150)? else {
        return Ok(());
    };
    session.capture_until_contains("Log Filter", STARTUP_TIMEOUT)?;

    // Ctrl+X o opens dun's Run Command prompt (the chord letter is lowercase).
    // Run a command whose stdout is fed to the log-filter host; with no pattern
    // set every line is kept, and they fill the host's surface window titled
    // "logfilter: command-output" — the command-output -> stream-read -> surface
    // path (the one the chunking fix repaired) exercised live.
    session.send_keys(&["C-x", "o"])?;
    session.capture_until_contains("Run Command", INTERACTION_TIMEOUT)?;
    session.send_keys(&["seq", "Space", "5"])?;
    session.send_keys(&["Enter"])?;

    let screen =
        session.capture_until_contains("logfilter: command-output", INTERACTION_TIMEOUT)?;
    assert!(
        screen.text.contains("logfilter: command-output"),
        "the command output should be filtered into the plugin surface\n{}",
        screen.text
    );
    Ok(())
}
