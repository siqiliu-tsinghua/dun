#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct BufferId(pub u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BufferKind {
    Untitled,
    File,
    ReadOnly,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum LineEnding {
    #[default]
    Lf,
    CrLf,
}

impl LineEnding {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Lf => "\n",
            Self::CrLf => "\r\n",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Position {
    pub line: usize,
    pub column: usize,
}

impl Position {
    pub const fn new(line: usize, column: usize) -> Self {
        Self { line, column }
    }

    pub const fn zero() -> Self {
        Self { line: 0, column: 0 }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Cursor {
    pub position: Position,
    pub preferred_column: usize,
}

impl Cursor {
    pub const fn new(position: Position) -> Self {
        Self {
            position,
            preferred_column: position.column,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct TextRange {
    pub start: Position,
    pub end: Position,
}

impl TextRange {
    pub fn new(start: Position, end: Position) -> Self {
        Self { start, end }.normalized()
    }

    pub const fn empty(at: Position) -> Self {
        Self { start: at, end: at }
    }

    pub fn normalized(self) -> Self {
        if self.start <= self.end {
            self
        } else {
            Self {
                start: self.end,
                end: self.start,
            }
        }
    }

    pub fn is_empty(self) -> bool {
        self.start == self.end
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Selection {
    pub anchor: Position,
    pub cursor: Position,
}

impl Selection {
    pub const fn new(anchor: Position, cursor: Position) -> Self {
        Self { anchor, cursor }
    }

    pub fn range(self) -> TextRange {
        TextRange::new(self.anchor, self.cursor)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EditTransaction {
    pub edits: Vec<TextEdit>,
    pub before_cursor: Position,
    pub after_cursor: Position,
    pub before_selection: Option<Selection>,
    pub after_selection: Option<Selection>,
    pub merge_kind: EditMergeKind,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TextEdit {
    Replace {
        range: TextRange,
        old_text: String,
        new_text: String,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EditMergeKind {
    None,
    InsertRun,
    DeleteBackwardRun,
    DeleteForwardRun,
}

pub(super) struct MergeEdit<'a> {
    pub merge_kind: EditMergeKind,
    pub range: TextRange,
    pub old_text: &'a str,
    pub new_text: &'a str,
    pub before_cursor: Position,
    pub after_cursor: Position,
    pub before_selection: Option<Selection>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BufferError {
    InvalidPosition(Position),
    InvalidRange(TextRange),
    ReadOnly,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SearchMatch {
    pub range: TextRange,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SearchOptions {
    pub case_sensitive: bool,
    pub whole_word: bool,
}

impl Default for SearchOptions {
    fn default() -> Self {
        Self {
            case_sensitive: true,
            whole_word: false,
        }
    }
}

#[derive(Clone, Debug)]
pub struct TextBuffer {
    pub(super) kind: BufferKind,
    pub(super) lines: Vec<String>,
    pub(super) line_ending: LineEnding,
    pub(super) cursor: Cursor,
    pub(super) selection: Option<Selection>,
    pub(super) undo_stack: Vec<EditTransaction>,
    pub(super) redo_stack: Vec<EditTransaction>,
    pub(super) revision: u64,
    pub(super) saved_fingerprint: u64,
    // Lazily computed dirty state for the current revision; costs O(buffer)
    // to fill, so status rendering must not recompute it every frame.
    pub(super) dirty_cache: std::cell::Cell<Option<bool>>,
}

impl PartialEq for TextBuffer {
    fn eq(&self, other: &Self) -> bool {
        // dirty_cache is a memoization detail, not buffer state.
        self.kind == other.kind
            && self.lines == other.lines
            && self.line_ending == other.line_ending
            && self.cursor == other.cursor
            && self.selection == other.selection
            && self.undo_stack == other.undo_stack
            && self.redo_stack == other.redo_stack
            && self.revision == other.revision
            && self.saved_fingerprint == other.saved_fingerprint
    }
}

impl Eq for TextBuffer {}
