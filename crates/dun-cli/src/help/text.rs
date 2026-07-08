use crate::*;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct OutlineEntry {
    pub(crate) label: String,
    pub(crate) line: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct NumberedListRow {
    pub(crate) line: usize,
    pub(crate) index: usize,
}

pub(crate) fn numbered_list_rows(buffer: &TextBuffer) -> Vec<NumberedListRow> {
    (0..buffer.line_count())
        .filter_map(|line| {
            let index = numbered_list_index_for_line(buffer.line(line)?)?;
            Some(NumberedListRow { line, index })
        })
        .collect()
}

pub(crate) fn numbered_list_index_for_line(line: &str) -> Option<usize> {
    let trimmed = line.trim_start();
    let digit_len = trimmed
        .chars()
        .take_while(|ch| ch.is_ascii_digit())
        .map(char::len_utf8)
        .sum::<usize>();
    if digit_len == 0 || trimmed.get(digit_len..)?.chars().next()? != '.' {
        return None;
    }

    trimmed
        .get(..digit_len)?
        .parse::<usize>()
        .ok()?
        .checked_sub(1)
}

pub(crate) fn outline_entries_for_buffer(buffer: &TextBuffer) -> Vec<OutlineEntry> {
    (0..buffer.line_count())
        .filter_map(|line_index| {
            let line = buffer.line(line_index)?.trim();
            outline_label_for_line(line).map(|label| OutlineEntry {
                label,
                line: line_index,
            })
        })
        .collect()
}

pub(crate) fn outline_label_for_line(line: &str) -> Option<String> {
    const HEADINGS: &[&str] = &[
        "App",
        "File",
        "Edit",
        "Windows",
        "Prompts",
        "Selection",
        "Navigation",
        "File Dialogs",
        "Menus",
        "Notes",
        "Summary",
        "Paths",
        "Source",
        "Terminal",
        "Input",
        "Clipboard",
        "Limits",
        "Keymap",
        "File Dialog Keymap",
        "Index",
    ];
    if HEADINGS.contains(&line) || line.starts_with("--- stdout") || line.starts_with("--- stderr")
    {
        return Some(line.to_string());
    }

    markdown_outline_label(line)
        .or_else(|| bracket_section_outline_label(line))
        .or_else(|| rust_outline_label(line))
        .or_else(|| shell_outline_label(line))
}

pub(crate) fn markdown_outline_label(line: &str) -> Option<String> {
    let trimmed = line.trim_start();
    let level = trimmed.chars().take_while(|ch| *ch == '#').count();
    if !(1..=6).contains(&level) {
        return None;
    }
    let rest = trimmed.get(level..)?;
    if !rest.starts_with(char::is_whitespace) {
        return None;
    }
    let title = rest.trim();
    if title.is_empty() {
        None
    } else {
        Some(format!("{} {title}", "#".repeat(level)))
    }
}

pub(crate) fn bracket_section_outline_label(line: &str) -> Option<String> {
    let trimmed = line.trim();
    let inner = trimmed
        .strip_prefix("[[")
        .and_then(|rest| rest.strip_suffix("]]"))
        .or_else(|| {
            trimmed
                .strip_prefix('[')
                .and_then(|rest| rest.strip_suffix(']'))
        })?
        .trim();
    if inner.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

pub(crate) fn rust_outline_label(line: &str) -> Option<String> {
    let mut tokens = line.split_whitespace().peekable();
    while matches!(
        tokens.peek().copied(),
        Some("pub")
            | Some("async")
            | Some("unsafe")
            | Some("const")
            | Some("extern")
            | Some("default")
    ) || tokens.peek().is_some_and(|token| token.starts_with("pub("))
    {
        tokens.next();
    }

    match tokens.next()? {
        "fn" => {
            let name = outline_identifier(tokens.next()?)?;
            Some(format!("fn {name}"))
        }
        "struct" => {
            let name = outline_identifier(tokens.next()?)?;
            Some(format!("struct {name}"))
        }
        "enum" => {
            let name = outline_identifier(tokens.next()?)?;
            Some(format!("enum {name}"))
        }
        "trait" => {
            let name = outline_identifier(tokens.next()?)?;
            Some(format!("trait {name}"))
        }
        "mod" => {
            let name = outline_identifier(tokens.next()?)?;
            Some(format!("mod {name}"))
        }
        "impl" => {
            let rest = tokens.collect::<Vec<_>>().join(" ");
            let label = rest
                .split('{')
                .next()
                .unwrap_or_default()
                .split(" where ")
                .next()
                .unwrap_or_default()
                .trim();
            if label.is_empty() {
                Some("impl".to_string())
            } else {
                Some(format!("impl {label}"))
            }
        }
        _ => None,
    }
}

pub(crate) fn shell_outline_label(line: &str) -> Option<String> {
    let trimmed = line.trim();
    if let Some(rest) = trimmed.strip_prefix("function ") {
        let name = outline_identifier(rest.split_whitespace().next()?)?;
        return Some(format!("function {name}"));
    }

    let Some(name) = trimmed.split("()").next() else {
        return None;
    };
    let name = name.trim();
    if name.is_empty()
        || !trimmed[name.len()..].trim_start().starts_with("()")
        || !name.chars().all(is_shell_identifier_char)
    {
        return None;
    }
    let after = trimmed[name.len() + 2..].trim_start();
    if after.is_empty() || after.starts_with('{') {
        Some(format!("{name}()"))
    } else {
        None
    }
}

pub(crate) fn outline_identifier(token: &str) -> Option<String> {
    let end = token
        .find(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '_' || ch == '-'))
        .unwrap_or(token.len());
    let ident = token.get(..end)?.trim();
    if ident.is_empty() {
        None
    } else {
        Some(ident.to_string())
    }
}

pub(crate) fn is_shell_identifier_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_' || ch == '-'
}

