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

/// Start `dun` with the log-filter host configured, or `None` to skip.
fn start_with_host(label: &str) -> io::Result<Option<TmuxSession>> {
    let (Some(python), Some(host)) = (python3(), host_script()) else {
        eprintln!("skipping tmux log-filter test: python3 or the host is unavailable");
        return Ok(None);
    };
    let config = write_config(&python, &host)?;
    TmuxSession::start_dun(
        label,
        100,
        24,
        &[OsStr::new("--config"), config.as_os_str()],
    )
}

#[test]
fn tmux_logfilter_menu_is_injected_after_handshake() -> io::Result<()> {
    let _guard = tmux_test_guard();
    let Some(session) = start_with_host("logfilter-menu")? else {
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
fn tmux_logfilter_keybinding_opens_the_scratch_window() -> io::Result<()> {
    let _guard = tmux_test_guard();
    let Some(session) = start_with_host("logfilter-scratch")? else {
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
