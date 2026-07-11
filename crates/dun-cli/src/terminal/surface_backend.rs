use std::io::{self, Write};

use crossterm::cursor;
use crossterm::terminal::{self, ClearType};
use dun_ui::{SurfaceRenderer, UiFrame, UiShell};

use super::TerminalWriter;

pub(crate) struct SurfaceBackend {
    writer: TerminalWriter,
    renderer: SurfaceRenderer,
}

impl SurfaceBackend {
    pub(crate) fn new(writer: TerminalWriter) -> Self {
        Self {
            writer,
            renderer: SurfaceRenderer::new(),
        }
    }

    pub(crate) fn draw(
        &mut self,
        shell: &UiShell,
        ui_frame: &UiFrame,
        width: u16,
        height: u16,
    ) -> io::Result<()> {
        let frame = self.renderer.render(shell, ui_frame, width, height);
        crossterm::queue!(self.writer, cursor::Hide)?;
        self.writer.write_all(&frame.bytes)?;
        if let Some((x, y)) = frame.cursor {
            crossterm::queue!(self.writer, cursor::MoveTo(x, y), cursor::Show)?;
        }
        self.writer.flush()
    }

    pub(crate) fn clear(&mut self) -> io::Result<()> {
        crossterm::queue!(self.writer, terminal::Clear(ClearType::All))?;
        self.writer.flush()?;
        self.invalidate();
        Ok(())
    }

    pub(crate) fn show_cursor(&mut self) -> io::Result<()> {
        crossterm::queue!(self.writer, cursor::Show)?;
        self.writer.flush()
    }

    pub(crate) fn invalidate(&mut self) {
        self.renderer.invalidate();
    }
}
