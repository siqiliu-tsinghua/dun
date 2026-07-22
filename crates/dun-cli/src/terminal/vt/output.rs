use std::io::{self, Write};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::terminal) enum Sequence {
    EnterAlternateScreen,
    LeaveAlternateScreen,
    EnableBracketedPaste,
    DisableBracketedPaste,
    EnableMouseCapture,
    DisableMouseCapture,
    HideCursor,
    ShowCursor,
    MoveTo { column: u16, row: u16 },
    ClearAll,
    MoveToColumnZero,
    ClearCurrentLine,
}

pub(in crate::terminal) fn queue<W: Write + ?Sized>(
    writer: &mut W,
    sequence: Sequence,
) -> io::Result<()> {
    let bytes = match sequence {
        Sequence::EnterAlternateScreen => b"\x1b[?1049h".as_slice(),
        Sequence::LeaveAlternateScreen => b"\x1b[?1049l".as_slice(),
        Sequence::EnableBracketedPaste => b"\x1b[?2004h".as_slice(),
        Sequence::DisableBracketedPaste => b"\x1b[?2004l".as_slice(),
        Sequence::EnableMouseCapture => b"\x1b[?1000h\x1b[?1002h\x1b[?1006h".as_slice(),
        Sequence::DisableMouseCapture => b"\x1b[?1006l\x1b[?1002l\x1b[?1000l".as_slice(),
        Sequence::HideCursor => b"\x1b[?25l".as_slice(),
        Sequence::ShowCursor => b"\x1b[?25h".as_slice(),
        Sequence::MoveTo { column, row } => {
            return write!(
                writer,
                "\x1b[{};{}H",
                u32::from(row) + 1,
                u32::from(column) + 1
            );
        }
        Sequence::ClearAll => b"\x1b[2J".as_slice(),
        Sequence::MoveToColumnZero => b"\x1b[1G".as_slice(),
        Sequence::ClearCurrentLine => b"\x1b[2K".as_slice(),
    };
    writer.write_all(bytes)
}

pub(in crate::terminal) fn execute<W: Write + ?Sized>(
    writer: &mut W,
    sequence: Sequence,
) -> io::Result<()> {
    queue(writer, sequence)?;
    writer.flush()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct FlushCountingWriter {
        bytes: Vec<u8>,
        flush_count: usize,
    }

    impl Write for FlushCountingWriter {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            self.bytes.extend_from_slice(bytes);
            Ok(bytes.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            self.flush_count += 1;
            Ok(())
        }
    }

    #[test]
    fn fixed_sequences_match_literal_bytes() {
        let cases: &[(Sequence, &[u8])] = &[
            (Sequence::EnterAlternateScreen, b"\x1b[?1049h"),
            (Sequence::LeaveAlternateScreen, b"\x1b[?1049l"),
            (Sequence::EnableBracketedPaste, b"\x1b[?2004h"),
            (Sequence::DisableBracketedPaste, b"\x1b[?2004l"),
            (Sequence::HideCursor, b"\x1b[?25l"),
            (Sequence::ShowCursor, b"\x1b[?25h"),
            (Sequence::ClearAll, b"\x1b[2J"),
            (Sequence::MoveToColumnZero, b"\x1b[1G"),
            (Sequence::ClearCurrentLine, b"\x1b[2K"),
        ];

        for &(sequence, expected) in cases {
            let mut writer = FlushCountingWriter::default();
            queue(&mut writer, sequence).expect("queue sequence");

            assert_eq!(writer.bytes, expected);
            assert_eq!(writer.flush_count, 0, "queue must not flush");
        }
    }

    #[test]
    fn mouse_sequences_use_exact_supported_modes_and_order() {
        let mut writer = FlushCountingWriter::default();

        queue(&mut writer, Sequence::EnableMouseCapture).expect("queue mouse enable");
        assert_eq!(
            writer.bytes, b"\x1b[?1000h\x1b[?1002h\x1b[?1006h",
            "mouse enable must use 1000/1002/1006 in that order"
        );
        assert_eq!(writer.flush_count, 0, "queue must not flush");

        writer.bytes.clear();
        queue(&mut writer, Sequence::DisableMouseCapture).expect("queue mouse disable");
        assert_eq!(
            writer.bytes, b"\x1b[?1006l\x1b[?1002l\x1b[?1000l",
            "mouse disable must reverse the enable order"
        );
        assert_eq!(writer.flush_count, 0, "queue must not flush");
    }

    #[test]
    fn move_to_converts_zero_based_column_row_to_one_based_row_column() {
        let mut writer = FlushCountingWriter::default();

        queue(&mut writer, Sequence::MoveTo { column: 2, row: 7 }).expect("queue cursor move");

        assert_eq!(
            writer.bytes, b"\x1b[8;3H",
            "move-to must encode 1-based row before column"
        );
        assert_eq!(writer.flush_count, 0, "queue must not flush");
    }

    #[test]
    fn execute_writes_then_flushes_once() {
        let mut writer = FlushCountingWriter::default();

        execute(&mut writer, Sequence::ShowCursor).expect("execute cursor show");

        assert_eq!(writer.bytes, b"\x1b[?25h");
        assert_eq!(writer.flush_count, 1, "execute must flush once");
    }
}
