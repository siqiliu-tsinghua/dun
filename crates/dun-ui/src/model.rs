use std::borrow::Cow;

use dun_core::{
    BufferId, EditorCommand, Position, Rect, SanitizedLine, SearchMatch, TextBuffer, WindowId,
};
use dun_term::BorderGlyphs;

#[derive(Clone, Copy, Debug)]
pub struct BufferView<'a> {
    pub id: BufferId,
    pub buffer: &'a TextBuffer,
    pub first_line: usize,
    pub first_visual_row: usize,
    pub first_column: usize,
    pub search_matches: &'a [SearchMatch],
    pub highlights: &'a [BufferHighlightSpan],
    pub active_search_match: Option<usize>,
    pub wrap: bool,
}

impl<'a> BufferView<'a> {
    pub const fn new(id: BufferId, buffer: &'a TextBuffer) -> Self {
        Self {
            id,
            buffer,
            first_line: 0,
            first_visual_row: 0,
            first_column: 0,
            search_matches: &[],
            highlights: &[],
            active_search_match: None,
            wrap: false,
        }
    }

    pub const fn scrolled(id: BufferId, buffer: &'a TextBuffer, first_line: usize) -> Self {
        Self {
            id,
            buffer,
            first_line,
            first_visual_row: 0,
            first_column: 0,
            search_matches: &[],
            highlights: &[],
            active_search_match: None,
            wrap: false,
        }
    }

    pub const fn scrolled_xy(
        id: BufferId,
        buffer: &'a TextBuffer,
        first_line: usize,
        first_column: usize,
    ) -> Self {
        Self {
            id,
            buffer,
            first_line,
            first_visual_row: 0,
            first_column,
            search_matches: &[],
            highlights: &[],
            active_search_match: None,
            wrap: false,
        }
    }

    pub const fn with_first_visual_row(mut self, first_visual_row: usize) -> Self {
        self.first_visual_row = first_visual_row;
        self
    }

    pub const fn with_search(
        mut self,
        search_matches: &'a [SearchMatch],
        active_search_match: Option<usize>,
    ) -> Self {
        self.search_matches = search_matches;
        self.active_search_match = active_search_match;
        self
    }

    pub fn with_wrap(mut self, wrap: bool) -> Self {
        self.wrap = wrap;
        self
    }

