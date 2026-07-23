use std::io::{self, Write};
use std::time::{Duration, Instant};

use dun_term::{AmbiguousWidth, EncodingProfile};

use super::sys::{Readiness, Terminal};
use super::vt::output::{self as vt_output, Sequence};
use super::vt::parser::{Mode, Parser};

const PROBE_BYTES: &[u8] = b"\r\xe2\x94\x80\x1b[6n\x1b[c";
const PROBE_TIMEOUT: Duration = Duration::from_millis(500);
const MAX_RESPONSE_BYTES: usize = 256;

pub(crate) fn detect_ambiguous_width(
    terminal: &Terminal,
    encoding: EncodingProfile,
) -> AmbiguousWidth {
    let stdout = io::stdout();
    if !should_probe(encoding, terminal.stdin_is_tty(), terminal.stdout_is_tty()) {
        return AmbiguousWidth::Narrow;
    }

    let deadline = Instant::now() + PROBE_TIMEOUT;
    let mut output = stdout.lock();
    let result = if write_probe(&mut output).is_ok() {
        read_responses(terminal, deadline)
    } else {
        AmbiguousWidth::Narrow
    };
    if clear_probe(&mut output).is_err() {
        AmbiguousWidth::Narrow
    } else {
        result
    }
}

fn should_probe(encoding: EncodingProfile, stdin_is_tty: bool, stdout_is_tty: bool) -> bool {
    matches!(encoding, EncodingProfile::Utf8) && stdin_is_tty && stdout_is_tty
}

fn write_probe(output: &mut impl Write) -> io::Result<()> {
    output.write_all(PROBE_BYTES)?;
    output.flush()
}

fn clear_probe(output: &mut impl Write) -> io::Result<()> {
    let move_result = vt_output::queue(output, Sequence::MoveToColumnZero);
    let clear_result = vt_output::queue(output, Sequence::ClearCurrentLine);
    let flush_result = output.flush();
    move_result.and(clear_result).and(flush_result)
}

fn read_responses(terminal: &Terminal, deadline: Instant) -> AmbiguousWidth {
    let mut buffer = [0_u8; MAX_RESPONSE_BYTES];
    let mut parser = Parser::new(Mode::Probe);

    loop {
        if let Some(result) = parser.probe_result() {
            return result;
        }
        let capacity = parser.probe_remaining_capacity();
        if capacity == 0 {
            return parser.finish_probe();
        }

        match terminal.poll_readable(deadline) {
            Ok(Readiness::Readable) => {}
            Ok(Readiness::TimedOut) | Err(_) => return parser.finish_probe(),
        }

        let count = match terminal.read(&mut buffer[..capacity]) {
            Ok(0) | Err(_) => return parser.finish_probe(),
            Ok(count) => count,
        };
        parser.feed(&buffer[..count], Instant::now());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DA1: &[u8] = b"\x1b[?1;2c";

    fn completed_response(bytes: &[u8]) -> AmbiguousWidth {
        let mut parser = Parser::new(Mode::Probe);
        parser.feed(bytes, Instant::now());
        parser
            .probe_result()
            .unwrap_or_else(|| parser.finish_probe())
    }

    #[test]
    fn probe_writes_the_exact_query_bytes() {
        let mut output = Vec::new();

        write_probe(&mut output).expect("write probe");

        assert_eq!(output, b"\r\xe2\x94\x80\x1b[6n\x1b[c");
    }

    #[test]
    fn probe_requires_utf8_stdin_tty_and_stdout_tty() {
        assert!(should_probe(EncodingProfile::Utf8, true, true));
        assert!(!should_probe(EncodingProfile::Ascii, true, true));
        assert!(!should_probe(EncodingProfile::Utf8, false, true));
        assert!(!should_probe(EncodingProfile::Utf8, true, false));
    }

    #[test]
    fn fragmented_narrow_cpr_waits_for_da1() {
        let mut parser = Parser::new(Mode::Probe);
        let instant = Instant::now();

        parser.feed(b"ignored\x1b[31m\x1b[1;", instant);
        assert_eq!(parser.probe_result(), None);
        parser.feed(b"2R\x1b", instant);
        assert_eq!(parser.probe_result(), None);
        parser.feed(b"[?1;2c", instant);
        assert_eq!(parser.probe_result(), Some(AmbiguousWidth::Narrow));
    }

    #[test]
    fn fragmented_wide_cpr_waits_for_da1() {
        let mut parser = Parser::new(Mode::Probe);
        let instant = Instant::now();

        parser.feed(b"\x1b", instant);
        assert_eq!(parser.probe_result(), None);
        parser.feed(b"[1;3", instant);
        assert_eq!(parser.probe_result(), None);
        parser.feed(b"R", instant);
        assert_eq!(parser.probe_result(), None);
        parser.feed(DA1, instant);
        assert_eq!(parser.probe_result(), Some(AmbiguousWidth::Wide));
    }

    #[test]
    fn da1_without_cpr_is_narrow() {
        assert_eq!(completed_response(DA1), AmbiguousWidth::Narrow);
    }

    #[test]
    fn cpr_without_da1_times_out_to_narrow() {
        let mut parser = Parser::new(Mode::Probe);
        parser.feed(b"\x1b[1;3R", Instant::now());

        assert_eq!(parser.probe_result(), None);
        assert_eq!(parser.finish_probe(), AmbiguousWidth::Narrow);
    }

    #[test]
    fn malformed_cpr_is_narrow() {
        assert_eq!(
            completed_response(b"\x1b[1;;3R\x1b[?1;2c"),
            AmbiguousWidth::Narrow
        );
    }

    #[test]
    fn out_of_range_cpr_column_is_narrow() {
        assert_eq!(
            completed_response(b"\x1b[1;4R\x1b[?1;2c"),
            AmbiguousWidth::Narrow
        );
    }

    #[test]
    fn oversized_csi_is_narrow() {
        let mut bytes = b"\x1b[".to_vec();
        bytes.extend(std::iter::repeat_n(b'1', 30));
        bytes.push(b'R');
        bytes.extend_from_slice(DA1);

        assert_eq!(completed_response(&bytes), AmbiguousWidth::Narrow);
    }

    #[test]
    fn exhausted_response_buffer_is_narrow() {
        assert_eq!(
            completed_response(&[b'x'; MAX_RESPONSE_BYTES]),
            AmbiguousWidth::Narrow
        );
    }
}
