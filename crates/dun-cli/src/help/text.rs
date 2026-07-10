use crate::*;

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

pub(crate) fn line_with_exact_text(buffer: &TextBuffer, text: &str) -> Option<usize> {
    (0..buffer.line_count()).find(|line_index| buffer.line(*line_index) == Some(text))
}
