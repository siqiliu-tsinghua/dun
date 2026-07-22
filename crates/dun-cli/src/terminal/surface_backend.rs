use std::io::{self, Write};

use dun_ui::{SurfaceRenderer, UiFrame, UiShell};

use super::{
    TerminalWriter,
    vt::output::{self as vt_output, Sequence},
};

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
        vt_output::queue(&mut self.writer, Sequence::HideCursor)?;
        self.writer.write_all(&frame.bytes)?;
        if let Some((x, y)) = frame.cursor {
            vt_output::queue(&mut self.writer, Sequence::MoveTo { column: x, row: y })?;
            vt_output::queue(&mut self.writer, Sequence::ShowCursor)?;
        }
        self.writer.flush()
    }

    pub(crate) fn clear(&mut self) -> io::Result<()> {
        vt_output::queue(&mut self.writer, Sequence::ClearAll)?;
        self.writer.flush()?;
        self.invalidate();
        Ok(())
    }

    pub(crate) fn show_cursor(&mut self) -> io::Result<()> {
        vt_output::queue(&mut self.writer, Sequence::ShowCursor)?;
        self.writer.flush()
    }

    pub(crate) fn invalidate(&mut self) {
        self.renderer.invalidate();
    }
}
