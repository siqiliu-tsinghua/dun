use dun_term::{PALETTE_ROLE_IDS, StyleAttrs, TerminalProfile};

use crate::colors::{format_attrs, format_color};
use crate::{Config, command_id, file_dialog_action_id};

pub fn default_config_text() -> String {
    let config = Config::default();
    let mut out = String::from(
        "\
# Dun default configuration
# Copy to ~/.config/dun/config and edit as needed.

",
    );

    out.push_str("# Appearance\n");
    out.push_str(&format!("theme = {}\n", config.theme.as_str()));

    out.push_str("\n# Terminal fallback overrides\n");
    out.push_str("# terminal.encoding = utf8\n");
    out.push_str("# terminal.colors = 256\n");
    out.push_str("# terminal.ambiguous-width = narrow\n");

    out.push_str("\n# Mouse\n");
    out.push_str(&format!("mouse.enabled = {}\n", config.mouse.enabled));

    out.push_str("\n# Plugin status indicator\n");
    out.push_str(&format!(
        "plugins.status_bar = {}\n",
        config.plugin_status.status_bar
    ));
    out.push_str(&format!(
        "plugins.idle_after_ms = {}\n",
        config.plugin_status.idle_after_ms
    ));

    out.push_str("\n# Clipboard\n");
    out.push_str(&format!(
        "clipboard.osc52.enabled = {}\n",
        config.clipboard.osc52.enabled
    ));
    out.push_str(&format!(
        "clipboard.osc52.max_bytes = {}\n",
        config.clipboard.osc52.max_bytes
    ));

    out.push_str("\n# File and display limits\n");
    out.push_str(&format!(
        "limits.editable_file_soft_limit_bytes = {}\n",
        config.limits.editable_file_soft_limit_bytes
    ));
    out.push_str(&format!(
        "limits.line_display_soft_limit_bytes = {}\n",
        config.limits.line_display_soft_limit_bytes
    ));
    out.push_str(&format!(
        "limits.run_command_timeout_ms = {}\n",
        config.limits.run_command_timeout_ms
    ));

    out.push_str(
        "\n# Plugin hosts\n\
# plugin.example.command = /path/to/plugin-host\n\
# plugin.example.trust = user-trusted-external\n\
# plugin.example.roles = syntax-highlight, log-filter\n\
# plugin.example.timeout_ms = 2000\n\
# plugin.example.max_frame_bytes = 256 KiB\n",
    );

    out.push_str("\n# Global editor command keybindings\n");
    let mut keybindings = config
        .keybindings
        .bindings
        .iter()
        .map(|binding| (command_id(&binding.command), binding.sequence.to_string()))
        .collect::<Vec<_>>();
    keybindings.sort_by(|left, right| left.0.cmp(right.0));
    for (command, sequence) in keybindings {
        out.push_str(&format!("key.{command} = {sequence}\n"));
    }

    out.push_str("\n# Open/Save As modal keybindings\n");
    let mut file_dialog_bindings = config
        .file_dialog_keys
        .bindings
        .iter()
        .map(|binding| {
            (
                file_dialog_action_id(binding.action),
                binding.stroke.to_string(),
            )
        })
        .collect::<Vec<_>>();
    file_dialog_bindings.sort_by(|left, right| left.0.cmp(right.0));
    for (action, stroke) in file_dialog_bindings {
        out.push_str(&format!("key.{action} = {stroke}\n"));
    }

    let palette = config.resolved_theme(TerminalProfile::utf8_256()).palette;
    out.push_str(
        "\n# Color overrides (theme defaults shown; uncomment and edit)\n\
# Shorthand `color.<role> = <fg> / <bg>`, or granular `color.<role>.fg`,\n\
# `color.<role>.bg`, `color.<role>.attrs`. A color is a palette index 0-255,\n\
# an ANSI name (red, bright_blue, …), or `default`. Attrs is a comma list of\n\
# bold, underline, reverse, or none.\n",
    );
    for id in PALETTE_ROLE_IDS {
        let style = palette.role(id).expect("listed role resolves");
        let mut line = format!(
            "# color.{id} = {} / {}",
            format_color(style.fg),
            format_color(style.bg)
        );
        if style.attrs != StyleAttrs::NONE {
            line.push_str(&format!("  # attrs: {}", format_attrs(style.attrs)));
        }
        line.push('\n');
        out.push_str(&line);
    }

    out
}
