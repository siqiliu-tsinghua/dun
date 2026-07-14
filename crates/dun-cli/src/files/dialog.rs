use std::fs;
use std::path::Path;

use crate::*;

pub(crate) fn file_dialog_context(input: &str) -> FileDialogContext {
    let input = input.trim();
    if input.is_empty() {
        return FileDialogContext {
            base_input: String::new(),
            prefix: String::new(),
            directory: env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
        };
    }

    if let Some(index) = input.rfind('/') {
        let base_input = input[..=index].to_string();
        let prefix = input[index + 1..].to_string();
        let directory = directory_path_from_dialog_base(&base_input);
        return FileDialogContext {
            base_input,
            prefix,
            directory,
        };
    }

    FileDialogContext {
        base_input: String::new(),
        prefix: input.to_string(),
        directory: PathBuf::from("."),
    }
}

fn directory_path_from_dialog_base(base: &str) -> PathBuf {
    if base == "/" {
        return PathBuf::from("/");
    }

    let without_trailing = base.trim_end_matches('/');
    if without_trailing.is_empty() {
        PathBuf::from(".")
    } else {
        expand_user_path(without_trailing)
    }
}

fn parent_input_for_dialog_base(base: &str) -> String {
    if base.is_empty() {
        return "../".to_string();
    }
    if base == "/" {
        return "/".to_string();
    }

    let trimmed = base.trim_end_matches('/');
    if trimmed.is_empty() {
        return "/".to_string();
    }

    if let Some(index) = trimmed.rfind('/') {
        return trimmed[..=index].to_string();
    }

    String::new()
}

pub(crate) fn list_file_dialog_entries(
    context: &FileDialogContext,
    show_hidden: bool,
) -> io::Result<FileDialogListing> {
    let mut entries = Vec::new();
    let mut hidden_filtered = 0;
    let parent_input = parent_input_for_dialog_base(&context.base_input);
    entries.push(FileDialogEntry {
        name: "..".to_string(),
        input: parent_input.clone(),
        path: expand_user_path(&parent_input),
        is_dir: true,
        is_parent: true,
    });

    let include_hidden = show_hidden || context.prefix.starts_with('.');
    for entry in fs::read_dir(&context.directory)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with('.') && !include_hidden {
            hidden_filtered += 1;
            continue;
        }
        if !name.starts_with(&context.prefix) {
            continue;
        }

        let path = entry.path();
        let is_dir = path.is_dir();
        let input = format!(
            "{}{}{}",
            context.base_input,
            name,
            if is_dir { "/" } else { "" }
        );
        entries.push(FileDialogEntry {
            name,
            path: expand_user_path(&input),
            input,
            is_dir,
            is_parent: false,
        });
    }

    entries.sort_by(|left, right| {
        right
            .is_parent
            .cmp(&left.is_parent)
            .then_with(|| right.is_dir.cmp(&left.is_dir))
            .then_with(|| left.name.cmp(&right.name))
    });
    Ok(FileDialogListing {
        entries,
        hidden_filtered,
    })
}

pub(crate) fn file_dialog_list_message(
    context: &FileDialogContext,
    entries: &[FileDialogEntry],
    show_hidden: bool,
    hidden_filtered: usize,
) -> Option<FileDialogMessage> {
    let visible_entries = entries.iter().filter(|entry| !entry.is_parent).count();
    if visible_entries > 0 {
        return None;
    }

    if !context.prefix.is_empty() {
        if hidden_filtered > 0 && !show_hidden && !context.prefix.starts_with('.') {
            return Some(FileDialogMessage::NoVisibleMatches);
        }
        return Some(FileDialogMessage::NoMatchesForPrefix(
            context.prefix.clone(),
        ));
    }

    if hidden_filtered > 0 && !show_hidden {
        Some(FileDialogMessage::OnlyHiddenFiltered)
    } else {
        Some(FileDialogMessage::DirectoryEmpty)
    }
}

pub(crate) fn common_entry_prefix(
    entries: &[FileDialogEntry],
    current_prefix: &str,
) -> Option<String> {
    let mut iter = entries
        .iter()
        .filter(|entry| is_completable_file_dialog_entry(entry, current_prefix));
    let first = iter.next()?.name.clone();
    let mut prefix = first;

    for entry in iter {
        prefix = common_prefix(&prefix, &entry.name);
        if prefix.is_empty() {
            break;
        }
    }

    Some(prefix)
}

pub(crate) fn is_completable_file_dialog_entry(
    entry: &FileDialogEntry,
    current_prefix: &str,
) -> bool {
    !entry.is_parent || current_prefix.starts_with("..")
}

fn common_prefix(left: &str, right: &str) -> String {
    let mut end = 0;
    for ((left_index, left_char), (_, right_char)) in left.char_indices().zip(right.char_indices())
    {
        if left_char != right_char {
            break;
        }
        end = left_index + left_char.len_utf8();
    }

    left[..end].to_string()
}

pub(crate) fn previous_char_boundary(input: &str, index: usize) -> usize {
    input[..index]
        .char_indices()
        .last()
        .map(|(index, _)| index)
        .unwrap_or(0)
}

pub(crate) fn next_char_boundary(input: &str, index: usize) -> usize {
    input[index..]
        .chars()
        .next()
        .map(|ch| index + ch.len_utf8())
        .unwrap_or(input.len())
}

pub(crate) fn single_line_paste_text(input: &str) -> String {
    let mut output = String::new();
    let mut in_line_break = false;
    for ch in input.chars() {
        if matches!(ch, '\r' | '\n') {
            if !in_line_break {
                output.push(' ');
                in_line_break = true;
            }
        } else {
            output.push(ch);
            in_line_break = false;
        }
    }
    output
}

pub(crate) fn expand_user_path(input: &str) -> PathBuf {
    if input == "~" {
        return env::var_os("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(input));
    }

    if let Some(rest) = input.strip_prefix("~/") {
        if let Some(home) = env::var_os("HOME") {
            return PathBuf::from(home).join(rest);
        }
    }

    PathBuf::from(input)
}

pub(crate) fn ensure_trailing_separator(mut input: String) -> String {
    if !input.ends_with('/') {
        input.push('/');
    }
    input
}

pub(crate) fn file_dialog_recent_input_for_path(path: &Path) -> String {
    path.parent()
        .map(|parent| ensure_trailing_separator(parent.to_string_lossy().into_owned()))
        .unwrap_or_default()
}
