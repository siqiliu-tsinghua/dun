//! Resolving a host's `menu` contribution into a dun-ui menu item.
//!
//! Split out of `plugins.rs` when that file passed the 35k architecture-debt
//! threshold in docs/code-organization-guidelines.md. Behaviour-preserving:
//! the code moved verbatim, only its `use` list is new.

use dun_core::EditorCommand;
use dun_plugin::PluginMenu;
use dun_ui::{
    MenuEntry, MenuItem, compose_translated_menu_label, english_menu_mnemonic, menu_label_mnemonic,
};

use super::action_kind;

pub(super) fn resolve_plugin_menu(
    plugin_id: &str,
    menu: &PluginMenu,
    tags: &[String],
    mnemonic: char,
) -> MenuItem {
    // Entry mnemonics are author-chosen or absent — never derived. Duplicates
    // within one menu drop only the *later* entry's shortcut, not the entry
    // and not its siblings: a dropdown item stays reachable by arrows, Enter
    // and the mouse, so silently removing it would be a worse trade than
    // losing one letter. (A top-level collision is different and rejects the
    // whole subtree, because there the menu becomes unreachable entirely.)
    let mut claimed_entry_mnemonics: Vec<char> = Vec::new();
    let entries = menu
        .items
        .iter()
        .map(|item| {
            let base = item.label.resolve(tags);
            let label = match item.mnemonic {
                // Always composed, never conditionally: `entry_mnemonic` reads
                // ONLY a trailing `(M)` and has no first-character fallback,
                // so an entry whose suffix is omitted has no working key even
                // when its text starts with that very letter.
                Some(mnemonic) if !claimed_entry_mnemonics.contains(&mnemonic) => {
                    claimed_entry_mnemonics.push(mnemonic);
                    compose_translated_menu_label(base, mnemonic)
                }
                _ => base.to_string(),
            };
            MenuEntry::new(
                label,
                EditorCommand::PluginAction {
                    plugin_id: plugin_id.to_string(),
                    action_id: item.action_id.clone(),
                    kind: action_kind(item.kind),
                },
            )
        })
        .collect();
    let translation = menu.top_label.resolve_translation(tags);
    let base = translation.unwrap_or_else(|| menu.top_label.fallback());
    MenuItem::new(
        top_level_label(base, translation.is_some(), mnemonic),
        entries,
    )
}

/// Render a **top-level** label so it and the matcher agree about the mnemonic.
///
/// Two reasons to append `(M)`, and they are not the same reason:
///
/// - a translation is actively selected. Then the suffix goes on even when the
///   translated text happens to equal the English, because every other menu in
///   a translated UI carries one and a bare plugin label would read as having
///   no key at all;
/// - the rendered text would not resolve to this mnemonic anyway — a
///   translated label (`日志过滤`), or an author-declared letter that is not
///   the first one (`Log Filter` asking for `G`). Without the suffix the
///   declared key would simply not work, since the top-level matcher falls
///   back to the label's first character.
///
/// Plain English whose first letter already *is* the mnemonic gets nothing,
/// so `Log Filter` stays `Log Filter` exactly as `File` stays `File`.
fn top_level_label(base: &str, translated: bool, mnemonic: char) -> String {
    let already_matches =
        menu_label_mnemonic(base).is_some_and(|derived| derived.eq_ignore_ascii_case(&mnemonic));
    if translated || !already_matches {
        compose_translated_menu_label(base, mnemonic)
    } else {
        base.to_string()
    }
}

/// The top-level mnemonic: the host's choice if it declared one, else derived.
///
/// Unlike dropdown entries this one still derives when absent, because a
/// top-level menu without a mnemonic cannot be opened from the keyboard at
/// all. An author-declared mnemonic is taken as-is — it is already validated
/// as a single non-parenthesis ASCII graphic by the protocol layer — and only
/// the collision check in the caller can reject it.
pub(super) fn top_level_mnemonic(menu: &PluginMenu, english_label: &str) -> Option<char> {
    menu.top_mnemonic
        .or_else(|| valid_plugin_menu_mnemonic(english_label))
}

/// A plugin menu that declared no mnemonic falls back to its first English
/// ASCII letter. A raw English label may already carry a parenthesized
/// mnemonic; accept it only when that suffix agrees with the first-letter
/// rule, because dun-ui's matcher prefers it.
fn valid_plugin_menu_mnemonic(label: &str) -> Option<char> {
    let mnemonic = english_menu_mnemonic(label)?;
    if trailing_parenthesized_mnemonic(label)
        .is_some_and(|embedded| !embedded.eq_ignore_ascii_case(&mnemonic))
    {
        return None;
    }
    Some(mnemonic)
}

fn trailing_parenthesized_mnemonic(label: &str) -> Option<char> {
    let without_close = label.trim_end().strip_suffix(')')?;
    let (_, contents) = without_close.rsplit_once('(')?;
    let mut chars = contents.chars();
    let mnemonic = chars.next()?;
    chars.next().is_none().then_some(mnemonic)
}
