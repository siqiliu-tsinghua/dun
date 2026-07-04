#![forbid(unsafe_code)]

fn main() {
    let workspace = dun_core::Workspace::new_untitled();
    let profile = dun_term::TerminalProfile::default();
    let config = dun_config::Config::default();
    let ui = dun_ui::UiShell::from_config(&config, profile);

    println!(
        "dun baseline initialized: windows={}, theme={}, colors={:?}",
        workspace.window_count(),
        ui.theme.name,
        ui.profile.colors
    );
}
