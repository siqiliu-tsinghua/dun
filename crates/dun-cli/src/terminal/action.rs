#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum RuntimeAction {
    ShellEscape,
    WriteTerminal(String),
    QueryOsc52Clipboard { max_bytes: usize },
}
