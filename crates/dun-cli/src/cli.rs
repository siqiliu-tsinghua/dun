use std::ffi::OsString;
use std::io;
use std::path::PathBuf;

const EXIT_RUNTIME_ERROR: u8 = 1;
const EXIT_USAGE_ERROR: u8 = 2;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum CliAction {
    Help,
    Version,
    DumpConfig,
    Run {
        config_path: Option<PathBuf>,
        no_config: bool,
        path: Option<PathBuf>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct UsageError {
    message: String,
}

impl UsageError {
    pub(crate) fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for UsageError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}", self.message)
    }
}

#[derive(Debug)]
pub(crate) enum CliError {
    Usage(UsageError),
    Io(io::Error),
}

impl CliError {
    pub(crate) const fn exit_code(&self) -> u8 {
        match self {
            Self::Usage(_) => EXIT_USAGE_ERROR,
            Self::Io(_) => EXIT_RUNTIME_ERROR,
        }
    }
}

impl From<UsageError> for CliError {
    fn from(error: UsageError) -> Self {
        Self::Usage(error)
    }
}

impl std::fmt::Display for CliError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Usage(error) => write!(formatter, "dun: {error}\n\n{}", cli_usage_text()),
            Self::Io(error) => write!(formatter, "dun: {error}"),
        }
    }
}

pub(crate) fn parse_cli_args<I>(args: I) -> Result<CliAction, UsageError>
where
    I: IntoIterator,
    I::Item: Into<OsString>,
{
    let mut action = None;
    let mut paths = Vec::new();
    let mut config_path = None;
    let mut no_config = false;
    let mut parse_options = true;
    let mut args = args.into_iter();

    while let Some(arg) = args.next() {
        let arg = arg.into();
        if parse_options && arg == "--" {
            parse_options = false;
            continue;
        }

        if parse_options {
            match arg.to_string_lossy().as_ref() {
                "-h" | "--help" => {
                    set_cli_action(&mut action, CliAction::Help)?;
                    continue;
                }
                "-V" | "--version" => {
                    set_cli_action(&mut action, CliAction::Version)?;
                    continue;
                }
                "--dump-config" => {
                    set_cli_action(&mut action, CliAction::DumpConfig)?;
                    continue;
                }
                "--no-config" => {
                    if config_path.is_some() {
                        return Err(UsageError::new(
                            "--config and --no-config cannot be used together",
                        ));
                    }
                    no_config = true;
                    continue;
                }
                "--config" => {
                    if no_config {
                        return Err(UsageError::new(
                            "--config and --no-config cannot be used together",
                        ));
                    }
                    if config_path.is_some() {
                        return Err(UsageError::new("--config may only be used once"));
                    }
                    let Some(path) = args.next() else {
                        return Err(UsageError::new("missing path after --config"));
                    };
                    config_path = Some(PathBuf::from(path.into()));
                    continue;
                }
                option if option.starts_with("--config=") => {
                    if no_config {
                        return Err(UsageError::new(
                            "--config and --no-config cannot be used together",
                        ));
                    }
                    if config_path.is_some() {
                        return Err(UsageError::new("--config may only be used once"));
                    }
                    let path = option.trim_start_matches("--config=");
                    if path.is_empty() {
                        return Err(UsageError::new("missing path after --config"));
                    }
                    config_path = Some(PathBuf::from(path));
                    continue;
                }
                option if option.starts_with('-') && option != "-" => {
                    return Err(UsageError::new(format!("unknown option {option}")));
                }
                _ => {}
            }
        }

        paths.push(PathBuf::from(arg));
    }

    if let Some(action) = action {
        if paths.is_empty() {
            return Ok(action);
        }
        return Err(UsageError::new(
            "options --help, --version, and --dump-config cannot be combined with paths",
        ));
    }

    match paths.len() {
        0 => Ok(CliAction::Run {
            config_path,
            no_config,
            path: None,
        }),
        1 => Ok(CliAction::Run {
            config_path,
            no_config,
            path: paths.into_iter().next(),
        }),
        count => Err(UsageError::new(format!(
            "expected at most one path, got {count}"
        ))),
    }
}

fn set_cli_action(action: &mut Option<CliAction>, new_action: CliAction) -> Result<(), UsageError> {
    if action.is_some() {
        return Err(UsageError::new(
            "only one of --help, --version, or --dump-config may be used",
        ));
    }

    *action = Some(new_action);
    Ok(())
}

pub(crate) fn cli_version_text() -> String {
    format!("dun {}", env!("CARGO_PKG_VERSION"))
}

fn cli_usage_text() -> &'static str {
    "Usage: dun [OPTIONS] [--] [PATH]\nTry 'dun --help' for more information."
}

pub(crate) fn cli_help_text() -> &'static str {
    "\
dun - Microsoft Edit-like terminal editor

Usage:
  dun [OPTIONS] [--] [PATH]

Arguments:
  PATH              Open one UTF-8 text file at startup.

Options:
  -h, --help        Show this help text and exit.
  -V, --version     Show version information and exit.
      --config PATH Load configuration from PATH.
      --dump-config Print the built-in default configuration and exit.
      --no-config   Ignore DUN_CONFIG and default config paths.

Exit codes:
  0                 Success, --help, --version, or --dump-config.
  1                 Runtime, terminal, or file I/O error.
  2                 Command-line usage error.
"
}
