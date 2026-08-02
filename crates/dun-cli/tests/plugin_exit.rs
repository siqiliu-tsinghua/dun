#![cfg(unix)]
#![forbid(unsafe_code)]

use std::ffi::OsStr;
use std::fmt::Write as _;
use std::fs;
use std::io;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

mod support;

use support::pty::{command_on_path, pty_test_guard};

const HELLO_JSON: &str =
    r#"{"v":0,"kind":"hello","request_id":0,"plugin_id":"","payload":{"host":"dun"}}"#;
const HELPER_SLEEP: Duration = Duration::from_secs(8);
const SENTINEL_MARGIN: Duration = Duration::from_millis(500);
const HUNG_HOST_OBSERVATION: Duration = Duration::from_millis(10_000);
const FAST_QUIT_BOUND: Duration = Duration::from_millis(400);
const EXIT_ELAPSED_MARKER: &str = "DUN_PLUGIN_EXIT_ELAPSED_MS=";
static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(0);

#[test]
fn normal_exit_sweeps_host_helper_after_graceful_shutdown() -> io::Result<()> {
    let _guard = pty_test_guard();
    let Some(expect) = command_on_path("expect") else {
        eprintln!("skipping plugin exit PTY test: expect(1) is not on PATH");
        return Ok(());
    };
    let Some(dd) = command_on_path("dd") else {
        eprintln!("skipping plugin exit PTY test: dd(1) is not on PATH");
        return Ok(());
    };
    let Some(sleep) = command_on_path("sleep") else {
        eprintln!("skipping plugin exit PTY test: sleep(1) is not on PATH");
        return Ok(());
    };

    let fixture = ExitFixture::new("graceful", &sleep)?;
    let host = fixture.write_graceful_host(&dd)?;
    let config = fixture.write_config(&host)?;
    let run = run_editor_and_quit(
        &expect,
        fixture.directory(),
        Some(&config),
        Some(&fixture.ready),
    )?;

    assert_success(&run, "graceful plugin exit");
    assert!(fixture.ready.is_file(), "helper never wrote ready sentinel");
    eprintln!(
        "plugin exit latency with one host: {} ms",
        run.quit_elapsed.as_millis()
    );
    assert!(
        run.quit_elapsed < FAST_QUIT_BOUND,
        "graceful one-host quit took {:?}, output:\n{}",
        run.quit_elapsed,
        run.output
    );

    thread::sleep(HELPER_SLEEP + SENTINEL_MARGIN);
    assert!(
        !fixture.survived.exists(),
        "helper survived normal editor exit"
    );
    Ok(())
}

#[test]
fn normal_exit_sweeps_helper_when_host_never_finishes_handshake() -> io::Result<()> {
    let _guard = pty_test_guard();
    let Some(expect) = command_on_path("expect") else {
        eprintln!("skipping hung-handshake PTY test: expect(1) is not on PATH");
        return Ok(());
    };
    let Some(sleep) = command_on_path("sleep") else {
        eprintln!("skipping hung-handshake PTY test: sleep(1) is not on PATH");
        return Ok(());
    };

    let fixture = ExitFixture::new("hung-handshake", &sleep)?;
    let host = fixture.write_hung_host(&sleep)?;
    let config = fixture.write_config(&host)?;
    let run = run_editor_and_quit(
        &expect,
        fixture.directory(),
        Some(&config),
        Some(&fixture.ready),
    )?;

    assert_success(&run, "hung-handshake plugin exit");
    assert!(fixture.ready.is_file(), "helper never wrote ready sentinel");
    assert!(
        run.quit_elapsed < Duration::from_secs(3),
        "hung host made quit exceed the shared deadline: {:?}\n{}",
        run.quit_elapsed,
        run.output
    );

    // The observation must outlast the helper's own sleep, or an absent
    // sentinel would only mean "still sleeping" and the assertion would pass
    // vacuously. The helper's sleep must in turn outlast the worst-case
    // ready-to-quit-plus-deadline window, or the sentinel appears because the
    // helper finished on its own rather than because cleanup failed. Both
    // margins are sized for a loaded VM, where the earlier 2s/3.25s pair
    // failed for the second reason. Host and helper still exit on their own,
    // so even a mutation run leaves no strays.
    thread::sleep(HUNG_HOST_OBSERVATION);
    assert!(
        !fixture.survived.exists(),
        "pre-handshake helper survived the post-deadline sweep"
    );
    Ok(())
}

