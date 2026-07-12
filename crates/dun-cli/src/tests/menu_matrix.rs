use super::snapshots::assert_snapshot;
use super::support::*;
use std::fmt::Write as _;

const WORKSPACE_AREA: Rect = Rect::new(0, 0, 80, 24);
const FIXTURE_TEXT: &str = "alpha\nbeta\ngamma";
const FIXTURE_FILE_NAME: &str = "menu-matrix.txt";

struct MenuMatrixFixture {
    directory: PathBuf,
    path: PathBuf,
}

impl MenuMatrixFixture {
    fn new() -> Self {
        let directory = temp_file_path("menu-matrix");
        std::fs::create_dir(&directory).unwrap();
        let path = directory.join(FIXTURE_FILE_NAME);
        Self { directory, path }
    }

    fn fresh_app(&self) -> AppState {
        std::fs::write(&self.path, FIXTURE_TEXT).unwrap();
        let mut app = AppState::from_path(Some(self.path.clone())).unwrap();
        app.status_message = None;
        app.sync_view_for_area(WORKSPACE_AREA);

        assert_eq!(app.workspace.window_count(), 1);
        assert_eq!(app.buffers.len(), 1);
        assert!(!app.buffers[0].buffer.is_dirty());
        assert_eq!(app.buffers[0].buffer.to_text(), FIXTURE_TEXT);
        assert_eq!(
            app.buffers[0]
                .path
                .as_deref()
                .and_then(Path::file_name)
                .and_then(|name| name.to_str()),
            Some(FIXTURE_FILE_NAME)
        );
        app
    }

    fn redact_path(&self, text: &str) -> String {
        text.replace(&self.directory.to_string_lossy().into_owned(), "<FIXTURE>")
    }
}

impl Drop for MenuMatrixFixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.directory);
    }
}

struct MenuSpec {
    label: String,
    mnemonic: char,
    entries: Vec<EntrySpec>,
}

struct EntrySpec {
    label: String,
    command: &'static str,
    mnemonic: char,
}

struct MatrixRow {
    menu: String,
    entry: String,
    command: &'static str,
    windows: String,
    buffers: String,
    modal: String,
    status: String,
    quit: &'static str,
    runtime: String,
}

impl MatrixRow {
    /// `status` goes last on purpose. Column widths are computed from the
    /// longest cell, so a status in the middle makes one reworded message
    /// reflow the whole table -- and the point of the matrix is that a
    /// behaviour change is a one-line diff.
    fn cells(&self) -> [&str; 9] {
        [
            &self.menu,
            &self.entry,
            self.command,
            &self.windows,
            &self.buffers,
            &self.modal,
            self.quit,
            &self.runtime,
            &self.status,
        ]
    }
}

fn menu_specs(app: &AppState) -> Vec<MenuSpec> {
    let buffer_views = app.buffer_views();
    let frame = app
        .shell
        .frame_for_workspace(&app.workspace, WORKSPACE_AREA, &buffer_views);

    (0..app.shell.menu_count())
        .map(|menu_index| {
            let menu = frame
                .menu
                .items
                .get(menu_index)
                .expect("an enumerated menu must have a rendered label");
            let entry_count = app
                .shell
                .menu_entry_count(menu_index)
                .expect("an enumerated menu must exist");
            let entries = (0..entry_count)
                .map(|entry_index| {
                    let entry = menu
                        .entries
                        .get(entry_index)
                        .expect("an enumerated entry must have a rendered label");
                    let command = app
                        .shell
                        .menu_entry_command(menu_index, entry_index)
                        .expect("an enumerated entry must have a command");
                    let mnemonic = app
                        .shell
                        .menu_entry_mnemonic(menu_index, entry_index)
                        .expect("an enumerated entry must have a mnemonic");
                    EntrySpec {
                        label: entry.label.to_string(),
                        command: command_id(&command),
                        mnemonic,
                    }
                })
                .collect();

            MenuSpec {
                label: menu.label.to_string(),
                mnemonic: menu
                    .label
                    .chars()
                    .next()
                    .expect("an enumerated menu must have a mnemonic"),
                entries,
            }
        })
        .collect()
}

fn key(ch: char, modifiers: CrosstermKeyModifiers) -> CrosstermKeyEvent {
    CrosstermKeyEvent::new(CrosstermKeyCode::Char(ch), modifiers)
}

fn drive_entry(fixture: &MenuMatrixFixture, menu: &MenuSpec, entry: &EntrySpec) -> MatrixRow {
    let mut app = fixture.fresh_app();
    handle_key_event(&mut app, key(menu.mnemonic, CrosstermKeyModifiers::ALT));
    handle_key_event(&mut app, key(entry.mnemonic, CrosstermKeyModifiers::NONE));

    let status = app
        .status_message
        .as_deref()
        .map(|message| format!("{:?}", fixture.redact_path(message)))
        .unwrap_or_else(|| "-".to_string());
    let runtime = app
        .take_runtime_action()
        .map(|action| format!("{action:?}"))
        .unwrap_or_else(|| "-".to_string());

    MatrixRow {
        menu: menu.label.clone(),
        entry: entry.label.clone(),
        command: entry.command,
        windows: app.workspace.window_count().to_string(),
        buffers: app.buffers.len().to_string(),
        modal: modal_name(&app),
        status,
        quit: if app.should_quit { "yes" } else { "-" },
        runtime,
    }
}

fn modal_name(app: &AppState) -> String {
    let mut open = Vec::new();
    if let Some(dialog) = &app.file_dialog {
        open.push(dialog.kind.name());
    }
    if let Some(prompt) = &app.prompt {
        open.push(prompt.kind.name());
    }
    if app.confirm.is_some() {
        open.push("Unsaved Changes");
    }
    if app.buffer_switcher.is_some() {
        open.push("Switch Buffer");
    }

    if open.is_empty() {
        "-".to_string()
    } else {
        open.join(" + ")
    }
}

fn format_table(rows: &[MatrixRow]) -> String {
    let headers = [
        "menu", "entry", "command", "win", "buf", "modal", "quit", "runtime", "status",
    ];
    let mut widths = headers.map(str::len);
    for row in rows {
        for (index, cell) in row.cells().iter().enumerate() {
            widths[index] = widths[index].max(cell.len());
        }
    }

    let mut table = String::new();
    write_table_line(&mut table, &headers, &widths);
    for row in rows {
        write_table_line(&mut table, &row.cells(), &widths);
    }
    table
}

fn write_table_line(table: &mut String, cells: &[&str; 9], widths: &[usize; 9]) {
    for (index, (&cell, &width)) in cells.iter().zip(widths).enumerate() {
        if index > 0 {
            table.push_str("  ");
        }
        if index == 3 || index == 4 {
            write!(table, "{cell:>width$}").unwrap();
        } else if index + 1 == cells.len() {
            table.push_str(cell);
        } else {
            write!(table, "{cell:<width$}").unwrap();
        }
    }
    table.push('\n');
}

/// Records the observable result of every menu mnemonic from an identical,
/// clean file-backed state. Counts and commands come from the menu hit API, so
/// adding an entry adds a row without maintaining a second command list.
#[test]
fn every_menu_entry_matches_the_behaviour_matrix() {
    let fixture = MenuMatrixFixture::new();
    let catalog_app = fixture.fresh_app();
    let menus = menu_specs(&catalog_app);
    let mut rows = Vec::new();

    for menu in &menus {
        for entry in &menu.entries {
            rows.push(drive_entry(&fixture, menu, entry));
        }
    }

    assert_snapshot("menu_matrix", &format_table(&rows));
}
