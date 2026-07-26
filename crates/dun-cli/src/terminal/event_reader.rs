use std::io;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use signal_hook::consts::signal::SIGWINCH;
use signal_hook::{SigId, flag, low_level};

use super::sys::{Readiness, Terminal};
use super::vt::event::Event;
use super::vt::parser::{Mode, Parser};

const MAX_READ_BYTES: usize = 1_024;

pub(crate) struct EventReader {
    core: ReaderCore<Terminal>,
    signal_id: Option<SigId>,
}

impl EventReader {
    pub(crate) fn new(terminal: Arc<Terminal>) -> io::Result<Self> {
        let resize_pending = Arc::new(AtomicBool::new(false));
        let signal_id = flag::register(SIGWINCH, resize_pending.clone())?;
        Ok(Self {
            core: ReaderCore::new(terminal, resize_pending),
            signal_id: Some(signal_id),
        })
    }

    pub(crate) fn next_event(&mut self, timeout: Duration) -> io::Result<Option<Event>> {
        self.core.next_event(timeout)
    }

    #[allow(dead_code)]
    pub(crate) fn begin_osc52_query(&mut self, max_bytes: usize) {
        self.core.begin_osc52_query(max_bytes);
    }

    #[allow(dead_code)]
    pub(crate) fn cancel_osc52_query(&mut self) {
        self.core.cancel_osc52_query();
    }

    #[allow(dead_code)]
    pub(crate) fn next_osc52_response(&mut self, timeout: Duration) -> io::Result<Option<String>> {
        self.core.next_osc52_response(timeout)
    }
}

impl Drop for EventReader {
    fn drop(&mut self) {
        if let Some(signal_id) = self.signal_id.take() {
            low_level::unregister(signal_id);
        }
    }
}

trait EventSource {
    fn poll_readable(&self, deadline: Instant) -> io::Result<Readiness>;
    fn read(&self, buffer: &mut [u8]) -> io::Result<usize>;
    fn size(&self) -> io::Result<(u16, u16)>;
}

impl EventSource for Terminal {
    fn poll_readable(&self, deadline: Instant) -> io::Result<Readiness> {
        Terminal::poll_readable(self, deadline)
    }

    fn read(&self, buffer: &mut [u8]) -> io::Result<usize> {
        Terminal::read(self, buffer)
    }

    fn size(&self) -> io::Result<(u16, u16)> {
        Terminal::size(self)
    }
}

struct ReaderCore<S> {
    parser: Parser,
    resize_pending: Arc<AtomicBool>,
    source: Arc<S>,
}

impl<S: EventSource> ReaderCore<S> {
    fn new(source: Arc<S>, resize_pending: Arc<AtomicBool>) -> Self {
        Self {
            parser: Parser::new(Mode::Input),
            resize_pending,
            source,
        }
    }