    pub fn with_highlight_spans(mut self, highlights: &'a [BufferHighlightSpan]) -> Self {
        self.highlights = highlights;
        self
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UiScrollbar {
    pub y: u16,
    pub height: u16,
}

/// Style class for plugin-provided highlight spans; the renderer maps each
/// class onto the theme's `syntax_*` palette slot.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HighlightClass {
    Keyword,
    Comment,
    StringLiteral,
    Number,
    Emphasis,
}

/// A validated highlight span in buffer coordinates (byte columns, like
/// selections and search matches). Produced by the plugin layer after
/// converting the protocol's character columns.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BufferHighlightSpan {
    pub line: usize,
    pub start_column: usize,
    pub end_column: usize,
    pub class: HighlightClass,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UiHighlightLine {
    pub y: u16,
    pub start_x: u16,
    pub end_x: u16,
    pub class: HighlightClass,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UiSearchMatchLine {
    pub y: u16,
    pub start_x: u16,
    pub end_x: u16,
    pub active: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UiHorizontalEdgeLine {
    pub y: u16,
    pub left: bool,
    pub right: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UiFrame {
    pub menu: MenuBar,
    pub status: StatusBar,
    pub windows: Vec<UiWindow>,
    pub overlay: Option<UiOverlay>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UiOverlay {
    pub title: String,
    pub lines: Vec<String>,
    pub input: Option<String>,
    pub cursor_column: Option<usize>,
    pub list: Vec<String>,
    pub selected_list_index: Option<usize>,
    pub list_has_more_above: bool,
    pub list_has_more_below: bool,
    pub buttons: Vec<String>,
    pub min_width: u16,
}

impl UiOverlay {
    pub fn prompt(
        title: impl Into<String>,
        input: impl Into<String>,
        cursor_column: usize,
    ) -> Self {
        Self {
            title: title.into(),
            lines: Vec::new(),
            input: Some(input.into()),
            cursor_column: Some(cursor_column),
            list: Vec::new(),
            selected_list_index: None,
            list_has_more_above: false,
            list_has_more_below: false,
            buttons: Vec::new(),
            min_width: 24,
        }
    }

    pub fn message(title: impl Into<String>, lines: Vec<String>, buttons: Vec<String>) -> Self {
        Self {
            title: title.into(),
            lines,
            input: None,
            cursor_column: None,
            list: Vec::new(),
            selected_list_index: None,
            list_has_more_above: false,
            list_has_more_below: false,
            buttons,
            min_width: 24,
        }
    }

    pub fn file_dialog(
        title: impl Into<String>,
        lines: Vec<String>,
        input: impl Into<String>,
        cursor_column: usize,
        list: Vec<String>,
        selected_list_index: Option<usize>,
        buttons: Vec<String>,
    ) -> Self {
        Self {
            title: title.into(),
            lines,
            input: Some(input.into()),
            cursor_column: Some(cursor_column),
            list,
            selected_list_index,
            list_has_more_above: false,
            list_has_more_below: false,
            buttons,
            min_width: 60,
        }
    }

    pub fn with_list(
        mut self,
        list: Vec<String>,
        selected_list_index: Option<usize>,
        min_width: u16,
    ) -> Self {
        self.list = list;
        self.selected_list_index = selected_list_index;
        self.min_width = min_width;
        self
    }

    pub fn with_list_overflow(mut self, has_more_above: bool, has_more_below: bool) -> Self {
        self.list_has_more_above = has_more_above;
        self.list_has_more_below = has_more_below;
        self
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UiMouseHit {
    pub window_id: WindowId,
    pub buffer_id: BufferId,
    pub target: UiMouseTarget,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UiMouseTarget {
    Chrome,
    Gutter,
    Scrollbar {
        first_line: usize,
        first_visual_row: usize,
    },
    Body(Position),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MenuBar {
    pub active: Option<MenuSelection>,
    pub items: Vec<MenuItem>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MenuSelection {
    pub menu_index: usize,
    pub entry_index: Option<usize>,
}

impl MenuSelection {
    pub const fn menu_only(menu_index: usize) -> Self {
        Self {
            menu_index,
            entry_index: None,
        }
    }

    pub const fn with_entry(menu_index: usize, entry_index: usize) -> Self {
        Self {
            menu_index,
            entry_index: Some(entry_index),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MenuItem {
    /// Borrowed for the built-in English labels, owned for loaded
    /// translations (docs/i18n.md).
    pub label: Cow<'static, str>,
    pub entries: Vec<MenuEntry>,
}

impl MenuItem {
    pub fn new(label: impl Into<Cow<'static, str>>, entries: Vec<MenuEntry>) -> Self {
        Self {
            label: label.into(),
            entries,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MenuEntry {
    pub label: Cow<'static, str>,
    pub command: EditorCommand,
}

impl MenuEntry {
    pub fn new(label: impl Into<Cow<'static, str>>, command: EditorCommand) -> Self {
        Self {
            label: label.into(),
            command,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PluginIndicator {
    pub text: String,
    pub alert: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StatusBar {
    pub left: String,
    pub right: String,
    pub plugin: Option<PluginIndicator>,
    pub focused_window: WindowId,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WindowGeometry {
    pub border_columns: u16,
    pub inner: Rect,
    pub gutter: Rect,
    pub body: Rect,
    pub right_border_x: u16,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UiWindow {
    pub id: WindowId,
    pub buffer_id: BufferId,
    pub title: String,
    pub rect: Rect,
    pub focused: bool,
    pub collapsed: bool,
    pub dirty: bool,
    pub read_only: bool,
    pub border: BorderGlyphs,
    pub geometry: WindowGeometry,
    pub gutter: Vec<UiGutterLine>,
    pub cursor: Option<UiCursor>,
    pub selection: Vec<UiSelectionLine>,
    pub search_matches: Vec<UiSearchMatchLine>,
    pub highlights: Vec<UiHighlightLine>,
    pub horizontal_edges: Vec<UiHorizontalEdgeLine>,
    pub scrollbar: Option<UiScrollbar>,
    pub body: Vec<SanitizedLine>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UiGutterLine {
    pub y: u16,
    pub label: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UiCursor {
    pub x: u16,
    pub y: u16,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UiSelectionLine {
    pub y: u16,
    pub start_x: u16,
    pub end_x: u16,
}
