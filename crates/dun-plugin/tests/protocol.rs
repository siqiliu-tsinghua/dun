#![forbid(unsafe_code)]

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use dun_plugin::frame::FrameError;
use dun_plugin::proto::ProtocolError;
use dun_plugin::{HostClient, InputSnapshot, PluginError, Policy, StyleId, TrustClass};

const FIXTURE_HOST: &str = env!("CARGO_BIN_EXE_fixture-host");
const REVISION: u64 = 41;

fn policy(timeout: Duration) -> Policy {
    Policy {
        timeout,
        ..Policy::default()
    }
}

fn snapshot(language: &str) -> InputSnapshot {
    InputSnapshot {
        buffer_revision: REVISION,
        language: language.to_string(),
        first_line: 7,
        lines: vec!["fn main() {}".to_string()],
    }
}

fn launch(policy: Policy) -> HostClient {
    match HostClient::launch(Path::new(FIXTURE_HOST), "highlight", policy) {
        Ok(client) => client,
        Err(error) => panic!("fixture host launches: {error}"),
    }
}

fn request_error(language: &str, policy: Policy) -> PluginError {
    let mut client = launch(policy);
    match client.request_highlight(&snapshot(language)) {
        Ok(spans) => panic!("{language} unexpectedly returned spans: {spans:?}"),
        Err(error) => error,
    }
}

fn handshake_error(mode: &str) -> PluginError {
    let launcher = ModeLauncher::new(mode);
    // Handshake-mode hosts reply (or die) immediately; the generous timeout
    // only absorbs process-spawn latency under parallel test load.
    match HostClient::launch(launcher.path(), "highlight", policy(Duration::from_secs(5))) {
        Ok(client) => {
            drop(client);
            panic!("{mode} unexpectedly completed the handshake");
        }
        Err(error) => error,
    }
}

/// Launches the fixture in a handshake-misbehavior mode without shell
/// scripts: a hard link named `fixture-host--<mode>` points at the fixture
/// binary, and the fixture reads the mode from its own program name. A hard
/// link shares the already-scanned inode, so macOS does not re-run its
/// first-exec malware assessment for every test run (a fresh shebang script
/// per run cost hundreds of milliseconds and made short handshake timeouts
/// flaky under parallel load).
struct ModeLauncher {
    path: PathBuf,
}

