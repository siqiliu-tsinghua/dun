#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EncodingProfile {
    Utf8,
    Ascii,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ColorProfile {
    Color256,
    Color16,
    Mono,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TerminalProfile {
    pub encoding: EncodingProfile,
    pub colors: ColorProfile,
}

impl Default for TerminalProfile {
    fn default() -> Self {
        Self {
            encoding: EncodingProfile::Utf8,
            colors: ColorProfile::Color256,
        }
    }
}
