use std::io;
use std::path::Path;

use super::{FileReadSnapshot, current_file_snapshot};

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
        format!("{}: {}", path_error_label(path), path_error_detail(&error)),
    )
}

fn path_error_label(path: &Path) -> String {
    if path.as_os_str().is_empty() {
        "(empty path)".to_string()
    } else {
        path.display().to_string()
    }
}

pub(crate) fn path_error_detail(error: &io::Error) -> String {
    let message = error.to_string();
    match error.kind() {
        io::ErrorKind::NotFound if message == "parent directory does not exist" => message,
        io::ErrorKind::NotFound => "not found".to_string(),
        io::ErrorKind::PermissionDenied if message == "destination is read-only" => message,
        io::ErrorKind::PermissionDenied => "permission denied".to_string(),
        _ => message,
    }
}
