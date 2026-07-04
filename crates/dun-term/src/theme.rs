#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThemeName {
    MsEdit,
    Turbo,
    Dark,
    Dun,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Theme {
    pub name: &'static str,
    pub theme: ThemeName,
}

impl Theme {
    pub const fn msedit() -> Self {
        Self {
            name: "msedit",
            theme: ThemeName::MsEdit,
        }
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::msedit()
    }
}
