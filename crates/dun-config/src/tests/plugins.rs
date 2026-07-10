use std::path::PathBuf;

use super::support::*;

const COMPLETE_PLUGIN: &str = "\
plugin.alpha.command = ./alpha-host
plugin.alpha.trust = user-trusted-external
plugin.alpha.roles = syntax-highlight
";

#[test]
fn parses_two_plugin_entries_with_defaults_and_overrides() {
    let config = parse_config(
        "\
plugin.highlighter.command = ./bin/highlighter
plugin.highlighter.trust = pure-sandbox
plugin.highlighter.roles = syntax-highlight, text-transform, config-helper
plugin.log-tools.command = /opt/dun/log-tools
plugin.log-tools.trust = user-trusted-external
plugin.log-tools.roles = log-filter
plugin.log-tools.timeout_ms = 5000
plugin.log-tools.max_frame_bytes = 512 KiB
",
    )
    .expect("plugin config parses");

    assert_eq!(config.plugins.len(), 2);
    assert_eq!(config.plugins[0].id, "highlighter");
    assert_eq!(
        config.plugins[0].command,
        PathBuf::from("./bin/highlighter")
    );
    assert_eq!(config.plugins[0].trust, PluginTrust::PureSandbox);
    assert_eq!(
        config.plugins[0].roles,
        vec![
            PluginRole::SyntaxHighlight,
            PluginRole::TextTransform,
            PluginRole::ConfigHelper
        ]
    );
    assert_eq!(config.plugins[0].timeout_ms, 2_000);
    assert_eq!(config.plugins[0].max_frame_bytes, 256 * 1024);

    assert_eq!(config.plugins[1].id, "log-tools");
    assert_eq!(
        config.plugins[1].command,
        PathBuf::from("/opt/dun/log-tools")
    );
    assert_eq!(config.plugins[1].trust, PluginTrust::UserTrustedExternal);
    assert_eq!(config.plugins[1].roles, vec![PluginRole::LogFilter]);
    assert_eq!(config.plugins[1].timeout_ms, 5_000);
    assert_eq!(config.plugins[1].max_frame_bytes, 512 * 1024);
}

#[test]
fn rejects_invalid_plugin_ids() {
    for input in [
        "plugin..command = host",
        "plugin.Upper.command = host",
        "plugin.has_underscore.command = host",
        "plugin.has.dot.command = host",
        "plugin.has/slash.command = host",
    ] {
        let error = parse_config(input).expect_err("invalid plugin id is rejected");

        assert_eq!(error.line, Some(1));
        assert!(error.message.contains("invalid plugin id"), "{error}");
        assert!(error.message.contains("[a-z0-9-]"), "{error}");
    }
}

#[test]
fn unknown_plugin_trust_names_allowed_values() {
    let error = parse_config(
        "\
plugin.alpha.command = ./alpha-host
plugin.alpha.trust = unsafe
plugin.alpha.roles = syntax-highlight
",
    )
    .expect_err("unknown trust is rejected");

    assert_eq!(error.line, Some(2));
    assert!(error.message.contains("pure-sandbox"));
    assert!(error.message.contains("user-trusted-external"));
}

#[test]
fn unknown_plugin_role_names_allowed_values() {
    let error = parse_config(
        "\
plugin.alpha.command = ./alpha-host
plugin.alpha.trust = pure-sandbox
plugin.alpha.roles = terminal-access
",
    )
    .expect_err("unknown role is rejected");

    assert_eq!(error.line, Some(3));
    for allowed in [
        "syntax-highlight",
        "log-filter",
        "text-transform",
        "config-helper",
    ] {
        assert!(error.message.contains(allowed), "{error}");
    }
}

#[test]
fn requires_command_trust_and_roles() {
    let cases = [
        (
            "\
plugin.alpha.trust = pure-sandbox
plugin.alpha.roles = syntax-highlight
",
            "command",
        ),
        (
            "\
plugin.alpha.command = ./alpha-host
plugin.alpha.roles = syntax-highlight
",
            "trust",
        ),
        (
            "\
plugin.alpha.command = ./alpha-host
plugin.alpha.trust = pure-sandbox
",
            "role",
        ),
    ];

    for (input, expected) in cases {
        let error = parse_config(input).expect_err("required field is validated");

        assert_eq!(error.line, None);
        assert!(error.message.contains("alpha"), "{error}");
        assert!(error.message.contains(expected), "{error}");
    }
}

#[test]
fn rejects_zero_plugin_timeout_and_frame_cap() {
    let timeout_error = parse_config(&format!("{COMPLETE_PLUGIN}plugin.alpha.timeout_ms = 0\n"))
        .expect_err("zero timeout is rejected");
    assert_eq!(timeout_error.line, None);
    assert!(timeout_error.message.contains("timeout"));
    assert!(timeout_error.message.contains("greater than zero"));

    let frame_error = parse_config(&format!(
        "{COMPLETE_PLUGIN}plugin.alpha.max_frame_bytes = 0\n"
    ))
    .expect_err("zero frame cap is rejected");
    assert_eq!(frame_error.line, None);
    assert!(frame_error.message.contains("frame byte limit"));
    assert!(frame_error.message.contains("greater than zero"));
}

#[test]
fn rejects_duplicate_plugin_role() {
    let error = parse_config(
        "\
plugin.alpha.command = ./alpha-host
plugin.alpha.trust = pure-sandbox
plugin.alpha.roles = syntax-highlight, syntax-highlight
",
    )
    .expect_err("duplicate role is rejected");

    assert_eq!(error.line, None);
    assert!(error.message.contains("duplicate role"));
    assert!(error.message.contains("syntax-highlight"));
}

#[test]
fn plugin_overlay_override_of_one_key_wins() {
    let base = parse_config(
        "\
plugin.alpha.command = ./alpha-host
plugin.alpha.trust = pure-sandbox
plugin.alpha.roles = syntax-highlight, text-transform
plugin.alpha.timeout_ms = 4000
plugin.alpha.max_frame_bytes = 512 KiB
",
    )
    .expect("base config parses");
    let expected = base.plugins[0].clone();

    let overlaid = parse_config_overlay(base, "plugin.alpha.timeout_ms = 9000")
        .expect("plugin overlay parses");

    assert_eq!(overlaid.plugins.len(), 1);
    assert_eq!(overlaid.plugins[0].timeout_ms, 9_000);
    assert_eq!(overlaid.plugins[0].id, expected.id);
    assert_eq!(overlaid.plugins[0].command, expected.command);
    assert_eq!(overlaid.plugins[0].trust, expected.trust);
    assert_eq!(overlaid.plugins[0].roles, expected.roles);
    assert_eq!(
        overlaid.plugins[0].max_frame_bytes,
        expected.max_frame_bytes
    );
}

#[test]
fn default_config_text_documents_commented_plugin_example() {
    let text = default_config_text();

    assert!(text.contains("# Plugin hosts"));
    for example in [
        "plugin.example.command = /path/to/plugin-host",
        "plugin.example.trust = user-trusted-external",
        "plugin.example.roles = syntax-highlight, log-filter",
        "plugin.example.timeout_ms = 2000",
        "plugin.example.max_frame_bytes = 256 KiB",
    ] {
        let line = text
            .lines()
            .find(|line| line.contains(example))
            .expect("plugin example is present");
        assert!(line.starts_with("# "), "example must be commented: {line}");
    }
}
