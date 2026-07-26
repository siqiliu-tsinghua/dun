#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EditorCommand {
    File(FileCommand),
    Edit(EditCommand),
    Window(WindowCommand),
    App(AppCommand),
    /// An action a plugin host contributed, invoked from its menu subtree (the
    /// `menu` capability) or a leader chord (the `keybinding` capability). It
    /// carries the owning host's `plugin_id`, the item's `action_id`, and the
    /// action `kind` so dispatch can route the invocation back to that host.
    /// Plugin actions are never *user*-bindable: `command_id` collapses every
    /// instance to one generic static id and there is no `command_from_id`
    /// round-trip, so a `PluginAction` never appears in a user keymap (a
    /// plugin's own leader chords are injected as a separate keymap, not parsed
    /// from config).
    PluginAction {
        plugin_id: String,
        action_id: String,
        kind: PluginActionKind,
    },
}

/// What an invoked plugin action does, declared by the host on each menu item
/// or leader chord. Defaults to `Surface` when the host omits it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum PluginActionKind {
    /// Open (or reuse) the plugin's read-only surface window and, if the host
    /// holds `surface-write`, fetch its content.
    #[default]
    Surface,
    /// Open (or reuse) the plugin's editable scratch window (`scratch-input`).
    Scratch,
    /// Submit the plugin's scratch buffer text to the host (`execute`).
    Execute,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FileCommand {
    New,
    Open,
    SwitchBuffer,
    Save,
    SaveAs,
    Reload,
    Close,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EditCommand {
    Undo,
    Redo,
    Cut,
    Copy,
    CopyExternal,
    Paste,
    SelectAll,
    SelectLine,
    CopyLine,
    DeleteLine,
    MoveLineUp,
    MoveLineDown,
    IndentLine,
    OutdentLine,
    TrimTrailingWhitespace,
    ToggleWordWrap,
    ToggleVisibleWhitespace,
    ToggleBookmark,
    NextBookmark,
    PreviousBookmark,
    MoveLeft,
    MoveRight,
    MoveUp,
    MoveDown,
    MovePageUp,
    MovePageDown,
    MoveDocumentStart,
    MoveDocumentEnd,
    ScrollLeft,
    ScrollRight,
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
    RunCommand,
    ConfigDiagnosticsClipboard,
    ConfigDiagnosticsFileDialogKeymap,
    ConfigDiagnosticsInput,
    ConfigDiagnosticsKeymap,
    ConfigDiagnosticsLimits,
    ConfigDiagnosticsPaths,
    ConfigDiagnosticsSource,
    ConfigDiagnosticsSummary,
    ConfigDiagnosticsTerminal,
    SearchResults,
    ShellEscape,
    StatusHistory,
    Quit,
}
