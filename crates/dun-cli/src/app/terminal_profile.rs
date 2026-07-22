use crate::*;

impl AppState {
    pub(crate) fn apply_ambiguous_width_probe(
        &mut self,
        ambiguous_width: AmbiguousWidth,
        terminal_overrides: TerminalOverrides,
    ) {
        self.detected_profile.ambiguous_width = ambiguous_width;
        self.shell.profile = terminal_overrides.apply_to(self.detected_profile);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn apply_with_override(
        detected: AmbiguousWidth,
        ambiguous_width: Option<AmbiguousWidth>,
    ) -> AppState {
        let mut app = AppState::new();
        let terminal_overrides = TerminalOverrides {
            ambiguous_width,
            ..TerminalOverrides::default()
        };

        app.apply_ambiguous_width_probe(detected, terminal_overrides);
        app
    }

    #[test]
    fn configured_narrow_wins_over_detected_wide() {
        let app = apply_with_override(AmbiguousWidth::Wide, Some(AmbiguousWidth::Narrow));

        assert_eq!(app.detected_profile.ambiguous_width, AmbiguousWidth::Wide);
        assert_eq!(app.shell.profile.ambiguous_width, AmbiguousWidth::Narrow);
    }

    #[test]
    fn configured_wide_wins_over_detected_narrow() {
        let app = apply_with_override(AmbiguousWidth::Narrow, Some(AmbiguousWidth::Wide));

        assert_eq!(app.detected_profile.ambiguous_width, AmbiguousWidth::Narrow);
        assert_eq!(app.shell.profile.ambiguous_width, AmbiguousWidth::Wide);
    }

    #[test]
    fn detected_wide_is_effective_without_an_override() {
        let app = apply_with_override(AmbiguousWidth::Wide, None);

        assert_eq!(app.detected_profile.ambiguous_width, AmbiguousWidth::Wide);
        assert_eq!(app.shell.profile.ambiguous_width, AmbiguousWidth::Wide);
    }

    #[test]
    fn detected_width_survives_config_reload() {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time should follow the Unix epoch")
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "dun-cli-ambiguous-width-reload-{}-{unique}",
            std::process::id()
        ));
        std::fs::write(&path, "terminal.ambiguous-width = narrow\n").expect("write initial config");
        let request = ConfigLoadRequest::explicit(path.clone());
        let loaded_config = load_config(&request).expect("load initial config");
        let terminal_overrides = loaded_config.config.terminal;
        let mut app = AppState::from_loaded_config(request, loaded_config);
        app.apply_ambiguous_width_probe(AmbiguousWidth::Wide, terminal_overrides);
        assert_eq!(app.shell.profile.ambiguous_width, AmbiguousWidth::Narrow);

        std::fs::write(&path, "theme = dun\n").expect("write reloaded config");
        app.reload_config();

        assert_eq!(app.detected_profile.ambiguous_width, AmbiguousWidth::Wide);
        assert_eq!(app.shell.profile.ambiguous_width, AmbiguousWidth::Wide);
        let _ = std::fs::remove_file(path);
    }
}
