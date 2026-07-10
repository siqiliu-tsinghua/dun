use crate::terminal::{duration_status_text, exit_status_text};
use crate::*;

pub(crate) fn command_output_buffer(text: &str) -> TextBuffer {
    TextBuffer::from_text_with_kind(BufferKind::ReadOnly, text)
}

pub(crate) fn command_output_text(result: &CommandRunResult) -> String {
    let mut out = String::from("Dun Command Output\n\n");
    out.push_str(&format!("Command: {}\n", result.command));
    out.push_str(&format!("Shell: {}\n", result.shell.to_string_lossy()));
    out.push_str(&format!("Status: {}\n", exit_status_text(result.status)));
    out.push_str(&format!(
        "Elapsed: {}\n",
        duration_status_text(result.elapsed)
    ));
    out.push_str(&format!(
        "Limit: {} bytes per stream\n",
        COMMAND_OUTPUT_STREAM_SOFT_LIMIT_BYTES
    ));
    out.push_str(&format!(
        "Stdout: {}\n",
        command_stream_summary(&result.stdout)
    ));
    out.push_str(&format!(
        "Stdout Lines: {}\n",
        command_stream_line_count(&result.stdout)
    ));
    out.push_str(&format!(
        "Stderr: {}\n",
        command_stream_summary(&result.stderr)
    ));
    out.push_str(&format!(
        "Stderr Lines: {}\n",
        command_stream_line_count(&result.stderr)
    ));
    out.push_str(&format!(
        "Truncated: {}\n",
        if result.stdout.truncated || result.stderr.truncated {
            "yes"
        } else {
            "no"
        }
    ));

    out.push_str(&format!(
        "\n--- stdout ({}) ---\n",
        command_stream_summary(&result.stdout)
    ));
    push_decoded_command_stream(&mut out, &result.stdout);
    out.push_str(&format!(
        "\n--- stderr ({}) ---\n",
        command_stream_summary(&result.stderr)
    ));
    push_decoded_command_stream(&mut out, &result.stderr);
    out
}

fn command_stream_summary(stream: &CapturedCommandStream) -> String {
    format!(
        "{} bytes, {}",
        stream.bytes.len(),
        if stream.truncated {
            "truncated"
        } else {
            "complete"
        }
    )
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

fn push_decoded_command_stream(out: &mut String, stream: &CapturedCommandStream) {
    if stream.bytes.is_empty() {
        out.push_str("(empty)\n");
    } else {
        let decoded = decode_file_text(stream.bytes.clone());
        out.push_str(&decoded.text);
        if !decoded.text.ends_with('\n') {
            out.push('\n');
        }
    }
    if stream.truncated {
        out.push_str("[truncated]\n");
    }
}
