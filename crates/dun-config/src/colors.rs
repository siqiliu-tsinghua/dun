use dun_term::{AnsiColor, PALETTE_ROLE_IDS, Palette, StyleAttrs, TerminalColor};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ColorOverrides {
    entries: Vec<RoleOverride>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct RoleOverride {
    role: &'static str,
    fg: Option<TerminalColor>,
    bg: Option<TerminalColor>,
    attrs: Option<StyleAttrs>,
}

impl ColorOverrides {
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn set_fg(&mut self, role: &'static str, color: TerminalColor) {
        self.entry_mut(role).fg = Some(color);
    }

    pub fn set_bg(&mut self, role: &'static str, color: TerminalColor) {
        self.entry_mut(role).bg = Some(color);
    }

    pub fn set_attrs(&mut self, role: &'static str, attrs: StyleAttrs) {
        self.entry_mut(role).attrs = Some(attrs);
    }

    pub fn apply_to(&self, palette: &mut Palette) {
        for entry in &self.entries {
            if let Some(style) = palette.role_mut(entry.role) {
                if let Some(fg) = entry.fg {
                    style.fg = fg;
                }
                if let Some(bg) = entry.bg {
                    style.bg = bg;
                }
                if let Some(attrs) = entry.attrs {
                    style.attrs = attrs;
                }
            }
        }
    }

    fn entry_mut(&mut self, role: &'static str) -> &mut RoleOverride {
        if let Some(index) = self.entries.iter().position(|entry| entry.role == role) {
            return &mut self.entries[index];
        }

        self.entries.push(RoleOverride {
            role,
            fg: None,
            bg: None,
            attrs: None,
        });
        self.entries
            .last_mut()
            .expect("color override was just inserted")
    }
}

pub(crate) fn canonical_role(id: &str) -> Option<&'static str> {
    PALETTE_ROLE_IDS.iter().copied().find(|role| *role == id)
}

pub(crate) fn parse_color(spec: &str) -> Option<TerminalColor> {
    let spec = spec.trim();
    if !spec.is_empty() && spec.bytes().all(|byte| byte.is_ascii_digit()) {
        return spec.parse::<u8>().ok().map(TerminalColor::Indexed);
    }

    let normalized = spec
        .chars()
        .filter(|ch| !matches!(ch, '-' | ' '))
        .flat_map(char::to_lowercase)
        .collect::<String>();

    Some(match normalized.as_str() {
        "default" => TerminalColor::Default,
        "black" => TerminalColor::Ansi(AnsiColor::Black),
        "red" => TerminalColor::Ansi(AnsiColor::Red),
        "green" => TerminalColor::Ansi(AnsiColor::Green),
        "yellow" => TerminalColor::Ansi(AnsiColor::Yellow),
        "blue" => TerminalColor::Ansi(AnsiColor::Blue),
        "magenta" => TerminalColor::Ansi(AnsiColor::Magenta),
        "cyan" => TerminalColor::Ansi(AnsiColor::Cyan),
        "white" => TerminalColor::Ansi(AnsiColor::White),
        "brightblack" | "bright_black" => TerminalColor::Ansi(AnsiColor::BrightBlack),
        "brightred" | "bright_red" => TerminalColor::Ansi(AnsiColor::BrightRed),
        "brightgreen" | "bright_green" => TerminalColor::Ansi(AnsiColor::BrightGreen),
        "brightyellow" | "bright_yellow" => TerminalColor::Ansi(AnsiColor::BrightYellow),
        "brightblue" | "bright_blue" => TerminalColor::Ansi(AnsiColor::BrightBlue),
        "brightmagenta" | "bright_magenta" => TerminalColor::Ansi(AnsiColor::BrightMagenta),
        "brightcyan" | "bright_cyan" => TerminalColor::Ansi(AnsiColor::BrightCyan),
        "brightwhite" | "bright_white" => TerminalColor::Ansi(AnsiColor::BrightWhite),
        _ => return None,
    })
}

pub(crate) fn format_color(color: TerminalColor) -> String {
    match color {
        TerminalColor::Default => "default".to_string(),
        TerminalColor::Indexed(index) => index.to_string(),
        TerminalColor::Ansi(color) => match color {
            AnsiColor::Black => "black",
            AnsiColor::Red => "red",
            AnsiColor::Green => "green",
            AnsiColor::Yellow => "yellow",
            AnsiColor::Blue => "blue",
            AnsiColor::Magenta => "magenta",
            AnsiColor::Cyan => "cyan",
            AnsiColor::White => "white",
            AnsiColor::BrightBlack => "bright_black",
            AnsiColor::BrightRed => "bright_red",
            AnsiColor::BrightGreen => "bright_green",
            AnsiColor::BrightYellow => "bright_yellow",
            AnsiColor::BrightBlue => "bright_blue",
            AnsiColor::BrightMagenta => "bright_magenta",
            AnsiColor::BrightCyan => "bright_cyan",
            AnsiColor::BrightWhite => "bright_white",
        }
        .to_string(),
    }
}

pub(crate) fn parse_attrs(spec: &str) -> Option<StyleAttrs> {
    let mut attrs = StyleAttrs::NONE;
    let mut saw_none = false;
    let mut saw_flag = false;

    for token in spec
        .split(|ch: char| ch == ',' || ch.is_whitespace())
        .filter(|token| !token.is_empty())
    {
        match token {
            "none" => {
                if saw_flag {
                    return None;
                }
                saw_none = true;
            }
            "bold" => {
                if saw_none {
                    return None;
                }
                saw_flag = true;
                attrs.bold = true;
            }
            "underline" => {
                if saw_none {
                    return None;
                }
                saw_flag = true;
                attrs.underline = true;
            }
            "reverse" => {
                if saw_none {
                    return None;
                }
                saw_flag = true;
                attrs.reverse = true;
            }
            _ => return None,
        }
    }

    Some(attrs)
}

pub(crate) fn format_attrs(attrs: StyleAttrs) -> String {
    let mut flags = Vec::new();
    if attrs.bold {
        flags.push("bold");
    }
    if attrs.underline {
        flags.push("underline");
    }
    if attrs.reverse {
        flags.push("reverse");
    }

    if flags.is_empty() {
        "none".to_string()
    } else {
        flags.join(", ")
    }
}
