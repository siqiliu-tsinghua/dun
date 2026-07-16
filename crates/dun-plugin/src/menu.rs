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
    pub fn resolve(&self, active_tags: &[String]) -> &str {
        for tag in active_tags {
            if let Some(label) = self.get(tag) {
                return label;
            }
        }
        self.get(MENU_FALLBACK_TAG)
            .expect("validated label set has an en_US fallback")
    }

    fn get(&self, tag: &str) -> Option<&str> {
        self.0
            .iter()
            .find(|(candidate, _)| candidate == tag)
            .map(|(_, label)| label.as_str())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PluginMenuItem {
    pub label: LabelSet,
    pub action_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PluginMenu {
    pub top_label: LabelSet,
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
            validated.push(PluginMenuItem {
                label,
                action_id: action_id.to_string(),
            });
        }

        Ok(Self {
            top_label,
            items: validated,
        })
    }
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
