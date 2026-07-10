use crate::*;

pub(crate) const COMMAND_LINE_HELP: &str = "Commands: help, results [N], config [section], status, reload-config, reloadfile, shell, run [\"command\"], theme [name], open [path], save [path], save-as [path], find [query], replace QUERY TEXT, replace all QUERY TEXT, goto LINE, or any command id such as edit.scroll_right";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CommandLineParseError {
    TrailingEscape,
    UnclosedQuote,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum CommandCompletion {
    None,
    Unique(String),
    CommonPrefix(String, usize),
    Candidates(Vec<CommandCompletionCandidate>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CommandCompletionCandidate {
    pub(crate) display: String,
    pub(crate) replacement: String,
}

pub(crate) fn command_line_completion(input: &str) -> CommandCompletion {
    let trailing_space = input.chars().last().is_some_and(char::is_whitespace);
    let tokens = match parse_command_line(input) {
        Ok(tokens) => tokens,
        Err(_) => return CommandCompletion::None,
    };
    let mut tokens = tokens;
    if trailing_space {
        tokens.push(String::new());
    }

    match tokens.as_slice() {
        [] => complete_last_token(input, "", command_line_top_level_candidates(), true),
        [partial] => complete_last_token("", partial, command_line_top_level_candidates(), true),
        [command, partial] if command_accepts_path_argument(command) => {
            complete_path_token(&format!("{command} "), partial)
        }
        [command, partial] => {
            let candidates = match normalize_command_line_token(command).as_str() {
                "config" | "diagnostics" | "configdiagnostics" => {
                    config_diagnostics_section_candidates()
                }
                "theme" => theme_command_candidates(),
                _ => return CommandCompletion::None,
            };
            let prefix = format!("{command} ");
            complete_last_token(&prefix, partial, candidates, false)
        }
        _ => CommandCompletion::None,
    }
}

fn complete_last_token(
    prefix: &str,
    partial: &str,
    candidates: &[&'static str],
    add_space_after_unique: bool,
) -> CommandCompletion {
    let normalized_partial = normalize_command_line_token(partial);
    let matches = candidates
        .iter()
        .copied()
        .filter(|candidate| {
            normalize_command_line_token(candidate).starts_with(&normalized_partial)
        })
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [] => CommandCompletion::None,
        [candidate] => {
            let mut text = format!("{prefix}{candidate}");
            if add_space_after_unique {
                text.push(' ');
            }
            CommandCompletion::Unique(text)
        }
        _ => {
            let common = common_candidate_prefix(&matches);
            if common.len() > partial.len() {
                CommandCompletion::CommonPrefix(format!("{prefix}{common}"), matches.len())
            } else {
                CommandCompletion::Candidates(
                    matches
                        .iter()
                        .map(|candidate| {
                            let mut replacement = format!("{prefix}{candidate}");
                            if add_space_after_unique {
                                replacement.push(' ');
                            }
                            CommandCompletionCandidate {
                                display: candidate.to_string(),
                                replacement,
                            }
                        })
                        .collect(),
                )
            }
        }
    }
}

fn command_accepts_path_argument(command: &str) -> bool {
    matches!(
        normalize_command_line_token(command).as_str(),
        "open" | "save" | "saveas" | "reloadfile"
    )
}

fn complete_path_token(prefix: &str, partial: &str) -> CommandCompletion {
    let context = file_dialog_context(partial);
    let Ok(listing) = list_file_dialog_entries(&context, false) else {
        return CommandCompletion::None;
    };
    let entries = listing
        .entries
        .into_iter()
        .filter(|entry| is_completable_file_dialog_entry(entry, &context.prefix))
        .collect::<Vec<_>>();
    match entries.as_slice() {
        [] => CommandCompletion::None,
        [entry] => CommandCompletion::Unique(format!(
            "{prefix}{}",
            quote_command_line_token(&entry.input)
        )),
        _ => {
            if let Some(common) = common_entry_prefix(&entries, &context.prefix) {
                if common.len() > context.prefix.len() {
                    return CommandCompletion::CommonPrefix(
                        format!(
                            "{prefix}{}",
                            quote_command_line_token(&format!("{}{}", context.base_input, common))
                        ),
                        entries.len(),
                    );
                }
            }

            CommandCompletion::Candidates(
                entries
                    .iter()
                    .map(|entry| CommandCompletionCandidate {
                        display: if entry.is_dir {
                            format!("{}/", entry.name)
                        } else {
                            entry.name.clone()
                        },
                        replacement: format!("{prefix}{}", quote_command_line_token(&entry.input)),
                    })
                    .collect(),
            )
        }
    }
}

fn quote_command_line_token(token: &str) -> String {
    if token.is_empty() {
        return "\"\"".to_string();
    }
    if !token
        .chars()
        .any(|ch| ch.is_whitespace() || matches!(ch, '"' | '\\'))
    {
        return token.to_string();
    }

    let mut quoted = String::from("\"");
    for ch in token.chars() {
        if matches!(ch, '"' | '\\') {
            quoted.push('\\');
        }
        quoted.push(ch);
    }
    quoted.push('"');
    quoted
}

fn common_candidate_prefix(candidates: &[&str]) -> String {
    let Some(first) = candidates.first() else {
        return String::new();
    };
    let mut prefix = (*first).to_string();
    for candidate in &candidates[1..] {
        while !candidate.starts_with(&prefix) {
            let Some((last, _)) = prefix.char_indices().last() else {
                return String::new();
            };
            prefix.truncate(last);
        }
    }
    prefix
}

const fn command_line_top_level_candidates() -> &'static [&'static str] {
    &[
        "buffers",
        "close",
        "commands",
        "config",
        "diagnostics",
        "find",
        "goto",
        "help",
        "mark",
        "matches",
        "new",
        "open",
        "quit",
        "reload-config",
        "reloadfile",
        "replace",
        "results",
        "save",
        "save-as",
        "shell",
        "status",
        "theme",
        "whitespace",
        "wrap",
    ]
}

const fn config_diagnostics_section_candidates() -> &'static [&'static str] {
    &[
        "clipboard",
        "file-dialog-keymap",
        "input",
        "keymap",
        "limits",
        "paths",
        "source",
        "summary",
        "terminal",
    ]
}

const fn theme_command_candidates() -> &'static [&'static str] {
    &["dark", "dun", "msedit", "turbo"]
}

