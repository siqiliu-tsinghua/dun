use std::fs;
use std::io::{self, Read};
use std::path::Path;

use dun_core::{FileTextEncoding, TextBuffer, decode_file_text};

use super::{FileReadSnapshot, validate_stable_file_read};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct LoadedTextBuffer {
    pub(crate) buffer: TextBuffer,
    pub(crate) encoding: FileTextEncoding,
    pub(crate) snapshot: Option<FileReadSnapshot>,
}

pub(crate) fn load_text_buffer(path: &Path, soft_limit: u64) -> io::Result<LoadedTextBuffer> {
    let read = read_editable_file_with_snapshot(path, soft_limit)?;
    let decoded = decode_file_text(read.bytes);
    Ok(LoadedTextBuffer {
        buffer: TextBuffer::from_text_with_kind(decoded.encoding.buffer_kind(), &decoded.text),
        encoding: decoded.encoding,
        snapshot: Some(read.snapshot),
    })
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct EditableFileRead {
    bytes: Vec<u8>,
    snapshot: FileReadSnapshot,
}

fn read_editable_file_with_snapshot(path: &Path, soft_limit: u64) -> io::Result<EditableFileRead> {
    let metadata = fs::metadata(path)?;
    if metadata.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "path is a directory",
        ));
    }
    if !metadata.is_file() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "path is not a regular file",
        ));
    }
    if metadata.len() > soft_limit {
        return Err(editable_file_soft_limit_error(metadata.len(), soft_limit));
    }
    let snapshot = FileReadSnapshot::from_metadata(&metadata);

    let file = fs::File::open(path)?;
    let mut reader = file.take(soft_limit.saturating_add(1));
    let mut bytes = Vec::new();
    reader.read_to_end(&mut bytes)?;
    let bytes_read = bytes.len() as u64;
    if bytes_read > soft_limit {
        return Err(editable_file_soft_limit_error(bytes_read, soft_limit));
    }
    validate_stable_file_read(path, snapshot, bytes_read)?;

    Ok(EditableFileRead { bytes, snapshot })
}

fn editable_file_soft_limit_error(size: u64, soft_limit: u64) -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        format!(
            "too large for editable mode: {size} bytes exceeds the {soft_limit} byte soft limit",
        ),
    )
}

pub(crate) fn opened_file_status(path: &Path, encoding: FileTextEncoding) -> String {
    match encoding {
        FileTextEncoding::Utf8 => format!("Opened {}", path.display()),
        FileTextEncoding::EscapedBytes => format!(
            "Opened {} read-only: non-UTF-8 bytes shown as escapes",
            path.display()
        ),
    }
}

pub(crate) fn reloaded_file_status(path: &Path, encoding: FileTextEncoding) -> String {
    match encoding {
        FileTextEncoding::Utf8 => format!("Reloaded {}", path.display()),
        FileTextEncoding::EscapedBytes => format!(
            "Reloaded {} read-only: non-UTF-8 bytes shown as escapes",
            path.display()
        ),
    }
}

pub(crate) fn title_for_path(path: &Path) -> String {
    path.file_name()
        .filter(|name| !name.is_empty())
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string())
}
