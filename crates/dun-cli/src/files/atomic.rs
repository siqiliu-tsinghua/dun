use std::ffi::{OsStr, OsString};
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use dun_config::TextCatalog;

use crate::ui_text;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct AtomicTempReconcileReport {
    cleaned: usize,
    cleanup_failures: usize,
    recovery_candidates: Vec<PathBuf>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct AtomicWriteReport {
    pub(crate) temp_reconcile: AtomicTempReconcileReport,
}

pub(crate) fn atomic_write_text_file(path: &Path, text: &str) -> io::Result<AtomicWriteReport> {
    let destination = atomic_write_destination(path)?;
    let preexisting_temp_report = reconcile_atomic_save_temp_files(&destination);
    let recovery_candidates_to_preserve = preexisting_temp_report.recovery_candidates.clone();
    let existing_permissions = existing_atomic_write_permissions(&destination)?;
    let (temp_path, mut temp_file) = create_atomic_temp_file(&destination)?;

    let write_result = (|| {
        if let Some(permissions) = existing_permissions {
            temp_file.set_permissions(permissions)?;
        }
        temp_file.write_all(text.as_bytes())?;
        temp_file.sync_all()?;
        Ok(())
    })();

    if let Err(error) = write_result {
        drop(temp_file);
        let _ = fs::remove_file(&temp_path);
        return Err(error);
    }

    drop(temp_file);
    if let Err(error) = fs::rename(&temp_path, &destination) {
        let _ = fs::remove_file(&temp_path);
        return Err(error);
    }

    let post_save_temp_report =
        reconcile_atomic_save_temp_files_preserving(&destination, &recovery_candidates_to_preserve);
    Ok(AtomicWriteReport {
        temp_reconcile: merged_atomic_temp_reports(preexisting_temp_report, post_save_temp_report),
    })
}

fn atomic_write_destination(path: &Path) -> io::Result<PathBuf> {
    if path.as_os_str().is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "save path is empty",
        ));
    }

    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => fs::canonicalize(path),
        Ok(_) => Ok(path.to_path_buf()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(path.to_path_buf()),
        Err(error) => Err(error),
    }
}

fn existing_atomic_write_permissions(path: &Path) -> io::Result<Option<fs::Permissions>> {
    match fs::metadata(path) {
        Ok(metadata) => {
            if metadata.is_dir() {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "destination is a directory",
                ));
            }
            if !metadata.is_file() {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "destination is not a regular file",
                ));
            }

            let permissions = metadata.permissions();
            if permissions.readonly() {
                return Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    "destination is read-only",
                ));
            }

            Ok(Some(permissions))
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error),
    }
}

fn create_atomic_temp_file(path: &Path) -> io::Result<(PathBuf, fs::File)> {
    let directory = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    validate_save_parent_directory(directory)?;
    let file_name = path
        .file_name()
        .filter(|name| !name.is_empty())
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "save path has no file name"))?;

    for attempt in 0..1000 {
        let temp_path = atomic_temp_path(directory, file_name, attempt);
        match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp_path)
        {
            Ok(file) => return Ok((temp_path, file)),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
            Err(error) => return Err(error),
        }
    }

    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        "could not allocate atomic save temp file",
    ))
}

pub(crate) fn atomic_temp_path(directory: &Path, file_name: &OsStr, attempt: u32) -> PathBuf {
    let mut temp_name = OsString::from(".");
    temp_name.push(file_name);
    temp_name.push(format!(".dun-save-{}-{attempt}.tmp", std::process::id()));
    directory.join(temp_name)
}

pub(crate) fn reconcile_atomic_save_temp_files(path: &Path) -> AtomicTempReconcileReport {
    reconcile_atomic_save_temp_files_preserving(path, &[])
}

