use std::fmt;
use std::io;
use std::path::Path;

use dun_config::TextCatalog;

use super::{FileReadSnapshot, current_file_snapshot};
use crate::ui_text;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum PathErrorDetail {
    NotFound,
    PermissionDenied,
    ParentMissing,
    DestinationReadOnly,
    Other(String),
}

impl PathErrorDetail {
    pub(crate) fn classify(error: &io::Error) -> Self {
        let message = error.to_string();
        match error.kind() {
            io::ErrorKind::NotFound if message == "parent directory does not exist" => {
                Self::ParentMissing
            }
            io::ErrorKind::NotFound => Self::NotFound,
            io::ErrorKind::PermissionDenied if message == "destination is read-only" => {
                Self::DestinationReadOnly
            }
            io::ErrorKind::PermissionDenied => Self::PermissionDenied,
            _ => Self::Other(message),
        }
    }

    pub(crate) fn render(&self, catalog: &TextCatalog) -> String {
        match self {
            Self::NotFound => {
                ui_text::tr(catalog, ui_text::STATUS_PATH_ERROR_NOT_FOUND).to_string()
            }
            Self::PermissionDenied => {
                ui_text::tr(catalog, ui_text::STATUS_PATH_ERROR_PERMISSION_DENIED).to_string()
            }
            Self::ParentMissing => {
                ui_text::tr(catalog, ui_text::STATUS_PATH_ERROR_PARENT_MISSING).to_string()
            }
            Self::DestinationReadOnly => {
                ui_text::tr(catalog, ui_text::STATUS_PATH_ERROR_DESTINATION_READ_ONLY).to_string()
            }
            Self::Other(message) => message.clone(),
        }
    }
}

/// The English form, for the `io::Error` that escapes to the CLI's own
/// `eprintln!` at startup. It renders through the catalog table with an empty
/// catalog rather than repeating the English, so this and the in-editor status
/// text cannot drift apart.
impl fmt::Display for PathErrorDetail {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.render(&TextCatalog::empty()))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PathIoError {
    pub(crate) label: String,
    pub(crate) detail: PathErrorDetail,
}

impl fmt::Display for PathIoError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let catalog = TextCatalog::empty();
        formatter.write_str(&ui_text::tr_fmt(
            &catalog,
            ui_text::STATUS_PATH_ERROR_FRAME,
            &[&self.label, &self.detail.render(&catalog)],
        ))
    }
}

impl std::error::Error for PathIoError {}

pub(crate) fn validate_save_snapshot(
    snapshot: Option<FileReadSnapshot>,
    path: &Path,
) -> io::Result<()> {
    let Some(snapshot) = snapshot else {
        return Ok(());
    };

    match current_file_snapshot(path) {
        Ok(current) if current == snapshot => Ok(()),
        Ok(_) => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "file changed on disk; reload before saving or use Save As",
        )),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Err(io::Error::new(
            io::ErrorKind::NotFound,
            "file no longer exists; use Save As",
        )),
        Err(error) => Err(error),
    }
}

pub(crate) fn path_io_error(path: &Path, error: io::Error) -> io::Error {
    let kind = error.kind();
    io::Error::new(
        kind,
        PathIoError {
            label: path_error_label(path),
            detail: PathErrorDetail::classify(&error),
        },
    )
}

pub(crate) fn path_error_status_text(catalog: &TextCatalog, error: &io::Error) -> String {
    let Some(error) = error
        .get_ref()
        .and_then(|error| error.downcast_ref::<PathIoError>())
    else {
        return error.to_string();
    };
    let detail = error.detail.render(catalog);
    ui_text::tr_fmt(
        catalog,
        ui_text::STATUS_PATH_ERROR_FRAME,
        &[&error.label, &detail],
    )
}

fn path_error_label(path: &Path) -> String {
    if path.as_os_str().is_empty() {
        "(empty path)".to_string()
    } else {
        path.display().to_string()
    }
}
