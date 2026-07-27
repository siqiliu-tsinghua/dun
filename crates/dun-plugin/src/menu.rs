//! Typed contribution model and validator for the `menu` capability.
//!
//! This mirrors `validate.rs`, the `overlay-write` capability's validator.
//! Labels are locale-tag maps with a required `en_US` fallback. This module
//! enforces structural bounds, including rejecting control characters; the
//! full `DisplaySanitizer` runs later at render time, not here.

use crate::json::Json;

pub const MENU_MAX_ITEMS: usize = 16;
pub const MENU_MAX_LABEL_CHARS: usize = 64;
pub const MENU_MAX_ACTION_ID_CHARS: usize = 64;
pub const MENU_FALLBACK_TAG: &str = "en_US";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LabelSet(Vec<(String, String)>);

impl LabelSet {
    /// The required English source label.
    ///
    /// Construction validates this entry; the empty fallback is a defensive
    /// degradation if that invariant is ever broken internally.
    pub fn fallback(&self) -> &str {
        self.get(MENU_FALLBACK_TAG).unwrap_or_default()
    }

    /// Resolve only an actively selected locale tag.
    ///
    /// `None` means the caller must use [`Self::fallback`]. Keeping that state
    /// separate lets menu composition distinguish a selected translation from
    /// English fallback even when both strings happen to be equal.
    pub fn resolve_translation(&self, active_tags: &[String]) -> Option<&str> {
        for tag in active_tags {
            if let Some(label) = self.get(tag) {
                return Some(label);
            }
        }
        None
    }

    /// Compatibility wrapper returning an active translation or `en_US`.
    pub fn resolve(&self, active_tags: &[String]) -> &str {
        self.resolve_translation(active_tags)
            .unwrap_or_else(|| self.fallback())
    }

    fn get(&self, tag: &str) -> Option<&str> {
        self.0
            .iter()
            .find(|(candidate, _)| candidate == tag)
            .map(|(_, label)| label.as_str())
    }
}

/// What an invoked plugin action does, declared by the host on each menu item
/// or leader chord via an optional `kind` field. Defaults to `Surface`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum PluginActionKind {
    #[default]
    Surface,
    Scratch,
    Execute,
}

