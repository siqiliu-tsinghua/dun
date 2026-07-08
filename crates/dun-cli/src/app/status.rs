#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct StatusEntry {
    pub(crate) level: StatusLevel,
    pub(crate) message: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum StatusLevel {
    Info,
    Error,
}

impl StatusLevel {
    pub(crate) fn for_message(message: &str) -> Self {
        let message = message.to_ascii_lowercase();
        if message.contains("failed") || message.contains("error") || message.contains("invalid") {
            Self::Error
        } else {
            Self::Info
        }
    }

    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::Info => "info",
            Self::Error => "error",
        }
    }
}