pub(crate) fn parse_command_line(input: &str) -> Result<Vec<String>, CommandLineParseError> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut quote = None;
    let mut escaped = false;
    let mut token_started = false;

    for ch in input.chars() {
        if escaped {
            current.push(ch);
            escaped = false;
            continue;
        }

        if ch == '\\' {
            escaped = true;
            token_started = true;
            continue;
        }

        if let Some(quote_char) = quote {
            if ch == quote_char {
                quote = None;
            } else {
                current.push(ch);
            }
            continue;
        }

        match ch {
            '\'' | '"' => {
                quote = Some(ch);
                token_started = true;
            }
            ch if ch.is_whitespace() => {
                if token_started {
                    tokens.push(std::mem::take(&mut current));
                    token_started = false;
                }
            }
            _ => {
                current.push(ch);
                token_started = true;
            }
        }
    }

    if escaped {
        return Err(CommandLineParseError::TrailingEscape);
    }
    if quote.is_some() {
        return Err(CommandLineParseError::UnclosedQuote);
    }
    if token_started {
        tokens.push(current);
    }

    Ok(tokens)
}

pub(crate) fn command_line_parse_error_text(error: CommandLineParseError) -> &'static str {
    match error {
        CommandLineParseError::TrailingEscape => "trailing escape",
        CommandLineParseError::UnclosedQuote => "unclosed quote",
    }
}

pub(crate) fn normalize_command_line_token(input: &str) -> String {
    input
        .trim()
        .chars()
        .filter(|ch| *ch != '-' && *ch != '_' && !ch.is_whitespace())
        .flat_map(char::to_lowercase)
        .collect()
}

pub(crate) fn parse_theme_command_value(input: &str) -> Option<ThemeName> {
    match normalize_command_line_token(input).as_str() {
        "msedit" | "microsoftedit" => Some(ThemeName::MsEdit),
        "turbo" | "turbovision" => Some(ThemeName::Turbo),
        "dark" => Some(ThemeName::Dark),
        "dun" => Some(ThemeName::Dun),
        _ => None,
    }
}

pub(crate) const fn theme_command_values() -> &'static str {
    "msedit|turbo|dark|dun"
}

pub(crate) const fn config_diagnostics_section_values() -> &'static str {
    "summary|paths|source|terminal|input|clipboard|limits|keymap|file-dialog-keymap"
}
