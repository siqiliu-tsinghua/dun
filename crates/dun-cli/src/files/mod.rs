mod atomic;
mod dialog;
mod open;
mod save;
mod snapshot;
pub(crate) mod text;

#[cfg(test)]
pub(crate) use atomic::atomic_temp_path;
pub(crate) use atomic::{
    AtomicTempReconcileReport, atomic_write_text_file, reconcile_atomic_save_temp_files,
    status_with_atomic_temp_report,
};
pub(crate) use dialog::{
    common_entry_prefix, ensure_trailing_separator, expand_user_path, file_dialog_context,
    file_dialog_list_message, file_dialog_recent_input_for_path, is_completable_file_dialog_entry,
    list_file_dialog_entries, next_char_boundary, previous_char_boundary, single_line_paste_text,
};
pub(crate) use open::{
    LoadedTextBuffer, load_text_buffer, opened_file_status, reloaded_file_status, title_for_path,
};
pub(crate) use save::{path_error_detail, path_io_error, validate_save_snapshot};
pub(crate) use snapshot::{FileReadSnapshot, current_file_snapshot, validate_stable_file_read};
