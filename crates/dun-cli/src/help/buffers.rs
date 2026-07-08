use crate::*;

pub(crate) fn status_history_buffer(text: &str) -> TextBuffer {
    TextBuffer::from_text_with_kind(BufferKind::ReadOnly, text)
}

pub(crate) fn outline_buffer(text: &str) -> TextBuffer {
    TextBuffer::from_text_with_kind(BufferKind::ReadOnly, text)
}

pub(crate) fn search_results_buffer(text: &str) -> TextBuffer {
    TextBuffer::from_text_with_kind(BufferKind::ReadOnly, text)
}

pub(crate) fn config_diagnostics_buffer(text: &str) -> TextBuffer {
    TextBuffer::from_text_with_kind(BufferKind::ReadOnly, text)
}