    fn next_event(&mut self, timeout: Duration) -> io::Result<Option<Event>> {
        let caller_deadline = Instant::now()
            .checked_add(timeout)
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "event timeout overflow"))?;

        loop {
            if let Some(event) = self.parser.pop_event() {
                return Ok(Some(event));
            }
            if let Some(event) = self.take_resize()? {
                return Ok(Some(event));
            }

            let now = Instant::now();
            self.parser.expire_escape(now);
            if let Some(event) = self.parser.pop_event() {
                return Ok(Some(event));
            }
            if now >= caller_deadline {
                return Ok(None);
            }

            let poll_deadline = self
                .parser
                .pending_escape_deadline()
                .map_or(caller_deadline, |escape_deadline| {
                    escape_deadline.min(caller_deadline)
                });
            match self.source.poll_readable(poll_deadline)? {
                Readiness::TimedOut => {}
                Readiness::Readable => {
                    let mut buffer = [0; MAX_READ_BYTES];
                    let count = self.source.read(&mut buffer)?;
                    if count == 0 {
                        return Err(io::Error::new(
                            io::ErrorKind::UnexpectedEof,
                            "terminal input closed after readable readiness",
                        ));
                    }
                    self.parser.feed(&buffer[..count], Instant::now());
                }
            }
        }
    }

    fn begin_osc52_query(&mut self, max_bytes: usize) {
        self.parser.begin_osc52_query(max_bytes);
    }

    fn cancel_osc52_query(&mut self) {
        self.parser.cancel_osc52_query();
    }

    fn next_osc52_response(&mut self, timeout: Duration) -> io::Result<Option<String>> {
        let caller_deadline = Instant::now()
            .checked_add(timeout)
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "event timeout overflow"))?;

        loop {
            if let Some(text) = self.parser.pop_osc52_response() {
                return Ok(Some(text));
            }

            let now = Instant::now();
            self.parser.expire_escape(now);
            if let Some(text) = self.parser.pop_osc52_response() {
                return Ok(Some(text));
            }
            if now >= caller_deadline {
                self.parser.cancel_osc52_query();
                return Ok(None);
            }

            let poll_deadline = self
                .parser
                .pending_escape_deadline()
                .map_or(caller_deadline, |escape_deadline| {
                    escape_deadline.min(caller_deadline)
                });
            match self.source.poll_readable(poll_deadline)? {
                Readiness::TimedOut => {}
                Readiness::Readable => {
                    let mut buffer = [0; MAX_READ_BYTES];
                    let count = self.source.read(&mut buffer)?;
                    if count == 0 {
                        self.parser.cancel_osc52_query();
                        return Err(io::Error::new(
                            io::ErrorKind::UnexpectedEof,
                            "terminal input closed after readable readiness",
                        ));
                    }
                    self.parser.feed(&buffer[..count], Instant::now());
                }
            }
        }
    }

    fn take_resize(&self) -> io::Result<Option<Event>> {
        if !self.resize_pending.swap(false, Ordering::SeqCst) {
            return Ok(None);
        }
        let (width, height) = self.source.size()?;
        Ok(Some(Event::Resize(width, height)))
    }
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;
    use std::sync::atomic::AtomicUsize;
    use std::sync::{Mutex, MutexGuard, PoisonError};

    use super::*;
    use crate::terminal::vt::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

    struct FakeSource {
        batches: Mutex<VecDeque<Vec<u8>>>,
        polls: AtomicUsize,
        reads: AtomicUsize,
        size: Mutex<(u16, u16)>,
    }

    impl FakeSource {
        fn new(batches: &[&[u8]]) -> Self {
            Self {
                batches: Mutex::new(batches.iter().map(|bytes| bytes.to_vec()).collect()),
                polls: AtomicUsize::new(0),
                reads: AtomicUsize::new(0),
                size: Mutex::new((80, 24)),
            }
        }

        fn batches(&self) -> MutexGuard<'_, VecDeque<Vec<u8>>> {
            self.batches.lock().unwrap_or_else(PoisonError::into_inner)
        }
    }

    impl EventSource for FakeSource {
        fn poll_readable(&self, _deadline: Instant) -> io::Result<Readiness> {
            self.polls.fetch_add(1, Ordering::SeqCst);
            Ok(if self.batches().is_empty() {
                Readiness::TimedOut
            } else {
                Readiness::Readable
            })
        }

        fn read(&self, buffer: &mut [u8]) -> io::Result<usize> {
            self.reads.fetch_add(1, Ordering::SeqCst);
            let mut batches = self.batches();
            let batch = batches.pop_front().expect("readable fake batch");
            let count = batch.len().min(buffer.len());
            buffer[..count].copy_from_slice(&batch[..count]);
            if count != batch.len() {
                batches.push_front(batch[count..].to_vec());
            }
            Ok(count)
        }

        fn size(&self) -> io::Result<(u16, u16)> {
            Ok(*self.size.lock().unwrap_or_else(PoisonError::into_inner))
        }
    }

    fn pressed(ch: char) -> Event {
        Event::Key(KeyEvent {
            code: KeyCode::Char(ch),
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
        })
    }

    #[test]
    fn queued_events_are_drained_before_the_next_input_batch_is_read() {
        let source = Arc::new(FakeSource::new(&[b"ab", b"c"]));
        let resize_pending = Arc::new(AtomicBool::new(false));
        let mut reader = ReaderCore::new(source.clone(), resize_pending);

        assert_eq!(
            reader.next_event(Duration::from_secs(1)).unwrap(),
            Some(pressed('a'))
        );
        assert_eq!(source.reads.load(Ordering::SeqCst), 1);
        assert_eq!(
            reader.next_event(Duration::from_secs(1)).unwrap(),
            Some(pressed('b'))
        );
        assert_eq!(
            source.reads.load(Ordering::SeqCst),
            1,
            "second batch was read too early"
        );
        assert_eq!(
            reader.next_event(Duration::from_secs(1)).unwrap(),
            Some(pressed('c'))
        );
        assert_eq!(source.reads.load(Ordering::SeqCst), 2);
        assert_eq!(source.polls.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn resize_flag_is_consumed_before_polling_for_input() {
        let source = Arc::new(FakeSource::new(&[]));
        *source.size.lock().unwrap_or_else(PoisonError::into_inner) = (100, 30);
        let resize_pending = Arc::new(AtomicBool::new(true));
        let mut reader = ReaderCore::new(source.clone(), resize_pending.clone());

        assert_eq!(
            reader.next_event(Duration::ZERO).unwrap(),
            Some(Event::Resize(100, 30))
        );
        assert!(!resize_pending.load(Ordering::SeqCst));
        assert_eq!(source.polls.load(Ordering::SeqCst), 0);
        assert_eq!(source.reads.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn osc52_response_is_extracted_without_reordering_ordinary_events() {
        let source = Arc::new(FakeSource::new(&[b"a\x1b]52;c;Zm9v\x07b"]));
        let resize_pending = Arc::new(AtomicBool::new(false));
        let mut reader = ReaderCore::new(source, resize_pending);
        reader.begin_osc52_query(3);

        assert_eq!(
            reader.next_osc52_response(Duration::from_secs(1)).unwrap(),
            Some("foo".to_string())
        );
        assert_eq!(
            reader.next_event(Duration::ZERO).unwrap(),
            Some(pressed('a'))
        );
        assert_eq!(
            reader.next_event(Duration::ZERO).unwrap(),
            Some(pressed('b'))
        );
    }

    #[test]
    fn osc52_no_response_deadline_preserves_queued_ordinary_events() {
        let source = Arc::new(FakeSource::new(&[]));
        let resize_pending = Arc::new(AtomicBool::new(false));
        let mut reader = ReaderCore::new(source.clone(), resize_pending);
        reader.begin_osc52_query(3);
        reader.parser.feed(b"ab", Instant::now());

        assert_eq!(reader.next_osc52_response(Duration::ZERO).unwrap(), None);
        assert_eq!(source.polls.load(Ordering::SeqCst), 0);
        assert_eq!(
            reader.next_event(Duration::ZERO).unwrap(),
            Some(pressed('a'))
        );
        assert_eq!(
            reader.next_event(Duration::ZERO).unwrap(),
            Some(pressed('b'))
        );
    }

    #[test]
    fn osc52_cancel_restores_unarmed_escape_bracket_behavior() {
        let source = Arc::new(FakeSource::new(&[b"\x1b]x"]));
        let resize_pending = Arc::new(AtomicBool::new(false));
        let mut reader = ReaderCore::new(source, resize_pending);
        reader.begin_osc52_query(3);
        reader.cancel_osc52_query();

        assert_eq!(
            reader.next_event(Duration::from_secs(1)).unwrap(),
            Some(Event::Key(KeyEvent {
                code: KeyCode::Char(']'),
                modifiers: KeyModifiers::ALT,
                kind: KeyEventKind::Press,
            }))
        );
        assert_eq!(
            reader.next_event(Duration::ZERO).unwrap(),
            Some(pressed('x'))
        );
    }

    #[test]
    fn osc52_partial_frame_then_eof_is_an_error_without_an_event() {
        let source = Arc::new(FakeSource::new(&[b"\x1b]52;c;Zm", b""]));
        let resize_pending = Arc::new(AtomicBool::new(false));
        let mut reader = ReaderCore::new(source, resize_pending);
        reader.begin_osc52_query(3);

        let error = reader
            .next_osc52_response(Duration::from_secs(1))
            .unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::UnexpectedEof);
        assert_eq!(reader.parser.pop_osc52_response(), None);
        assert_eq!(reader.parser.pending_escape_deadline(), None);
    }
}
