use std::fs;
use std::io;
use std::path::Path;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct FileReadSnapshot {
    len: u64,
    modified: Option<std::time::SystemTime>,
    #[cfg(unix)]
    device: u64,
    #[cfg(unix)]
    inode: u64,
}

impl FileReadSnapshot {
    pub(crate) fn from_metadata(metadata: &fs::Metadata) -> Self {
        #[cfg(unix)]
        use std::os::unix::fs::MetadataExt;

        Self {
            len: metadata.len(),
            modified: metadata.modified().ok(),
            #[cfg(unix)]
            device: metadata.dev(),
            #[cfg(unix)]
            inode: metadata.ino(),
        }
    }
}

pub(crate) fn current_file_snapshot(path: &Path) -> io::Result<FileReadSnapshot> {
    let metadata = fs::metadata(path)?;
    if !metadata.is_file() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "path is not a regular file",
        ));
    }

    Ok(FileReadSnapshot::from_metadata(&metadata))
}

pub(crate) fn validate_stable_file_read(
    path: &Path,
    before: FileReadSnapshot,
    bytes_read: u64,
) -> io::Result<()> {
    if bytes_read != before.len {
        return Err(file_changed_while_reading_error());
    }

    let after = match fs::metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            return Err(file_changed_while_reading_error());
        }
        Err(error) => return Err(error),
    };
    if !after.is_file() || FileReadSnapshot::from_metadata(&after) != before {
        return Err(file_changed_while_reading_error());
    }

    Ok(())
}

fn file_changed_while_reading_error() -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        "file changed while reading; retry open",
    )
}