pub(crate) fn outline_text(source: &str, entries: &[OutlineEntry]) -> String {
    let mut out = String::from("Dun Outline\n\n");
    out.push_str(&format!("Source: {source}\n"));
    out.push_str(&format!("Sections: {}\n\n", entries.len()));
    for (index, entry) in entries.iter().enumerate() {
        out.push_str(&format!(
            "{:>3}. L{:<5} {}\n",
            index + 1,
            entry.line + 1,
            entry.label
        ));
    }
    out
}

pub(crate) fn parse_outline_target(target: &str, entries: &[OutlineEntry]) -> Option<usize> {
    if let Ok(number) = target.parse::<usize>() {
        return number.checked_sub(1).filter(|index| *index < entries.len());
    }
    let normalized = normalize_command_line_token(target);
    entries
        .iter()
        .position(|entry| normalize_command_line_token(&entry.label).contains(&normalized))
}

pub(crate) fn search_results_text(
    source: &str,
    spec: &SearchSpec,
    matches: &[SearchMatch],
    buffer: &TextBuffer,
) -> String {
    let mut out = String::from("Dun Search Results\n\n");
    out.push_str(&format!("Source: {source}\n"));
    out.push_str(&format!("Query: {}\n", spec.display()));
    out.push_str(&format!("Matches: {}\n\n", matches.len()));
    for (index, item) in matches.iter().enumerate() {
        let line = buffer.line(item.range.start.line).unwrap_or_default();
        out.push_str(&format!(
            "{:>3}. L{}:C{} {}\n",
            index + 1,
            item.range.start.line + 1,
            item.range.start.column + 1,
            clipped_result_line(line)
        ));
    }
    out
}

pub(crate) fn clipped_result_line(line: &str) -> String {
    const LIMIT: usize = 96;
    let mut out = String::new();
    for (index, ch) in line.chars().enumerate() {
        if index >= LIMIT {
            out.push_str("...");
            break;
        }
        out.push(ch);
    }
    out
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ConfigDiagnosticsSection {
    Summary,
    Paths,
    Source,
    Terminal,
    Input,
    Clipboard,
    Limits,
    Keymap,
    FileDialogKeymap,
}

impl ConfigDiagnosticsSection {
    pub(crate) const fn heading(self) -> &'static str {
        match self {
            Self::Summary => "Summary",
            Self::Paths => "Paths",
            Self::Source => "Source",
            Self::Terminal => "Terminal",
            Self::Input => "Input",
            Self::Clipboard => "Clipboard",
            Self::Limits => "Limits",
            Self::Keymap => "Keymap",
            Self::FileDialogKeymap => "File Dialog Keymap",
        }
    }

    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::Summary => "summary",
            Self::Paths => "paths",
            Self::Source => "source",
            Self::Terminal => "terminal",
            Self::Input => "input",
            Self::Clipboard => "clipboard",
            Self::Limits => "limits",
            Self::Keymap => "keymap",
            Self::FileDialogKeymap => "file dialog keymap",
        }
    }
}

pub(crate) fn parse_config_diagnostics_section(input: &str) -> Option<ConfigDiagnosticsSection> {
    match normalize_command_line_token(input).as_str() {
        "summary" => Some(ConfigDiagnosticsSection::Summary),
        "paths" | "path" => Some(ConfigDiagnosticsSection::Paths),
        "source" => Some(ConfigDiagnosticsSection::Source),
        "terminal" | "term" => Some(ConfigDiagnosticsSection::Terminal),
        "input" => Some(ConfigDiagnosticsSection::Input),
        "clipboard" | "clip" => Some(ConfigDiagnosticsSection::Clipboard),
        "limits" | "limit" => Some(ConfigDiagnosticsSection::Limits),
        "keymap" | "keys" => Some(ConfigDiagnosticsSection::Keymap),
        "filedialogkeymap" | "filedialogkeys" | "dialogkeymap" | "dialogkeys" => {
            Some(ConfigDiagnosticsSection::FileDialogKeymap)
        }
        _ => None,
    }
}