impl PluginActionKind {
    /// Parse an optional `kind` field. Absent means the default (`Surface`); a
    /// present-but-unknown value is a validation error.
    pub(crate) fn from_field(value: Option<&Json>) -> Result<Self, &'static str> {
        match value {
            None => Ok(Self::Surface),
            Some(value) => match value.as_str() {
                Some("surface") => Ok(Self::Surface),
                Some("scratch") => Ok(Self::Scratch),
                Some("execute") => Ok(Self::Execute),
                _ => Err("action kind is not one of surface/scratch/execute"),
            },
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PluginMenuItem {
    pub label: LabelSet,
    /// Author-chosen keyboard mnemonic, language-independent.
    ///
    /// `dun` deliberately does not derive one for dropdown entries: no general
    /// rule survives contact with a real plugin (an IDE host's `Find
    /// References` and `Format Document` both start with `F`, and only its
    /// author knows which should own the key). Absent simply means the entry
    /// has no letter shortcut; arrows, Enter and the mouse still reach it.
    pub mnemonic: Option<char>,
    pub action_id: String,
    pub kind: PluginActionKind,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PluginMenu {
    pub top_label: LabelSet,
    /// Author-chosen mnemonic for the top-level entry. Unlike dropdown
    /// entries this one *does* fall back to a derivation (the `en_US` label's
    /// first ASCII letter), because a top-level menu with no mnemonic is
    /// unreachable from the keyboard entirely.
    pub top_mnemonic: Option<char>,
    pub items: Vec<PluginMenuItem>,
}

impl PluginMenu {
    pub fn from_payload(payload: &Json) -> Result<Self, &'static str> {
        let top_label = payload
            .get("top_label")
            .ok_or("menu payload has no top_label")?;
        let top_label = parse_label_set(top_label)?;

        let items = payload
            .get("items")
            .ok_or("menu payload has no items")?
            .as_arr()
            .ok_or("menu items is not an array")?;
        if items.is_empty() {
            return Err("menu item list is empty");
        }
        if items.len() > MENU_MAX_ITEMS {
            return Err("menu item count exceeds limit");
        }

        let mut validated = Vec::with_capacity(items.len());
        for item in items {
            if !matches!(item, Json::Obj(_)) {
                return Err("menu item is not an object");
            }
            let label = item.get("label").ok_or("menu item has no label")?;
            let label = parse_label_set(label)?;
            let action_id = item
                .get("action_id")
                .ok_or("menu item has no action_id")?
                .as_str()
                .ok_or("menu action_id is not a string")?;
            if action_id.is_empty() {
                return Err("menu action_id is empty");
            }
            if action_id.chars().count() > MENU_MAX_ACTION_ID_CHARS {
                return Err("menu action_id exceeds character limit");
            }
            if !action_id.chars().all(|ch| ch.is_ascii_graphic()) {
                return Err("menu action_id contains a non-graphic character");
            }
            let kind = PluginActionKind::from_field(item.get("kind"))?;
            let mnemonic = parse_mnemonic(item.get("mnemonic"))?;
            validated.push(PluginMenuItem {
                label,
                mnemonic,
                action_id: action_id.to_string(),
                kind,
            });
        }

        Ok(Self {
            top_label,
            top_mnemonic: parse_mnemonic(payload.get("top_mnemonic"))?,
            items: validated,
        })
    }
}

/// Parse an optional `mnemonic` field: exactly one ASCII graphic character.
///
/// The character set is deliberately as wide as `dun`'s own — built-in entries
/// use `.`, `[` and `]` as well as letters (`Visible Whitespace (.)`,
/// `Scroll Left ([)`), so restricting plugins to letters would make the two
/// sets inconsistent. Parentheses are the one exclusion: the rendered form is
/// `label (M)` and the matcher reads the *last* parenthesised group, so a
/// mnemonic of `(` or `)` would make the label parse ambiguously.
fn parse_mnemonic(value: Option<&Json>) -> Result<Option<char>, &'static str> {
    let Some(value) = value else {
        return Ok(None);
    };
    let text = value.as_str().ok_or("menu mnemonic is not a string")?;
    let mut chars = text.chars();
    let mnemonic = chars.next().ok_or("menu mnemonic is empty")?;
    if chars.next().is_some() {
        return Err("menu mnemonic is longer than one character");
    }
    if !mnemonic.is_ascii_graphic() {
        return Err("menu mnemonic is not an ASCII graphic character");
    }
    if mnemonic == '(' || mnemonic == ')' {
        return Err("menu mnemonic cannot be a parenthesis");
    }
    Ok(Some(mnemonic))
}

fn parse_label_set(value: &Json) -> Result<LabelSet, &'static str> {
    let Json::Obj(entries) = value else {
        return Err("menu label set is not an object");
    };
    let mut labels: Vec<(String, String)> = Vec::with_capacity(entries.len());
    for (tag, value) in entries {
        if tag.is_empty() {
            return Err("menu label tag is empty");
        }
        if labels.iter().any(|(existing, _)| existing == tag) {
            return Err("menu label tag is duplicated");
        }
        let label = value.as_str().ok_or("menu label is not a string")?;
        if label.is_empty() {
            return Err("menu label is empty");
        }
        if label.chars().count() > MENU_MAX_LABEL_CHARS {
            return Err("menu label exceeds character limit");
        }
        if label.chars().any(char::is_control) {
            return Err("menu label contains a control character");
        }
        labels.push((tag.clone(), label.to_string()));
    }
    if !labels.iter().any(|(tag, _)| tag == MENU_FALLBACK_TAG) {
        return Err("menu label set has no en_US fallback");
    }
    Ok(LabelSet(labels))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::json;

    fn bilingual_labels() -> Json {
        json::obj([("en_US", json::str("Tools")), ("zh-CN", json::str("工具"))])
    }

    fn english_label(text: &str) -> Json {
        json::obj([("en_US", json::str(text))])
    }

    fn item(label: Json, action_id: &str) -> Json {
        json::obj([("label", label), ("action_id", json::str(action_id))])
    }

    fn payload(top_label: Json, items: Vec<Json>) -> Json {
        json::obj([("top_label", top_label), ("items", Json::Arr(items))])
    }

