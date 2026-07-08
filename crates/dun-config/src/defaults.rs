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

    out.push_str("\n# Mouse\n");
    out.push_str(&format!("mouse.enabled = {}\n", config.mouse.enabled));

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

    out
}