pub(crate) fn command_output_summary_line(buffer: &TextBuffer) -> Option<usize> {
    (0..buffer.line_count()).find(|line_index| {
        buffer
            .line(*line_index)
            .is_some_and(|line| line.starts_with("Command: "))
    })
}

pub(crate) fn command_output_index_line(buffer: &TextBuffer) -> Option<usize> {
    command_output_section_line(buffer, "Index")
}

pub(crate) fn command_output_status_line(buffer: &TextBuffer) -> Option<usize> {
    command_output_section_line(buffer, "Status: ")
}

pub(crate) fn command_output_truncated_line(buffer: &TextBuffer) -> Option<usize> {
    command_output_section_line(buffer, "Truncated: ")
}

pub(crate) fn command_output_stdout_line(buffer: &TextBuffer) -> Option<usize> {
    command_output_section_line(buffer, "--- stdout")
}

pub(crate) fn command_output_stdout_body_line(buffer: &TextBuffer) -> Option<usize> {
    command_output_body_line(buffer, command_output_stdout_line)
}

pub(crate) fn command_output_stderr_line(buffer: &TextBuffer) -> Option<usize> {
    command_output_section_line(buffer, "--- stderr")
}

pub(crate) fn command_output_stderr_body_line(buffer: &TextBuffer) -> Option<usize> {
    command_output_body_line(buffer, command_output_stderr_line)
}

pub(crate) fn command_output_section_line(buffer: &TextBuffer, prefix: &str) -> Option<usize> {
    (0..buffer.line_count()).find(|line_index| {
        buffer
            .line(*line_index)
            .is_some_and(|line| line.starts_with(prefix))
    })
}

pub(crate) fn command_output_section_view_text(
    buffer: &TextBuffer,
    section: CommandOutputSection,
) -> Option<String> {
    let header = match section {
        CommandOutputSection::Stdout => command_output_stdout_line(buffer)?,
        CommandOutputSection::Stderr => command_output_stderr_line(buffer)?,
    };
    let end = ((header + 1)..buffer.line_count())
        .find(|line_index| {
            buffer
                .line(*line_index)
                .is_some_and(|line| line.starts_with("--- "))
        })
        .unwrap_or(buffer.line_count());
    let body_line_count = end.saturating_sub(header + 1);
    let mut out = format!("Dun Command Output {}\n\n", section.label());
    out.push_str(&format!("Section: {}\n", section.label()));
    out.push_str(&format!("Lines: {body_line_count}\n\n"));
    for line_index in header..buffer.line_count() {
        let line = buffer.line(line_index).unwrap_or_default();
        if line_index > header && line.starts_with("--- ") {
            break;
        }
        out.push_str(line);
        out.push('\n');
    }
    Some(out)
}

pub(crate) fn command_output_relative_section_line(
    buffer: &TextBuffer,
    current_line: usize,
    direction: SearchDirection,
) -> Option<(usize, &'static str)> {
    let mut sections = Vec::new();
    if let Some(line) = command_output_summary_line(buffer) {
        sections.push((line, "summary"));
    }
    if let Some(line) = command_output_index_line(buffer) {
        sections.push((line, "index"));
    }
    if let Some(line) = command_output_stdout_line(buffer) {
        sections.push((line, "stdout"));
    }
    if let Some(line) = command_output_stderr_line(buffer) {
        sections.push((line, "stderr"));
    }
    sections.sort_by_key(|(line, _)| *line);
    sections.dedup_by_key(|(line, _)| *line);
    match direction {
        SearchDirection::Forward => sections
            .iter()
            .find(|(line, _)| *line > current_line)
            .or_else(|| sections.first())
            .copied(),
        SearchDirection::Backward => sections
            .iter()
            .rev()
            .find(|(line, _)| *line < current_line)
            .or_else(|| sections.last())
            .copied(),
    }
}

pub(crate) fn line_with_exact_text(buffer: &TextBuffer, text: &str) -> Option<usize> {
    (0..buffer.line_count()).find(|line_index| buffer.line(*line_index) == Some(text))
}

pub(crate) fn command_output_body_line(
    buffer: &TextBuffer,
    header_finder: fn(&TextBuffer) -> Option<usize>,
) -> Option<usize> {
    let header = header_finder(buffer)?;
    for line_index in header.saturating_add(1)..buffer.line_count() {
        let Some(line) = buffer.line(line_index) else {
            continue;
        };
        if line.starts_with("--- ") {
            return None;
        }
        let trimmed = line.trim();
        if !trimmed.is_empty() && trimmed != "(empty)" && trimmed != "[truncated]" {
            return Some(line_index);
        }
    }
    None
}
