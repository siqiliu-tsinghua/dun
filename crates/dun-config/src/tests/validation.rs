use super::support::*;

#[test]
fn limits_reject_zero_values() {
    assert_eq!(
        Limits {
            line_display_soft_limit_bytes: 0,
            ..Limits::default()
        }
        .validate(),
        Err(LimitsError::LineDisplaySoftLimitZero)
    );
}

#[test]
fn validation_error_text_covers_config_error_variants() {
    assert_eq!(
        crate::validation::config_error_text(&ConfigError::Keymap(KeymapError::DuplicateBinding(
            KeySequence::from_str("Ctrl+S").unwrap()
        ))),
        "invalid keymap: duplicate key sequence `Ctrl+S`"
    );
    assert_eq!(
        crate::validation::config_error_text(&ConfigError::Keymap(KeymapError::EmptySequence)),
        "invalid keymap: empty key sequence"
    );
    assert_eq!(
        crate::validation::config_error_text(&ConfigError::FileDialogKeymap(
            FileDialogKeymapError::DuplicateBinding(KeyStroke::from_str("Enter").unwrap())
        )),
        "invalid file dialog keymap: duplicate key stroke `Enter`"
    );
    assert_eq!(
        crate::validation::config_error_text(&ConfigError::Limits(
            LimitsError::EditableFileSoftLimitZero
        )),
        "invalid limits: editable file soft limit must be greater than zero"
    );
    assert_eq!(
        crate::validation::config_error_text(&ConfigError::Limits(
            LimitsError::LineDisplaySoftLimitZero
        )),
        "invalid limits: line display soft limit must be greater than zero"
    );
}

#[test]
fn validation_error_text_covers_key_parse_error_variants() {
    let cases = [
        (KeyParseError::EmptySequence, "empty sequence"),
        (KeyParseError::EmptyStroke, "empty stroke"),
        (KeyParseError::MissingKey, "missing key"),
        (
            KeyParseError::DuplicateModifier("Ctrl".to_string()),
            "duplicate modifier `Ctrl`",
        ),
        (
            KeyParseError::UnknownModifier("Hyper".to_string()),
            "unknown modifier `Hyper`",
        ),
        (
            KeyParseError::UnknownKey("Foo".to_string()),
            "unknown key `Foo`",
        ),
        (
            KeyParseError::InvalidFunctionKey("F0".to_string()),
            "invalid function key `F0`",
        ),
    ];

    for (error, expected) in cases {
        assert_eq!(crate::validation::key_parse_error_text(&error), expected);
    }
}
