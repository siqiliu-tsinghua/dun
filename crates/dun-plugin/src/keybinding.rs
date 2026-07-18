//! Typed contribution model and validator for the `keybinding` capability.
//!
//! A host reserves one leader prefix key and binds chords beneath it (Emacs
//! `C-x` style; docs/plugin-protocol.md). This module validates structure only
//! — bounded counts, no control/space characters, distinct chord keys. The
//! leader and chord *keys* are opaque strings here: `dun-plugin` has no key
//! model, so `dun` parses them into real keystrokes and checks the leader for
//! collisions against the live keymap.

use crate::json::Json;

pub const KEYBINDING_MAX_CHORDS: usize = 32;
pub const KEYBINDING_MAX_KEY_CHARS: usize = 32;
pub const KEYBINDING_MAX_ACTION_ID_CHARS: usize = 64;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PluginChord {
    /// A keystroke spec string (e.g. `"f"`, `"Ctrl+F"`), parsed by `dun`.
    pub key: String,
    pub action_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PluginKeybinding {
    /// The leader prefix keystroke spec string, parsed by `dun`.
    pub leader: String,
    pub chords: Vec<PluginChord>,
}

impl PluginKeybinding {
    pub fn from_payload(payload: &Json) -> Result<Self, &'static str> {
        let leader = payload
            .get("leader")
            .ok_or("keybinding payload has no leader")?
            .as_str()
            .ok_or("keybinding leader is not a string")?;
        if !is_valid_token(leader, KEYBINDING_MAX_KEY_CHARS) {
            return Err("keybinding leader is empty, too long, or has a non-graphic character");
        }

        let chords = payload
            .get("chords")
            .ok_or("keybinding payload has no chords")?
            .as_arr()
            .ok_or("keybinding chords is not an array")?;
        if chords.is_empty() {
            return Err("keybinding chord list is empty");
        }
        if chords.len() > KEYBINDING_MAX_CHORDS {
            return Err("keybinding chord count exceeds limit");
        }

        let mut validated: Vec<PluginChord> = Vec::with_capacity(chords.len());
        for chord in chords {
            if !matches!(chord, Json::Obj(_)) {
                return Err("keybinding chord is not an object");
            }
            let key = chord
                .get("key")
                .ok_or("keybinding chord has no key")?
                .as_str()
                .ok_or("keybinding chord key is not a string")?;
            if !is_valid_token(key, KEYBINDING_MAX_KEY_CHARS) {
                return Err(
                    "keybinding chord key is empty, too long, or has a non-graphic character",
                );
            }
            let action_id = chord
                .get("action_id")
                .ok_or("keybinding chord has no action_id")?
                .as_str()
                .ok_or("keybinding action_id is not a string")?;
            if !is_valid_token(action_id, KEYBINDING_MAX_ACTION_ID_CHARS) {
                return Err(
                    "keybinding action_id is empty, too long, or has a non-graphic character",
                );
            }
            if validated.iter().any(|existing| existing.key == key) {
                return Err("keybinding chord key is duplicated");
            }
            validated.push(PluginChord {
                key: key.to_string(),
                action_id: action_id.to_string(),
            });
        }

        Ok(Self {
            leader: leader.to_string(),
            chords: validated,
        })
    }
}

/// A keystroke spec or action id: non-empty, bounded, and free of control,
/// space, and other non-graphic characters (real keystroke specs are ASCII
/// graphic — `"Ctrl+X"`, `"f"`). The full `DisplaySanitizer` never sees these:
/// they are parsed, not rendered.
fn is_valid_token(token: &str, max_chars: usize) -> bool {
    !token.is_empty()
        && token.chars().count() <= max_chars
        && token.chars().all(|ch| ch.is_ascii_graphic())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::json;

    fn chord(key: &str, action_id: &str) -> Json {
        json::obj([("key", json::str(key)), ("action_id", json::str(action_id))])
    }

    fn payload(leader: &str, chords: Vec<Json>) -> Json {
        json::obj([("leader", json::str(leader)), ("chords", Json::Arr(chords))])
    }

    fn error(payload: &Json) -> &'static str {
        PluginKeybinding::from_payload(payload).expect_err("payload must be rejected")
    }

    #[test]
    fn accepts_a_leader_with_chords() {
        let payload = payload("Ctrl+J", vec![chord("f", "filter"), chord("c", "clear")]);
        let binding = PluginKeybinding::from_payload(&payload).expect("valid keybinding");
        assert_eq!(binding.leader, "Ctrl+J");
        assert_eq!(binding.chords.len(), 2);
        assert_eq!(binding.chords[1].key, "c");
        assert_eq!(binding.chords[1].action_id, "clear");
    }

    #[test]
    fn rejects_missing_leader() {
        let payload = json::obj([("chords", Json::Arr(vec![chord("f", "filter")]))]);
        assert_eq!(error(&payload), "keybinding payload has no leader");
    }

    #[test]
    fn rejects_leader_with_space() {
        let payload = payload("Ctrl X", vec![chord("f", "filter")]);
        assert_eq!(
            error(&payload),
            "keybinding leader is empty, too long, or has a non-graphic character"
        );
    }

    #[test]
    fn rejects_empty_chord_list() {
        let payload = payload("Ctrl+J", Vec::new());
        assert_eq!(error(&payload), "keybinding chord list is empty");
    }

    #[test]
    fn rejects_more_than_maximum_chords() {
        let chords = vec![chord("f", "filter"); KEYBINDING_MAX_CHORDS + 1];
        let payload = payload("Ctrl+J", chords);
        assert_eq!(error(&payload), "keybinding chord count exceeds limit");
    }

    #[test]
    fn rejects_chord_missing_action_id() {
        let payload = payload("Ctrl+J", vec![json::obj([("key", json::str("f"))])]);
        assert_eq!(error(&payload), "keybinding chord has no action_id");
    }

    #[test]
    fn rejects_chord_key_with_control_character() {
        let payload = payload("Ctrl+J", vec![chord("f\n", "filter")]);
        assert_eq!(
            error(&payload),
            "keybinding chord key is empty, too long, or has a non-graphic character"
        );
    }

    #[test]
    fn rejects_duplicate_chord_key() {
        let payload = payload("Ctrl+J", vec![chord("f", "filter"), chord("f", "find")]);
        assert_eq!(error(&payload), "keybinding chord key is duplicated");
    }
}