#[test]
fn normal_exit_with_zero_hosts_stays_fast() -> io::Result<()> {
    let _guard = pty_test_guard();
    let Some(expect) = command_on_path("expect") else {
        eprintln!("skipping zero-host exit PTY test: expect(1) is not on PATH");
        return Ok(());
    };

    let directory = FixtureDirectory::new("zero-host")?;
    let run = run_editor_and_quit(&expect, directory.path(), None, None)?;
    assert_success(&run, "zero-host editor exit");
    eprintln!(
        "plugin exit latency with zero hosts: {} ms",
        run.quit_elapsed.as_millis()
    );
    assert!(
        run.quit_elapsed < FAST_QUIT_BOUND,
        "zero-host quit took {:?}, output:\n{}",
        run.quit_elapsed,
        run.output
    );
    Ok(())
}

struct ExitFixture {
    directory: FixtureDirectory,
    helper: PathBuf,
    ready: PathBuf,
    survived: PathBuf,
}

impl ExitFixture {
    fn new(label: &str, sleep: &Path) -> io::Result<Self> {
        let directory = FixtureDirectory::new(label)?;
        let helper = directory.path().join("helper.sh");
        let ready = directory.path().join("ready");
        let survived = directory.path().join("survived");
        write_executable(
            &helper,
            &format!(
                "#!/bin/sh\n: > {}\n{} {}\n: > {}\n",
                shell_quote(&ready),
                shell_quote(sleep),
                HELPER_SLEEP.as_secs(),
                shell_quote(&survived)
            ),
        )?;
        Ok(Self {
            directory,
            helper,
            ready,
            survived,
        })
    }

    fn directory(&self) -> &Path {
        self.directory.path()
    }

    fn write_graceful_host(&self, dd: &Path) -> io::Result<PathBuf> {
        let host = self.directory().join("graceful-host.sh");
        let ack = br#"{"v":0,"kind":"hello-ack","request_id":0,"plugin_id":"exit-fixture","payload":{"host_id":"exit-fixture","trust":"user-trusted-external"}}"#;
        write_executable(
            &host,
            &format!(
                "#!/bin/sh\n{} &\n{} bs=1 count={} of=/dev/null 2>/dev/null\nprintf '%b' '{}'\n{} bs=1 count=4 of=/dev/null 2>/dev/null\n",
                shell_quote(&self.helper),
                shell_quote(dd),
                4 + HELLO_JSON.len(),
                printf_octal_frame(ack),
                shell_quote(dd),
            ),
        )?;
        Ok(host)
    }

    fn write_hung_host(&self, sleep: &Path) -> io::Result<PathBuf> {
        let host = self.directory().join("hung-host.sh");
        write_executable(
            &host,
            &format!(
                "#!/bin/sh\n{} &\n{} 20\n",
                shell_quote(&self.helper),
                shell_quote(sleep)
            ),
        )?;
        Ok(host)
    }

    fn write_config(&self, host: &Path) -> io::Result<PathBuf> {
        let config = self.directory().join("config");
        fs::write(
            &config,
            format!(
                "plugin.exit-fixture.command = {}\nplugin.exit-fixture.trust = user-trusted-external\nplugin.exit-fixture.roles = log-filter\nplugin.exit-fixture.timeout_ms = 5000\n",
                config_quote(host)
            ),
        )?;
        Ok(config)
    }
}

struct FixtureDirectory {
    path: PathBuf,
}

impl FixtureDirectory {
    fn new(label: &str) -> io::Result<Self> {
        let sequence = NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed);
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or_default();
        let path = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!(
            "dun-plugin-exit-{label}-{}-{sequence}-{nanos}",
            std::process::id()
        ));
        fs::create_dir_all(&path)?;
        Ok(Self { path })
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for FixtureDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

struct ExitRun {
    status: ExitStatus,
    output: String,
    quit_elapsed: Duration,
}