    fn error(payload: &Json) -> &'static str {
        PluginMenu::from_payload(payload).expect_err("payload must be rejected")
    }

    #[test]
    fn accepts_valid_menu() {
        let payload = payload(
            bilingual_labels(),
            vec![
                item(english_label("Run"), "run"),
                item(english_label("Stop"), "stop"),
            ],
        );
        let menu = PluginMenu::from_payload(&payload).expect("valid menu");
        assert_eq!(menu.items.len(), 2);
        assert_eq!(menu.items[1].action_id, "stop");
        // An item with no `kind` field defaults to Surface.
        assert_eq!(menu.items[0].kind, PluginActionKind::Surface);
    }

    #[test]
    fn parses_and_rejects_the_action_kind_field() {
        let with_kind = json::obj([
            ("label", english_label("Run")),
            ("action_id", json::str("run")),
            ("kind", json::str("execute")),
        ]);
        let good = payload(bilingual_labels(), vec![with_kind]);
        let menu = PluginMenu::from_payload(&good).expect("valid menu");
        assert_eq!(menu.items[0].kind, PluginActionKind::Execute);

        let bad_kind = json::obj([
            ("label", english_label("Run")),
            ("action_id", json::str("run")),
            ("kind", json::str("delete-everything")),
        ]);
        let bad = payload(bilingual_labels(), vec![bad_kind]);
        assert_eq!(
            error(&bad),
            "action kind is not one of surface/scratch/execute"
        );
    }

    #[test]
    fn resolve_prefers_an_active_tag() {
        let labels = parse_label_set(&bilingual_labels()).expect("valid labels");
        assert_eq!(labels.resolve(&["zh-CN".into()]), "工具");
    }

    #[test]
    fn resolve_falls_back_to_en_us() {
        let labels = parse_label_set(&bilingual_labels()).expect("valid labels");
        assert_eq!(labels.resolve(&["fr".into()]), "Tools");
        assert_eq!(labels.resolve(&[]), "Tools");
    }

    #[test]
    fn translation_selection_is_distinct_when_text_equals_fallback() {
        let labels = parse_label_set(&json::obj([
            ("en_US", json::str("Tools")),
            ("fr", json::str("Tools")),
        ]))
        .expect("valid labels");

        assert_eq!(labels.fallback(), "Tools");
        assert_eq!(labels.resolve_translation(&["fr".into()]), Some("Tools"));
        assert_eq!(labels.resolve_translation(&["de".into()]), None);
    }

    #[test]
    fn rejects_top_label_missing_en_us() {
        let payload = payload(
            json::obj([("zh-CN", json::str("工具"))]),
            vec![item(english_label("Run"), "run")],
        );
        assert_eq!(error(&payload), "menu label set has no en_US fallback");
    }

    #[test]
    fn rejects_item_label_missing_en_us() {
        let payload = payload(
            bilingual_labels(),
            vec![item(json::obj([("zh-CN", json::str("运行"))]), "run")],
        );
        assert_eq!(error(&payload), "menu label set has no en_US fallback");
    }

    #[test]
    fn rejects_label_longer_than_limit() {
        let long_label = "x".repeat(MENU_MAX_LABEL_CHARS + 1);
        let payload = payload(
            english_label(&long_label),
            vec![item(english_label("Run"), "run")],
        );
        assert_eq!(error(&payload), "menu label exceeds character limit");
    }

    #[test]
    fn rejects_label_with_control_character() {
        let payload = payload(
            english_label("a\nb"),
            vec![item(english_label("Run"), "run")],
        );
        assert_eq!(error(&payload), "menu label contains a control character");
    }

    #[test]
    fn rejects_more_than_maximum_items() {
        let items = vec![item(english_label("Run"), "run"); MENU_MAX_ITEMS + 1];
        let payload = payload(bilingual_labels(), items);
        assert_eq!(error(&payload), "menu item count exceeds limit");
    }

    #[test]
    fn rejects_zero_items() {
        let payload = payload(bilingual_labels(), Vec::new());
        assert_eq!(error(&payload), "menu item list is empty");
    }

    #[test]
    fn rejects_empty_action_id() {
        let payload = payload(bilingual_labels(), vec![item(english_label("Run"), "")]);
        assert_eq!(error(&payload), "menu action_id is empty");
    }

    #[test]
    fn rejects_action_id_with_space() {
        let payload = payload(
            bilingual_labels(),
            vec![item(english_label("Run"), "run now")],
        );
        assert_eq!(
            error(&payload),
            "menu action_id contains a non-graphic character"
        );
    }

    #[test]
    fn rejects_action_id_longer_than_limit() {
        let action_id = "x".repeat(MENU_MAX_ACTION_ID_CHARS + 1);
        let payload = payload(
            bilingual_labels(),
            vec![item(english_label("Run"), &action_id)],
        );
        assert_eq!(error(&payload), "menu action_id exceeds character limit");
    }

    #[test]
    fn rejects_duplicate_label_tag() {
        let duplicate = Json::Obj(vec![
            (MENU_FALLBACK_TAG.to_string(), json::str("Tools")),
            (MENU_FALLBACK_TAG.to_string(), json::str("More tools")),
        ]);
        let payload = payload(duplicate, vec![item(english_label("Run"), "run")]);
        assert_eq!(error(&payload), "menu label tag is duplicated");
    }
}