impl ModeLauncher {
    fn new(mode: &str) -> Self {
        let fixture = Path::new(FIXTURE_HOST);
        let directory = fixture.parent().expect("fixture binary has a directory");
        let path = directory.join(format!("fixture-host--{mode}"));
        // Recreate the link so it always tracks the freshly built fixture
        // inode; racing sibling runs are fine because every run links the
        // same target.
        if fs::hard_link(fixture, &path).is_err() {
            let _ = fs::remove_file(&path);
            if let Err(error) = fs::hard_link(fixture, &path) {
                assert!(
                    error.kind() == std::io::ErrorKind::AlreadyExists,
                    "links launcher: {error}"
                );
            }
        }
        Self { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

#[test]
fn happy_path_round_trips_one_validated_span_and_shuts_down() {
    let mut client = launch(policy(Duration::from_secs(5)));
    assert_eq!(client.host_id(), "fixture");
    assert_eq!(client.trust(), TrustClass::UserTrustedExternal);

    let spans = client
        .request_highlight(&snapshot("rust"))
        .expect("fixture returns a valid span");
    assert_eq!(spans.len(), 1);
    assert_eq!(spans[0].line, 7);
    assert_eq!(spans[0].start_col, 0);
    assert_eq!(spans[0].end_col, 2);
    assert_eq!(spans[0].style, StyleId::Keyword);

    client.shutdown().expect("fixture shuts down cleanly");
}

#[test]
fn bad_version_handshake_reports_unsupported_version() {
    let error = handshake_error("bad-version");
    assert!(matches!(
        error,
        PluginError::Protocol(ProtocolError::UnsupportedVersion(9))
    ));
}

#[test]
fn bad_trust_handshake_reports_handshake_error() {
    let error = handshake_error("bad-trust");
    assert!(matches!(error, PluginError::Handshake(_)));
}

#[test]
fn missing_ack_reports_handshake_error() {
    let error = handshake_error("no-ack");
    assert!(matches!(error, PluginError::Handshake(_)));
}

#[test]
fn truncated_handshake_frame_reports_host_closed() {
    let error = handshake_error("garbage-frame");
    assert!(matches!(error, PluginError::HostClosed));
}

#[test]
fn crashing_host_reports_host_closed() {
    let error = request_error("crash-test", policy(Duration::from_secs(5)));
    assert!(matches!(error, PluginError::HostClosed));
}

#[test]
fn span_flood_reports_policy_violation() {
    let error = request_error("flood-test", policy(Duration::from_secs(5)));
    assert!(matches!(error, PluginError::PolicyViolation(_)));
}

#[test]
fn out_of_bounds_coordinate_reports_policy_violation() {
    let error = request_error("badcoord-test", policy(Duration::from_secs(5)));
    assert!(matches!(error, PluginError::PolicyViolation(_)));
}

#[test]
fn unknown_style_reports_policy_violation() {
    let error = request_error("badstyle-test", policy(Duration::from_secs(5)));
    assert!(matches!(error, PluginError::PolicyViolation(_)));
}

#[test]
fn wrong_request_id_reports_policy_violation() {
    let error = request_error("wrong-id-test", policy(Duration::from_secs(5)));
    assert!(matches!(error, PluginError::PolicyViolation(_)));
}

#[test]
fn oversized_frame_reports_frame_oversized() {
    let error = request_error("bigframe-test", policy(Duration::from_secs(5)));
    assert!(matches!(
        error,
        PluginError::Frame(FrameError::Oversized { .. })
    ));
}

#[test]
fn malformed_json_response_reports_protocol_error() {
    let error = request_error("malformed-json-test", policy(Duration::from_secs(5)));
    assert!(
        matches!(error, PluginError::Protocol(_)),
        "expected a protocol error, got {error:?}"
    );
}

/// Diagnostic for the handshake-latency-spike investigation. Measures full
/// `HostClient::launch` (spawn + reader threads + hello/hello-ack) latency
/// sequentially and then with a burst of concurrent launches, to attribute
/// the spikes seen under `cargo test` parallelism. Not a gate — run with
/// `cargo test -p dun-plugin --ignored -- --nocapture measure_handshake`.
#[test]
#[ignore]
fn measure_handshake_latency_sequential_vs_parallel() {
    use std::thread;

    const N: usize = 24;
    let host = Path::new(FIXTURE_HOST);
    // Warm up so the first-exec scan / page-cache fill is not charged to the
    // first measured launch.
    drop(HostClient::launch(host, "highlight", policy(Duration::from_secs(5))).unwrap());

    let time_one = || {
        let start = Instant::now();
        let client = HostClient::launch(host, "highlight", policy(Duration::from_secs(5))).unwrap();
        let elapsed = start.elapsed();
        drop(client);
        elapsed
    };

    let mut sequential: Vec<Duration> = (0..N).map(|_| time_one()).collect();

    let parallel: Vec<Duration> = {
        let handles: Vec<_> = (0..N)
            .map(|_| {
                thread::spawn(move || {
                    let start = Instant::now();
                    let client =
                        HostClient::launch(host, "highlight", policy(Duration::from_secs(5)))
                            .unwrap();
                    let elapsed = start.elapsed();
                    drop(client);
                    elapsed
                })
            })
            .collect();
        handles.into_iter().map(|h| h.join().unwrap()).collect()
    };

    let summarize = |label: &str, samples: &mut Vec<Duration>| {
        samples.sort_unstable();
        let ms = |d: Duration| d.as_secs_f64() * 1000.0;
        let median = samples[samples.len() / 2];
        let max = *samples.last().unwrap();
        let min = *samples.first().unwrap();
        println!(
            "{label:12} n={} min={:.1}ms median={:.1}ms max={:.1}ms",
            samples.len(),
            ms(min),
            ms(median),
            ms(max)
        );
    };

    let mut parallel = parallel;
    summarize("sequential", &mut sequential);
    summarize("parallel", &mut parallel);
}

#[test]
fn diagnostic_flood_reports_policy_violation() {
    let error = request_error("diag-flood-test", policy(Duration::from_secs(5)));
    assert!(matches!(error, PluginError::PolicyViolation(_)));
}

#[test]
fn stale_revision_reports_stale_revision() {
    let error = request_error("stale-test", policy(Duration::from_secs(5)));
    assert!(matches!(
        error,
        PluginError::StaleRevision {
            expected: REVISION,
            received: Some(40)
        }
    ));
}

#[test]
fn slow_host_times_out_promptly() {
    // The policy timeout also bounds the handshake, so it cannot be made
    // tiny without turning spawn latency into flakiness; 2 s is far below
    // the fixture's 30 s sleep, which is what "promptly" means here.
    let timeout = Duration::from_secs(2);
    let mut client = launch(policy(timeout));
    let started = Instant::now();
    let error = match client.request_highlight(&snapshot("slow-test")) {
        Ok(spans) => panic!("slow-test unexpectedly returned spans: {spans:?}"),
        Err(error) => error,
    };
    let elapsed = started.elapsed();

    assert!(matches!(error, PluginError::Timeout));
    assert!(
        elapsed < Duration::from_secs(10),
        "timeout took {elapsed:?}, expected well under the 30s host sleep"
    );
}

#[test]
fn stderr_flood_does_not_break_protocol_response() {
    let mut client = launch(policy(Duration::from_secs(5)));
    let spans = client
        .request_highlight(&snapshot("stderr-test"))
        .expect("stderr flooding does not block the response");
    assert_eq!(spans.len(), 1);
    assert_eq!(spans[0].style, StyleId::Keyword);
    client.shutdown().expect("fixture shuts down cleanly");
}
