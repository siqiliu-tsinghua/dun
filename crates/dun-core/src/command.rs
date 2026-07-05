#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EditorCommand {
    File(FileCommand),
    Edit(EditCommand),
    Window(WindowCommand),
    App(AppCommand),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FileCommand {
    New,
    Open,
    Save,
    SaveAs,
    Close,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EditCommand {
    Undo,
    Redo,
    Cut,
    Copy,
    Paste,
    SelectAll,
    MoveLeft,
    MoveRight,
    MoveUp,
    MoveDown,
    MovePageUp,
    MovePageDown,
    MoveWordLeft,
    MoveWordRight,
    MoveLineStart,
    MoveLineEnd,
    ExtendSelectionPageUp,
    ExtendSelectionPageDown,
    ExtendSelectionWordLeft,
    ExtendSelectionWordRight,
    InsertNewline,
    DeleteBackward,
    DeleteForward,
    DeleteWordBackward,
    DeleteWordForward,
    Find,
    FindNext,
    FindPrevious,
    Replace,
    GoToLine,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WindowCommand {
    SplitHorizontal,
    SplitVertical,
    FocusLeft,
    FocusRight,
    FocusUp,
    FocusDown,
    ResizeLeft,
    ResizeRight,
    ResizeUp,
    ResizeDown,
    Equalize,
    RotateSplit,
    Collapse,
    Expand,
    ToggleCollapse,
    Close,
    Only,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AppCommand {
    CommandLine,
    ConfigDiagnostics,
    Help,
    ReloadConfig,
    StatusHistory,
    Quit,
}
