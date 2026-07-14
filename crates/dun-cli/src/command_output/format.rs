use crate::terminal::{duration_status_text, localized_exit_status_text};
use crate::*;

pub(crate) fn command_output_buffer(text: &str) -> TextBuffer {
    TextBuffer::from_text_with_kind(BufferKind::ReadOnly, text)
}

pub(crate) fn command_output_text(catalog: &TextCatalog, result: &CommandRunResult) -> String {
    let mut out = ui_text::tr(catalog, ui_text::COMMAND_OUTPUT_TITLE).to_string();
    out.push_str("\n\n");
    out.push_str(&ui_text::tr_fmt(
        catalog,
        ui_text::COMMAND_OUTPUT_COMMAND,
        &[&result.command],
    ));
    out.push('\n');
    out.push_str(&ui_text::tr_fmt(
        catalog,
        ui_text::COMMAND_OUTPUT_SHELL,
        &[&result.shell.to_string_lossy()],
    ));
    out.push('\n');
    let status = if result.timed_out {
        ui_text::tr(catalog, ui_text::COMMAND_OUTPUT_STATUS_TIMED_OUT).to_string()
    } else {
        localized_exit_status_text(catalog, result.status)
    };
    out.push_str(&ui_text::tr_fmt(
        catalog,
        ui_text::COMMAND_OUTPUT_STATUS,
        &[&status],
    ));
    out.push('\n');
    out.push_str(&ui_text::tr_fmt(
        catalog,
        ui_text::COMMAND_OUTPUT_ELAPSED,
        &[&duration_status_text(result.elapsed)],
    ));
    out.push('\n');
    out.push_str(&ui_text::tr_fmt(
        catalog,
        ui_text::COMMAND_OUTPUT_LIMIT,
        &[&COMMAND_OUTPUT_STREAM_SOFT_LIMIT_BYTES.to_string()],
    ));
    out.push('\n');
    out.push_str(&ui_text::tr_fmt(
        catalog,
        ui_text::COMMAND_OUTPUT_STDOUT,
        &[&command_stream_summary(catalog, &result.stdout)],
    ));
    out.push('\n');
    out.push_str(&ui_text::tr_fmt(
        catalog,
        ui_text::COMMAND_OUTPUT_STDOUT_LINES,
        &[&command_stream_line_count(&result.stdout).to_string()],
    ));
    out.push('\n');
    out.push_str(&ui_text::tr_fmt(
        catalog,
        ui_text::COMMAND_OUTPUT_STDERR,
        &[&command_stream_summary(catalog, &result.stderr)],
    ));
    out.push('\n');
    out.push_str(&ui_text::tr_fmt(
        catalog,
        ui_text::COMMAND_OUTPUT_STDERR_LINES,
        &[&command_stream_line_count(&result.stderr).to_string()],
    ));
    out.push('\n');
    let truncated = if result.stdout.truncated || result.stderr.truncated {
        ui_text::tr(catalog, ui_text::COMMAND_OUTPUT_YES)
    } else {
        ui_text::tr(catalog, ui_text::COMMAND_OUTPUT_NO)
    };
    out.push_str(&ui_text::tr_fmt(
        catalog,
        ui_text::COMMAND_OUTPUT_TRUNCATED,
        &[truncated],
    ));
    out.push_str("\n\n");
    out.push_str(&ui_text::tr_fmt(
        catalog,
        ui_text::COMMAND_OUTPUT_STDOUT_SECTION,
        &[&command_stream_summary(catalog, &result.stdout)],
    ));
    out.push('\n');
    push_decoded_command_stream(&mut out, catalog, &result.stdout);
    out.push('\n');
    out.push_str(&ui_text::tr_fmt(
        catalog,
        ui_text::COMMAND_OUTPUT_STDERR_SECTION,
        &[&command_stream_summary(catalog, &result.stderr)],
    ));
    out.push('\n');
    push_decoded_command_stream(&mut out, catalog, &result.stderr);
    out
}

fn command_stream_summary(catalog: &TextCatalog, stream: &CapturedCommandStream) -> String {
    let key = if stream.truncated {
        ui_text::COMMAND_OUTPUT_STREAM_TRUNCATED
    } else {
        ui_text::COMMAND_OUTPUT_STREAM_COMPLETE
    };
    ui_text::tr_fmt(catalog, key, &[&stream.bytes.len().to_string()])
}

fn command_stream_line_count(stream: &CapturedCommandStream) -> usize {
    if stream.bytes.is_empty() {
        return 0;
    }
    decode_file_text(stream.bytes.clone())
        .text
        .lines()
        .count()
        .max(1)
}

fn push_decoded_command_stream(
    out: &mut String,
    catalog: &TextCatalog,
    stream: &CapturedCommandStream,
) {
    if stream.bytes.is_empty() {
        out.push_str(ui_text::tr(catalog, ui_text::COMMAND_OUTPUT_EMPTY));
        out.push('\n');
    } else {
        let decoded = decode_file_text(stream.bytes.clone());
        out.push_str(&decoded.text);
        if !decoded.text.ends_with('\n') {
            out.push('\n');
        }
    }
    if stream.truncated {
        out.push_str(ui_text::tr(
            catalog,
            ui_text::COMMAND_OUTPUT_TRUNCATED_MARKER,
        ));
        out.push('\n');
    }
}