fn run_editor_and_quit(
    expect: &Path,
    directory: &Path,
    config: Option<&Path>,
    ready: Option<&Path>,
) -> io::Result<ExitRun> {
    let script = directory.join("drive-exit.tcl");
    fs::write(&script, EXPECT_SCRIPT)?;
    let mut command = Command::new(expect);
    command
        .arg(&script)
        .env("TERM", "xterm-256color")
        .env("LANG", "en_US.UTF-8")
        .env("LC_CTYPE", "en_US.UTF-8")
        .env("COLUMNS", "80")
        .env("LINES", "24")
        .env_remove("COLORTERM")
        .env_remove("NO_COLOR")
        .env("DUN_TEST_BINARY", env!("CARGO_BIN_EXE_dun"))
        .env(
            "DUN_TEST_CONFIG",
            config
                .map(Path::as_os_str)
                .unwrap_or_else(|| OsStr::new("")),
        )
        .env(
            "DUN_TEST_READY",
            ready.map(Path::as_os_str).unwrap_or_else(|| OsStr::new("")),
        )
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let output = command.output()?;
    let mut combined = String::from_utf8_lossy(&output.stdout).into_owned();
    combined.push_str(&String::from_utf8_lossy(&output.stderr));
    let quit_elapsed = marker_millis(&combined).ok_or_else(|| {
        io::Error::other(format!(
            "expect output omitted {EXIT_ELAPSED_MARKER}:\n{combined}"
        ))
    })?;
    Ok(ExitRun {
        status: output.status,
        output: combined,
        quit_elapsed: Duration::from_millis(quit_elapsed),
    })
}

fn marker_millis(output: &str) -> Option<u64> {
    let start = output.rfind(EXIT_ELAPSED_MARKER)? + EXIT_ELAPSED_MARKER.len();
    let digits: String = output[start..]
        .chars()
        .take_while(char::is_ascii_digit)
        .collect();
    digits.parse().ok()
}

fn assert_success(run: &ExitRun, case: &str) {
    assert!(
        run.status.success(),
        "{case} failed with {:?}:\n{}",
        run.status,
        run.output
    );
}

fn write_executable(path: &Path, text: &str) -> io::Result<()> {
    fs::write(path, text)?;
    let mut permissions = fs::metadata(path)?.permissions();
    permissions.set_mode(0o700);
    fs::set_permissions(path, permissions)
}

fn config_quote(path: &Path) -> String {
    format!("\"{}\"", path.to_string_lossy().replace('"', "\\\""))
}

fn shell_quote(path: &Path) -> String {
    format!("'{}'", path.to_string_lossy().replace('\'', "'\\''"))
}

fn printf_octal_frame(payload: &[u8]) -> String {
    let mut encoded = String::new();
    for byte in u32::try_from(payload.len())
        .expect("fixture frame length fits u32")
        .to_le_bytes()
        .into_iter()
        .chain(payload.iter().copied())
    {
        write!(&mut encoded, "\\0{byte:03o}").expect("writing to a string cannot fail");
    }
    encoded
}

const EXPECT_SCRIPT: &str = r#"
set timeout 10
log_user 1
set ambiguous_width_probe {"\r\u2500\033\\\[6n\033\\\[c"}
if {$env(DUN_TEST_CONFIG) eq ""} {
    spawn -noecho $env(DUN_TEST_BINARY) --no-config
} else {
    spawn -noecho $env(DUN_TEST_BINARY) --config $env(DUN_TEST_CONFIG)
}
expect {
    -re $ambiguous_width_probe {
        send -- "\033\[1;2R\033\[?1;2c"
        exp_continue
    }
    -re {Untitled} {}
    eof {
        catch {wait}
        exit 125
    }
    timeout {
        catch {close}
        catch {wait}
        exit 124
    }
}
if {$env(DUN_TEST_READY) ne ""} {
    set ready_deadline [expr {[clock milliseconds] + 5000}]
    while {![file exists $env(DUN_TEST_READY)] && [clock milliseconds] < $ready_deadline} {
        after 10
    }
    if {![file exists $env(DUN_TEST_READY)]} {
        catch {close}
        catch {wait}
        exit 123
    }
}
after 100
set quit_started [clock milliseconds]
send -- "\021"
expect {
    eof {}
    timeout {
        catch {close}
        catch {wait}
        exit 124
    }
}
set quit_elapsed [expr {[clock milliseconds] - $quit_started}]
set wait_result [wait]
set exit_code [lindex $wait_result 3]
puts "DUN_PLUGIN_EXIT_ELAPSED_MS=$quit_elapsed"
if {[lindex $wait_result 4] eq "CHILDKILLED"} {
    exit 128
}
if {$exit_code eq ""} {
    exit 1
}
exit $exit_code
"#;
