#![allow(unused_imports)]

use super::support::*;

#[test]
fn parse_cli_args_accepts_no_path_or_single_path() {
    assert_eq!(
        parse_cli_args(Vec::<&str>::new()).unwrap(),
        CliAction::Run {
            config_path: None,
            no_config: false,
            path: None,
        }
    );

    assert_eq!(
        parse_cli_args(["sample.txt"]).unwrap(),
        CliAction::Run {
            config_path: None,
            no_config: false,
            path: Some(PathBuf::from("sample.txt"))
        }
    );
}

#[test]
fn parse_cli_args_accepts_help_version_and_separator() {
    assert_eq!(parse_cli_args(["--help"]).unwrap(), CliAction::Help);
    assert_eq!(parse_cli_args(["-h"]).unwrap(), CliAction::Help);
    assert_eq!(parse_cli_args(["--version"]).unwrap(), CliAction::Version);
    assert_eq!(parse_cli_args(["-V"]).unwrap(), CliAction::Version);
    assert_eq!(
        parse_cli_args(["--dump-config"]).unwrap(),
        CliAction::DumpConfig
    );
    assert_eq!(
        parse_cli_args(["--", "--literal-path"]).unwrap(),
        CliAction::Run {
            config_path: None,
            no_config: false,
            path: Some(PathBuf::from("--literal-path"))
        }
    );
}

#[test]
fn parse_cli_args_accepts_config_options() {
    assert_eq!(
        parse_cli_args(["--config", "dun.conf", "sample.txt"]).unwrap(),
        CliAction::Run {
            config_path: Some(PathBuf::from("dun.conf")),
            no_config: false,
            path: Some(PathBuf::from("sample.txt")),
        }
    );
    assert_eq!(
        parse_cli_args(["--config=dun.conf"]).unwrap(),
        CliAction::Run {
            config_path: Some(PathBuf::from("dun.conf")),
            no_config: false,
            path: None,
        }
    );
    assert_eq!(
        parse_cli_args(["--no-config", "sample.txt"]).unwrap(),
        CliAction::Run {
            config_path: None,
            no_config: true,
            path: Some(PathBuf::from("sample.txt")),
        }
    );
}

#[test]
fn parse_cli_args_reports_usage_errors() {
    assert_eq!(
        parse_cli_args(["--bad"]).unwrap_err().to_string(),
        "unknown option --bad"
    );
    assert_eq!(
        parse_cli_args(["one", "two"]).unwrap_err().to_string(),
        "expected at most one path, got 2"
    );
    assert_eq!(
        parse_cli_args(["--help", "file.txt"])
            .unwrap_err()
            .to_string(),
        "options --help, --version, and --dump-config cannot be combined with paths"
    );
    assert_eq!(
        parse_cli_args(["--help", "--version"])
            .unwrap_err()
            .to_string(),
        "only one of --help, --version, or --dump-config may be used"
    );
    assert_eq!(
        parse_cli_args(["--config"]).unwrap_err().to_string(),
        "missing path after --config"
    );
    assert_eq!(
        parse_cli_args(["--config", "one", "--config", "two"])
            .unwrap_err()
            .to_string(),
        "--config may only be used once"
    );
    assert_eq!(
        parse_cli_args(["--config", "one", "--no-config"])
            .unwrap_err()
            .to_string(),
        "--config and --no-config cannot be used together"
    );
}

#[test]
fn cli_error_exit_codes_are_stable() {
    assert_eq!(CliError::Usage(UsageError::new("bad")).exit_code(), 2);
    assert_eq!(CliError::Io(io::Error::other("boom")).exit_code(), 1);
    assert!(cli_help_text().contains("Exit codes:"));
    assert!(cli_help_text().contains("--config PATH"));
    assert!(cli_help_text().contains("--dump-config"));
    assert_eq!(
        cli_version_text(),
        format!("dun {}", env!("CARGO_PKG_VERSION"))
    );
}
