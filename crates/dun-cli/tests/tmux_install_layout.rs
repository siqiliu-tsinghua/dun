#![cfg(unix)]
#![forbid(unsafe_code)]

//! The installed layout, driven through the real binary.
//!
//! `scripts/install.sh --prefix` puts catalogs in `<prefix>/share/dun/i18n`
//! and the binary in `<prefix>/bin`, and nothing in the user's config
//! directory. Whether that arrangement translates the interface is a
//! property of `std::env::current_exe()` and of paths on disk, so the unit
//! tests can only check the arithmetic — this test checks the outcome, with
//! a copy of the shipped binary in a scratch prefix and an empty HOME.

use std::ffi::{OsStr, OsString};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::Duration;

mod support;

use support::pty::temp_path;
use support::tmux::{TmuxSession, tmux_test_guard};

const STARTUP_TIMEOUT: Duration = Duration::from_secs(5);

/// `<root>/bin/dun` plus an empty `<root>/home`; the caller decides whether
/// `<root>/share/dun/i18n` exists.
fn install_prefix(label: &str, with_shared_catalog: bool) -> io::Result<PathBuf> {
    let root = temp_path(&format!("dun-prefix-{label}"), "d");
    fs::create_dir_all(root.join("bin"))?;
    fs::create_dir_all(root.join("home"))?;

    let installed = root.join("bin").join("dun");
    fs::copy(env!("CARGO_BIN_EXE_dun"), &installed)?;

    if with_shared_catalog {
        let shared = root.join("share").join("dun").join("i18n");
        fs::create_dir_all(&shared)?;
        let shipped = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../i18n")
            .join("zh-Hans.conf");
        fs::copy(shipped, shared.join("zh-Hans.conf"))?;
    }

    Ok(root)
}

fn write_file(path: &Path, text: &str) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, text)
}

fn start_installed(label: &str, root: &Path) -> io::Result<Option<TmuxSession>> {
    start_installed_sized(label, root, 80, 24)
}

fn start_installed_sized(
    label: &str,
    root: &Path,
    cols: u16,
    rows: u16,
) -> io::Result<Option<TmuxSession>> {
    let home = OsString::from(format!("HOME={}", root.join("home").display()));
    let lc_all = OsString::from("LC_ALL=zh_CN.UTF-8");
    let lc_messages = OsString::from("LC_MESSAGES=zh_CN.UTF-8");
    let lang = OsString::from("LANG=zh_CN.UTF-8");
    let lc_ctype = OsString::from("LC_CTYPE=zh_CN.UTF-8");
    let binary = OsString::from(root.join("bin").join("dun"));

    // -u XDG_CONFIG_HOME so the developer's own config directory cannot be
    // what answers: the empty HOME is then the only per-user location, and
    // it has no i18n/ at all.
    let args = [
        OsStr::new("-u"),
        OsStr::new("XDG_CONFIG_HOME"),
        OsStr::new("-u"),
        OsStr::new("DUN_CONFIG"),
        home.as_os_str(),
        lc_all.as_os_str(),
        lc_messages.as_os_str(),
        lang.as_os_str(),
        lc_ctype.as_os_str(),
        binary.as_os_str(),
    ];
    TmuxSession::start_executable(label, cols, rows, OsStr::new("env"), &args)
}

#[test]
fn installed_prefix_translates_from_its_share_directory() -> io::Result<()> {
    let _guard = tmux_test_guard();
    let root = install_prefix("shared", true)?;
    let Some(session) = start_installed("install-shared", &root)? else {
        fs::remove_dir_all(&root)?;
        return Ok(());
    };

    // 文件 is the File menu; it can only have come from the catalog in
    // <prefix>/share/dun/i18n, since HOME is empty and English is what the
    // binary carries.
    let screen = session.capture_until_contains("文件", STARTUP_TIMEOUT)?;
    assert!(
        screen.line(0).contains("文件"),
        "menu bar should be translated from the installation directory: {:?}\n{}",
        screen.line(0),
        screen.text
    );

    drop(session);
    fs::remove_dir_all(&root)?;
    Ok(())
}

#[test]
fn the_same_binary_without_the_share_directory_stays_english() -> io::Result<()> {
    let _guard = tmux_test_guard();
    // The negative control that makes the test above mean something: same
    // binary, same locale, same empty HOME, only the catalog removed.
    let root = install_prefix("plain", false)?;
    let Some(session) = start_installed("install-plain", &root)? else {
        fs::remove_dir_all(&root)?;
        return Ok(());
    };

    let screen = session.capture_until_contains("File", STARTUP_TIMEOUT)?;
    assert!(
        !screen.text.contains("文件"),
        "without a catalog anywhere the interface must stay English\n{}",
        screen.text
    );

    drop(session);
    fs::remove_dir_all(&root)?;
    Ok(())
}

/// Both layers at once, through the real binary: a machine-wide setting the
/// user never mentioned is in force, *and* the user's own file overrides the
/// one they did. Replace-instead-of-overlay semantics cannot satisfy both
/// halves of this test at the same time.
#[test]
fn installed_config_and_user_config_are_both_in_force() -> io::Result<()> {
    let _guard = tmux_test_guard();
    let root = install_prefix("layers", false)?;

    // The installation: a theme, and a key nothing else binds.
    write_file(
        &root.join("share").join("dun").join("config"),
        "theme = turbo\nkey.app.help = F9\n",
    )?;
    // The user: the theme again, and nothing else.
    write_file(
        &root.join("home").join(".config").join("dun").join("config"),
        "theme = dark\n",
    )?;

    let Some(session) = start_installed_sized("install-layers", &root, 200, 44)? else {
        fs::remove_dir_all(&root)?;
        return Ok(());
    };
    session.capture_until_contains("Untitled", STARTUP_TIMEOUT)?;

    // F9 is bound by the installed file only. Help opening proves the base
    // layer survived a user file that never mentioned keybindings.
    session.send_keys(&["F9"])?;
    let screen = session.capture_until_contains("Dun Help", STARTUP_TIMEOUT)?;
    assert!(
        screen.text.contains("Dun Help"),
        "F9 from the installed config should open Help\n{}",
        screen.text
    );

    // F6 prints the effective theme: the user's `dark`, not the installed
    // `turbo`, and the installed file named as the base layer.
    session.send_keys(&["F6"])?;
    let screen = session.capture_until_contains("theme:", STARTUP_TIMEOUT)?;
    assert!(
        screen.text.contains("theme: dark"),
        "the user's theme should win over the installed one\n{}",
        screen.text
    );
    assert!(
        screen.text.contains("base layer:"),
        "diagnostics should name the installed layer\n{}",
        screen.text
    );

    drop(session);
    fs::remove_dir_all(&root)?;
    Ok(())
}