fn reconcile_atomic_save_temp_files_preserving(
    path: &Path,
    preserve: &[PathBuf],
) -> AtomicTempReconcileReport {
    let Ok(destination) = atomic_write_destination(path) else {
        return AtomicTempReconcileReport::default();
    };
    let Some(file_name) = destination.file_name().filter(|name| !name.is_empty()) else {
        return AtomicTempReconcileReport::default();
    };
    let directory = destination
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));

    let destination_modified = fs::metadata(&destination)
        .and_then(|metadata| metadata.modified())
        .ok();
    let Ok(entries) = fs::read_dir(directory) else {
        return AtomicTempReconcileReport::default();
    };

    let mut report = AtomicTempReconcileReport::default();
    for entry in entries.filter_map(Result::ok) {
        let entry_file_name = entry.file_name();
        if !is_atomic_temp_file_name_for(file_name, &entry_file_name) {
            continue;
        }

        let path = entry.path();
        if preserve.iter().any(|preserved| preserved == &path) {
            report.recovery_candidates.push(path);
            continue;
        }

        if atomic_temp_file_is_obsolete(&path, destination_modified) {
            match fs::remove_file(&path) {
                Ok(()) => report.cleaned += 1,
                Err(_) => report.cleanup_failures += 1,
            }
        } else {
            report.recovery_candidates.push(path);
        }
    }

    report
}

fn merged_atomic_temp_reports(
    first: AtomicTempReconcileReport,
    second: AtomicTempReconcileReport,
) -> AtomicTempReconcileReport {
    let mut recovery_candidates = first.recovery_candidates;
    for candidate in second.recovery_candidates {
        if !recovery_candidates.contains(&candidate) {
            recovery_candidates.push(candidate);
        }
    }

    AtomicTempReconcileReport {
        cleaned: first.cleaned + second.cleaned,
        cleanup_failures: first.cleanup_failures + second.cleanup_failures,
        recovery_candidates,
    }
}

fn is_atomic_temp_file_name_for(destination_file_name: &OsStr, candidate: &OsStr) -> bool {
    let mut prefix = OsString::from(".");
    prefix.push(destination_file_name);
    prefix.push(".dun-save-");
    let prefix = prefix.to_string_lossy();
    let candidate = candidate.to_string_lossy();

    let Some(suffix) = candidate
        .strip_prefix(&*prefix)
        .and_then(|suffix| suffix.strip_suffix(".tmp"))
    else {
        return false;
    };
    let Some((pid, attempt)) = suffix.split_once('-') else {
        return false;
    };

    !pid.is_empty()
        && !attempt.is_empty()
        && pid.bytes().all(|byte| byte.is_ascii_digit())
        && attempt.bytes().all(|byte| byte.is_ascii_digit())
}

fn atomic_temp_file_is_obsolete(
    path: &Path,
    destination_modified: Option<std::time::SystemTime>,
) -> bool {
    let Some(destination_modified) = destination_modified else {
        return false;
    };
    let Ok(metadata) = fs::metadata(path) else {
        return false;
    };
    if !metadata.is_file() {
        return false;
    }

    metadata
        .modified()
        .is_ok_and(|modified| modified <= destination_modified)
}

pub(crate) fn status_with_atomic_temp_report(
    catalog: &TextCatalog,
    status: impl Into<String>,
    report: &AtomicTempReconcileReport,
) -> String {
    let mut status = status.into();
    let mut suffixes = Vec::new();

    if report.cleaned > 0 {
        suffixes.push(ui_text::tr_fmt(
            catalog,
            ui_text::STATUS_ATOMIC_CLEANED,
            &[&report.cleaned.to_string()],
        ));
    }
    if report.cleanup_failures > 0 {
        suffixes.push(ui_text::tr_fmt(
            catalog,
            ui_text::STATUS_ATOMIC_CLEAN_FAILED,
            &[&report.cleanup_failures.to_string()],
        ));
    }
    if let Some(first) = report.recovery_candidates.first() {
        if report.recovery_candidates.len() == 1 {
            suffixes.push(ui_text::tr_fmt(
                catalog,
                ui_text::STATUS_ATOMIC_RECOVERY_FOUND,
                &[&first.display().to_string()],
            ));
        } else {
            suffixes.push(ui_text::tr_fmt(
                catalog,
                ui_text::STATUS_ATOMIC_RECOVERY_FOUND_MANY,
                &[
                    &report.recovery_candidates.len().to_string(),
                    &first.display().to_string(),
                ],
            ));
        }
    }

    if !suffixes.is_empty() {
        status.push_str("; ");
        status.push_str(&suffixes.join("; "));
    }

    status
}

fn validate_save_parent_directory(directory: &Path) -> io::Result<()> {
    match fs::metadata(directory) {
        Ok(metadata) if metadata.is_dir() => Ok(()),
        Ok(_) => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "parent path is not a directory",
        )),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Err(io::Error::new(
            io::ErrorKind::NotFound,
            "parent directory does not exist",
        )),
        Err(error) => Err(error),
    }
}
