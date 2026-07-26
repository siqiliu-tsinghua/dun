use std::time::{Duration, Instant};

use dun_term::AmbiguousWidth;

use super::super::event::{
    Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use super::{
    CSI_BODY_CAPACITY, EVENT_QUEUE_CAPACITY, Mode, PASTE_CAPACITY, PASTE_END, Parser, State,
};

const NONE: KeyModifiers = KeyModifiers::NONE;
const SHIFT: KeyModifiers = KeyModifiers::SHIFT;
const ALT: KeyModifiers = KeyModifiers::ALT;
const CTRL: KeyModifiers = KeyModifiers::CONTROL;

fn now() -> Instant {
    Instant::now()
}

fn key(code: KeyCode, modifiers: KeyModifiers) -> Event {
    Event::Key(KeyEvent {
        code,
        modifiers,
        kind: KeyEventKind::Press,
    })
}

fn mouse(kind: MouseEventKind, column: u16, row: u16, modifiers: KeyModifiers) -> Event {
    Event::Mouse(MouseEvent {
        kind,
        column,
        row,
        modifiers,
    })
}

fn drain(parser: &mut Parser) -> Vec<Event> {
    std::iter::from_fn(|| parser.pop_event()).collect()
}

fn input_events(bytes: &[u8]) -> Vec<Event> {
    let mut parser = Parser::new(Mode::Input);
    parser.feed(bytes, now());
    drain(&mut parser)
}

fn armed_events(bytes: &[u8], max_bytes: usize) -> Vec<Event> {
    let mut parser = Parser::new(Mode::Input);
    parser.begin_osc52_query(max_bytes);
    parser.feed(bytes, now());
    drain(&mut parser)
}

fn assert_all_splits(bytes: &[u8], expected: &[Event]) {
    let instant = now();
    for split in 0..=bytes.len() {
        let mut parser = Parser::new(Mode::Input);
        parser.feed(&bytes[..split], instant);
        parser.feed(&bytes[split..], instant);
        assert_eq!(
            drain(&mut parser),
            expected,
            "sequence {bytes:?} split at byte {split}"
        );
    }
}

#[test]
fn plain_utf8_controls_and_batches() {
    let controls: &[(&[u8], Event)] = &[
        (b"\0", key(KeyCode::Char(' '), CTRL)),
        (b"\x01", key(KeyCode::Char('a'), CTRL)),
        (b"\x02", key(KeyCode::Char('b'), CTRL)),
        (b"\x03", key(KeyCode::Char('c'), CTRL)),
        (b"\x04", key(KeyCode::Char('d'), CTRL)),
        (b"\x05", key(KeyCode::Char('e'), CTRL)),
        (b"\x06", key(KeyCode::Char('f'), CTRL)),
        (b"\x07", key(KeyCode::Char('g'), CTRL)),
        (b"\x08", key(KeyCode::Char('h'), CTRL)),
        (b"\t", key(KeyCode::Tab, NONE)),
        (b"\x0a", key(KeyCode::Char('j'), CTRL)),
        (b"\x0b", key(KeyCode::Char('k'), CTRL)),
        (b"\x0c", key(KeyCode::Char('l'), CTRL)),
        (b"\r", key(KeyCode::Enter, NONE)),
        (b"\x0e", key(KeyCode::Char('n'), CTRL)),
        (b"\x0f", key(KeyCode::Char('o'), CTRL)),
        (b"\x10", key(KeyCode::Char('p'), CTRL)),
        (b"\x11", key(KeyCode::Char('q'), CTRL)),
        (b"\x12", key(KeyCode::Char('r'), CTRL)),
        (b"\x13", key(KeyCode::Char('s'), CTRL)),
        (b"\x14", key(KeyCode::Char('t'), CTRL)),
        (b"\x15", key(KeyCode::Char('u'), CTRL)),
        (b"\x16", key(KeyCode::Char('v'), CTRL)),
        (b"\x17", key(KeyCode::Char('w'), CTRL)),
        (b"\x18", key(KeyCode::Char('x'), CTRL)),
        (b"\x19", key(KeyCode::Char('y'), CTRL)),
        (b"\x1a", key(KeyCode::Char('z'), CTRL)),
        (b"\x1c", key(KeyCode::Char('4'), CTRL)),
        (b"\x1d", key(KeyCode::Char('5'), CTRL)),
        (b"\x1e", key(KeyCode::Char('6'), CTRL)),
        (b"\x1f", key(KeyCode::Char('7'), CTRL)),
        (b"\x7f", key(KeyCode::Backspace, NONE)),
    ];
    for (bytes, expected) in controls {
        assert_all_splits(bytes, std::slice::from_ref(expected));
    }

    let text: &[(&[u8], Event)] = &[
        (b"a", key(KeyCode::Char('a'), NONE)),
        (b"A", key(KeyCode::Char('A'), SHIFT)),
        ("ñ".as_bytes(), key(KeyCode::Char('ñ'), NONE)),
        ("Ž".as_bytes(), key(KeyCode::Char('Ž'), SHIFT)),
        ("🦀".as_bytes(), key(KeyCode::Char('🦀'), NONE)),
    ];
    for (bytes, expected) in text {
        assert_all_splits(bytes, std::slice::from_ref(expected));
    }

    assert_eq!(
        input_events(b"aB\r"),
        vec![
            key(KeyCode::Char('a'), NONE),
            key(KeyCode::Char('B'), SHIFT),
            key(KeyCode::Enter, NONE),
        ]
    );
}

#[test]
fn bare_escape_uses_the_100ms_fake_clock_deadline() {
    let instant = now();

    let mut before = Parser::new(Mode::Input);
    before.feed(b"\x1b", instant);
    before.expire_escape(instant + Duration::from_millis(99));
    assert_eq!(drain(&mut before), Vec::<Event>::new());
    assert_eq!(
        before.pending_escape_deadline(),
        Some(instant + Duration::from_millis(100))
    );

    let mut exact = Parser::new(Mode::Input);
    exact.feed(b"\x1b", instant);
    exact.expire_escape(instant + Duration::from_millis(100));
    assert_eq!(drain(&mut exact), vec![key(KeyCode::Esc, NONE)]);

    let mut after = Parser::new(Mode::Input);
    after.feed(b"\x1b", instant);
    after.expire_escape(instant + Duration::from_millis(101));
    assert_eq!(drain(&mut after), vec![key(KeyCode::Esc, NONE)]);
}

#[test]
fn alt_escape_and_double_escape_are_incremental() {
    let cases: &[(&[u8], Event)] = &[
        (b"\x1bx", key(KeyCode::Char('x'), ALT)),
        (b"\x1bX", key(KeyCode::Char('X'), ALT | SHIFT)),
        (b"\x1b\x14", key(KeyCode::Char('t'), ALT | CTRL)),
        ("\u{1b}ñ".as_bytes(), key(KeyCode::Char('ñ'), ALT)),
        (b"\x1b\r", key(KeyCode::Enter, ALT)),
    ];
    for (bytes, expected) in cases {
        assert_all_splits(bytes, std::slice::from_ref(expected));
    }

    let instant = now();
    let mut parser = Parser::new(Mode::Input);
    parser.feed(b"\x1b\x1b", instant);
    assert_eq!(drain(&mut parser), vec![key(KeyCode::Esc, NONE)]);
    assert_eq!(
        parser.pending_escape_deadline(),
        Some(instant + Duration::from_millis(100))
    );
    parser.expire_escape(instant + Duration::from_millis(100));
    assert_eq!(drain(&mut parser), vec![key(KeyCode::Esc, NONE)]);
}

#[test]
fn navigation_sequences_cover_modifiers() {
    let unmodified: &[(&[u8], Event)] = &[
        (b"\x1b[A", key(KeyCode::Up, NONE)),
        (b"\x1b[B", key(KeyCode::Down, NONE)),
        (b"\x1b[C", key(KeyCode::Right, NONE)),
        (b"\x1b[D", key(KeyCode::Left, NONE)),
        (b"\x1b[H", key(KeyCode::Home, NONE)),
        (b"\x1b[F", key(KeyCode::End, NONE)),
        (b"\x1bOA", key(KeyCode::Up, NONE)),
        (b"\x1bOB", key(KeyCode::Down, NONE)),
        (b"\x1bOC", key(KeyCode::Right, NONE)),
        (b"\x1bOD", key(KeyCode::Left, NONE)),
        (b"\x1bOH", key(KeyCode::Home, NONE)),
        (b"\x1bOF", key(KeyCode::End, NONE)),
    ];
    for (bytes, expected) in unmodified {
        assert_all_splits(bytes, std::slice::from_ref(expected));
    }

    let masks: &[(&[u8], Event)] = &[
        (b"\x1b[1;1A", key(KeyCode::Up, NONE)),
        (b"\x1b[1;2A", key(KeyCode::Up, SHIFT)),
        (b"\x1b[1;3A", key(KeyCode::Up, ALT)),
        (b"\x1b[1;4A", key(KeyCode::Up, SHIFT | ALT)),
        (b"\x1b[1;5A", key(KeyCode::Up, CTRL)),
        (b"\x1b[1;6A", key(KeyCode::Up, SHIFT | CTRL)),
        (b"\x1b[1;7A", key(KeyCode::Up, ALT | CTRL)),
        (b"\x1b[1;8A", key(KeyCode::Up, SHIFT | ALT | CTRL)),
        (b"\x1b[1B", key(KeyCode::Down, NONE)),
        (b"\x1b[2B", key(KeyCode::Down, SHIFT)),
        (b"\x1b[3B", key(KeyCode::Down, ALT)),
        (b"\x1b[4B", key(KeyCode::Down, SHIFT | ALT)),
        (b"\x1b[5B", key(KeyCode::Down, CTRL)),
        (b"\x1b[6B", key(KeyCode::Down, SHIFT | CTRL)),
        (b"\x1b[7B", key(KeyCode::Down, ALT | CTRL)),
        (b"\x1b[8B", key(KeyCode::Down, SHIFT | ALT | CTRL)),
        (b"\x1b[1;2C", key(KeyCode::Right, SHIFT)),
        (b"\x1b[1;2D", key(KeyCode::Left, SHIFT)),
        (b"\x1b[1;2H", key(KeyCode::Home, SHIFT)),
        (b"\x1b[1;2F", key(KeyCode::End, SHIFT)),
    ];
    for (bytes, expected) in masks {
        assert_all_splits(bytes, std::slice::from_ref(expected));
    }
}

#[test]
fn tilde_editing_keys_and_backtab_cover_all_classic_rows() {
    let cases: &[(&[u8], Event)] = &[
        (b"\x1b[1~", key(KeyCode::Home, NONE)),
        (b"\x1b[7~", key(KeyCode::Home, NONE)),
        (b"\x1b[2~", key(KeyCode::Insert, NONE)),
        (b"\x1b[3~", key(KeyCode::Delete, NONE)),
        (b"\x1b[4~", key(KeyCode::End, NONE)),
        (b"\x1b[8~", key(KeyCode::End, NONE)),
        (b"\x1b[5~", key(KeyCode::PageUp, NONE)),
        (b"\x1b[6~", key(KeyCode::PageDown, NONE)),
        (b"\x1b[3;1~", key(KeyCode::Delete, NONE)),
        (b"\x1b[3;2~", key(KeyCode::Delete, SHIFT)),
        (b"\x1b[3;3~", key(KeyCode::Delete, ALT)),
        (b"\x1b[3;4~", key(KeyCode::Delete, SHIFT | ALT)),
        (b"\x1b[3;5~", key(KeyCode::Delete, CTRL)),
        (b"\x1b[3;6~", key(KeyCode::Delete, SHIFT | CTRL)),
        (b"\x1b[3;7~", key(KeyCode::Delete, ALT | CTRL)),
        (b"\x1b[3;8~", key(KeyCode::Delete, SHIFT | ALT | CTRL)),
        (b"\x1b[1;2~", key(KeyCode::Home, SHIFT)),
        (b"\x1b[2;2~", key(KeyCode::Insert, SHIFT)),
        (b"\x1b[4;2~", key(KeyCode::End, SHIFT)),
        (b"\x1b[5;2~", key(KeyCode::PageUp, SHIFT)),
        (b"\x1b[6;2~", key(KeyCode::PageDown, SHIFT)),
        (b"\x1b[Z", key(KeyCode::BackTab, SHIFT)),
    ];
    for (bytes, expected) in cases {
        assert_all_splits(bytes, std::slice::from_ref(expected));
    }
}

#[test]
fn classic_f1_through_f20_are_complete() {
    let cases: &[(&[u8], Event)] = &[
        (b"\x1bOP", key(KeyCode::F(1), NONE)),
        (b"\x1bOQ", key(KeyCode::F(2), NONE)),
        (b"\x1bOR", key(KeyCode::F(3), NONE)),
        (b"\x1bOS", key(KeyCode::F(4), NONE)),
        (b"\x1b[P", key(KeyCode::F(1), NONE)),
        (b"\x1b[Q", key(KeyCode::F(2), NONE)),
        (b"\x1b[R", key(KeyCode::F(3), NONE)),
        (b"\x1b[S", key(KeyCode::F(4), NONE)),
        (b"\x1b[1;2P", key(KeyCode::F(1), SHIFT)),
        (b"\x1b[1;3Q", key(KeyCode::F(2), ALT)),
        (b"\x1b[1;5R", key(KeyCode::F(3), CTRL)),
        (b"\x1b[1;8S", key(KeyCode::F(4), SHIFT | ALT | CTRL)),
        (b"\x1b[[A", key(KeyCode::F(1), NONE)),
        (b"\x1b[[B", key(KeyCode::F(2), NONE)),
        (b"\x1b[[C", key(KeyCode::F(3), NONE)),
        (b"\x1b[[D", key(KeyCode::F(4), NONE)),
        (b"\x1b[[E", key(KeyCode::F(5), NONE)),
        (b"\x1b[11~", key(KeyCode::F(1), NONE)),
        (b"\x1b[12~", key(KeyCode::F(2), NONE)),
        (b"\x1b[13~", key(KeyCode::F(3), NONE)),
        (b"\x1b[14~", key(KeyCode::F(4), NONE)),
        (b"\x1b[15~", key(KeyCode::F(5), NONE)),
        (b"\x1b[17~", key(KeyCode::F(6), NONE)),
        (b"\x1b[18~", key(KeyCode::F(7), NONE)),
        (b"\x1b[19~", key(KeyCode::F(8), NONE)),
        (b"\x1b[20~", key(KeyCode::F(9), NONE)),
        (b"\x1b[21~", key(KeyCode::F(10), NONE)),
        (b"\x1b[23~", key(KeyCode::F(11), NONE)),
        (b"\x1b[24~", key(KeyCode::F(12), NONE)),
        (b"\x1b[25~", key(KeyCode::F(13), NONE)),
        (b"\x1b[26~", key(KeyCode::F(14), NONE)),
        (b"\x1b[28~", key(KeyCode::F(15), NONE)),
        (b"\x1b[29~", key(KeyCode::F(16), NONE)),
        (b"\x1b[31~", key(KeyCode::F(17), NONE)),
        (b"\x1b[32~", key(KeyCode::F(18), NONE)),
        (b"\x1b[33~", key(KeyCode::F(19), NONE)),
        (b"\x1b[34~", key(KeyCode::F(20), NONE)),
        (b"\x1b[34;2~", key(KeyCode::F(20), SHIFT)),
    ];
    for (bytes, expected) in cases {
        assert_all_splits(bytes, std::slice::from_ref(expected));
    }
}

#[test]
fn sgr_mouse_covers_buttons_drag_move_scroll_and_modifiers() {
    let cases: &[(&[u8], Event)] = &[
        (
            b"\x1b[<0;1;1M",
            mouse(MouseEventKind::Down(MouseButton::Left), 0, 0, NONE),
        ),
        (
            b"\x1b[<1;2;3M",
            mouse(MouseEventKind::Down(MouseButton::Middle), 1, 2, NONE),
        ),
        (
            b"\x1b[<2;3;4M",
            mouse(MouseEventKind::Down(MouseButton::Right), 2, 3, NONE),
        ),
        (
            b"\x1b[<0;4;5m",
            mouse(MouseEventKind::Up(MouseButton::Left), 3, 4, NONE),
        ),
        (
            b"\x1b[<1;5;6m",
            mouse(MouseEventKind::Up(MouseButton::Middle), 4, 5, NONE),
        ),
        (
            b"\x1b[<2;6;7m",
            mouse(MouseEventKind::Up(MouseButton::Right), 5, 6, NONE),
        ),
        (
            b"\x1b[<32;7;8M",
            mouse(MouseEventKind::Drag(MouseButton::Left), 6, 7, NONE),
        ),
        (
            b"\x1b[<33;8;9M",
            mouse(MouseEventKind::Drag(MouseButton::Middle), 7, 8, NONE),
        ),
        (
            b"\x1b[<34;9;10M",
            mouse(MouseEventKind::Drag(MouseButton::Right), 8, 9, NONE),
        ),
        (
            b"\x1b[<35;10;11M",
            mouse(MouseEventKind::Moved, 9, 10, NONE),
        ),
        (
            b"\x1b[<64;11;12M",
            mouse(MouseEventKind::ScrollUp, 10, 11, NONE),
        ),
        (
            b"\x1b[<65;12;13M",
            mouse(MouseEventKind::ScrollDown, 11, 12, NONE),
        ),
        (
            b"\x1b[<66;13;14M",
            mouse(MouseEventKind::ScrollLeft, 12, 13, NONE),
        ),
        (
            b"\x1b[<67;14;15M",
            mouse(MouseEventKind::ScrollRight, 13, 14, NONE),
        ),
        (
            b"\x1b[<28;20;10M",
            mouse(
                MouseEventKind::Down(MouseButton::Left),
                19,
                9,
                SHIFT | ALT | CTRL,
            ),
        ),
    ];
    for (bytes, expected) in cases {
        assert_all_splits(bytes, std::slice::from_ref(expected));
    }

    for rejected in [
        b"\x1b[<0;0;1M".as_slice(),
        b"\x1b[<0;1;0M",
        b"\x1b[<256;1;1M",
        b"\x1b[<0;65536;1M",
        b"\x1b[<0;1;65536M",
        b"\x1b[<0;1;1;M",
    ] {
        assert_eq!(input_events(rejected), Vec::<Event>::new(), "{rejected:?}");
    }
}

#[test]
fn paste_terminator_is_exact_and_fragmented_at_every_byte() {
    assert_all_splits(
        b"\x1b[200~text\x1b[201~",
        &[Event::Paste("text".to_string())],
    );

    let instant = now();
    for split in 0..=PASTE_END.len() {
        let mut parser = Parser::new(Mode::Input);
        parser.feed(b"\x1b[200~payload", instant);
        parser.feed(&PASTE_END[..split], instant);
        if split != PASTE_END.len() {
            assert_eq!(drain(&mut parser), Vec::<Event>::new());
        }
        parser.feed(&PASTE_END[split..], instant);
        assert_eq!(
            drain(&mut parser),
            vec![Event::Paste("payload".to_string())],
            "terminator split at byte {split}"
        );
    }
}

#[test]
fn paste_prefix_without_tilde_does_not_terminate() {
    let instant = now();
    let mut parser = Parser::new(Mode::Input);
    parser.feed(b"\x1b[200~payload\x1b[201", instant);
    assert_eq!(drain(&mut parser), Vec::<Event>::new());
    parser.feed(b"~", instant);
    assert_eq!(
        drain(&mut parser),
        vec![Event::Paste("payload".to_string())]
    );
}

#[test]
fn paste_keeps_embedded_sequences_and_decodes_invalid_utf8_lossily() {
    assert_eq!(
        input_events(b"\x1b[200~a\x1b[2Db\x1b[201Xc\x1b[201~"),
        vec![Event::Paste("a\u{1b}[2Db\u{1b}[201Xc".to_string())]
    );
    assert_eq!(
        input_events(b"\x1b[200~a\xffb\x1b[201~"),
        vec![Event::Paste("a\u{fffd}b".to_string())]
    );
}

#[test]
fn paste_cap_accepts_exact_and_discards_overflow() {
    let instant = now();
    let payload = vec![b'x'; PASTE_CAPACITY];

    let mut exact = Parser::new(Mode::Input);
    exact.feed(b"\x1b[200~", instant);
    exact.feed(&payload, instant);
    exact.feed(PASTE_END, instant);
    let event = exact.pop_event().expect("exact-cap paste event");
    let Event::Paste(text) = event else {
        panic!("exact-cap input was not a paste")
    };
    assert_eq!(text.len(), PASTE_CAPACITY);
    assert!(text.bytes().all(|byte| byte == b'x'));
    assert_eq!(exact.pop_event(), None);

    let mut over = Parser::new(Mode::Input);
    over.feed(b"\x1b[200~", instant);
    over.feed(&payload, instant);
    over.feed(b"yignored\x1b[201~z", instant);
    assert_eq!(drain(&mut over), vec![key(KeyCode::Char('z'), NONE)]);
}

#[test]
fn input_mode_r_is_f3_and_probe_mode_r_is_cpr() {
    assert_all_splits(b"\x1b[R", &[key(KeyCode::F(3), NONE)]);
    assert_all_splits(b"\x1b[1;2R", &[key(KeyCode::F(3), SHIFT)]);

    let instant = now();
    let mut probe = Parser::new(Mode::Probe);
    probe.feed(b"\x1b[1;2R", instant);
    assert_eq!(probe.probe_result(), None);
    probe.feed(b"\x1b[?1;2c", instant);
    assert_eq!(probe.probe_result(), Some(AmbiguousWidth::Narrow));
}

#[test]
fn probe_cpr_da1_is_fragmented_fail_closed_and_sentinel_bound() {
    let instant = now();
    for (bytes, expected) in [
        (
            b"ignored\x1b[31m\x1b[1;2R\x1b[?1;2c".as_slice(),
            AmbiguousWidth::Narrow,
        ),
        (b"\x1b[1;3R\x1b[?1;2c", AmbiguousWidth::Wide),
        (b"\x1b[?1;2c", AmbiguousWidth::Narrow),
        (b"\x1b[1;;3R\x1b[?1;2c", AmbiguousWidth::Narrow),
        (b"\x1b[1;4R\x1b[?1;2c", AmbiguousWidth::Narrow),
        (b"\x1b[1;3R\x1b[?1;;2c\x1b[?1;2c", AmbiguousWidth::Narrow),
    ] {
        for split in 0..=bytes.len() {
            let mut parser = Parser::new(Mode::Probe);
            parser.feed(&bytes[..split], instant);
            parser.feed(&bytes[split..], instant);
            assert_eq!(
                parser.probe_result(),
                Some(expected),
                "{bytes:?} at {split}"
            );
        }
    }

    let mut missing_sentinel = Parser::new(Mode::Probe);
    missing_sentinel.feed(b"\x1b[1;3R", instant);
    assert_eq!(missing_sentinel.probe_result(), None);
    assert_eq!(missing_sentinel.finish_probe(), AmbiguousWidth::Narrow);
}

#[test]
fn probe_caps_response_and_csi_framing() {
    let instant = now();
    let mut exhausted = Parser::new(Mode::Probe);
    exhausted.feed(&[b'x'; 256], instant);
    assert_eq!(exhausted.probe_result(), Some(AmbiguousWidth::Narrow));
    assert_eq!(exhausted.probe_remaining_capacity(), 0);

    let mut oversized = b"\x1b[".to_vec();
    oversized.extend(std::iter::repeat_n(b'1', CSI_BODY_CAPACITY));
    oversized.push(b'R');
    oversized.extend_from_slice(b"\x1b[?1;2c");
    let mut parser = Parser::new(Mode::Probe);
    parser.feed(&oversized, instant);
    assert_eq!(parser.probe_result(), Some(AmbiguousWidth::Narrow));
}

#[test]
fn malformed_and_excluded_input_is_consumed_without_text_leaks() {
    for rejected in [
        b"\x1b[999~".as_slice(),
        b"\x1b[97;5u",
        b"\x1b[97;5:2u",
        b"\x1b[27;5;97~",
        b"\x1b[32;30;40M",
        b"\x1b[I",
        b"\x1b[O",
        b"\x1b[?1;2c",
        b"\xc0",
        b"\xed\xa0\x80",
    ] {
        assert_eq!(input_events(rejected), Vec::<Event>::new(), "{rejected:?}");
    }

    assert_eq!(input_events(b"\xc3(x"), vec![key(KeyCode::Char('x'), NONE)]);
    assert_eq!(
        input_events(b"\x1b[1\x01Ax"),
        vec![key(KeyCode::Char('x'), NONE)]
    );
    assert_all_splits(b"\x1b[Mabcx", &[key(KeyCode::Char('x'), NONE)]);
}

#[test]
fn parser_state_and_event_storage_stay_bounded() {
    let instant = now();
    let mut csi = Parser::new(Mode::Input);
    csi.feed(b"\x1b[", instant);
    csi.feed(&[b'1'; CSI_BODY_CAPACITY], instant);
    assert!(matches!(
        csi.state,
        State::Csi {
            len: CSI_BODY_CAPACITY,
            ..
        }
    ));
    csi.feed(b"1", instant);
    assert!(matches!(csi.state, State::OversizedCsi));

    let mut utf8 = Parser::new(Mode::Input);
    utf8.feed(&[0xf0, 0x9f, 0xa6], instant);
    assert!(matches!(
        utf8.state,
        State::Utf8 {
            len: 3,
            expected: 4,
            ..
        }
    ));

    let mut events = Parser::new(Mode::Input);
    events.feed(&[b'x'; EVENT_QUEUE_CAPACITY + 1], instant);
    assert_eq!(events.events.len(), EVENT_QUEUE_CAPACITY);

    let mut paste = Parser::new(Mode::Input);
    paste.feed(b"\x1b[200~", instant);
    paste.feed(&vec![b'x'; PASTE_CAPACITY], instant);
    assert_eq!(paste.paste.len(), PASTE_CAPACITY);
    paste.feed(b"x", instant);
    assert!(paste.paste.len() <= PASTE_CAPACITY);
    assert!(matches!(paste.state, State::DiscardPaste { .. }));
}

#[test]
fn osc52_armed_selectors_and_terminators_decode_text() {
    for (bytes, expected, limit) in [
        (
            b"\x1b]52;c;Zm9v\x07".as_slice(),
            Event::Osc52Clipboard("foo".to_string()),
            3,
        ),
        (
            b"\x1b]52;p;Zm8=\x1b\\".as_slice(),
            Event::Osc52Clipboard("fo".to_string()),
            2,
        ),
        (
            b"\x1b]52;c;/w==\x07".as_slice(),
            Event::Osc52Clipboard("\\xFF".to_string()),
            1,
        ),
        (
            b"\x1b]52;c;Gwo=\x07".as_slice(),
            Event::Osc52Clipboard("\u{1b}\n".to_string()),
            2,
        ),
    ] {
        assert_eq!(armed_events(bytes, limit), vec![expected], "{bytes:?}");
    }
}

#[test]
fn osc52_st_requires_backslash_byte_by_byte() {
    let instant = now();
    let bytes = b"\x1b]52;c;Zm9vYmFy\x1b\\";
    let mut parser = Parser::new(Mode::Input);
    parser.begin_osc52_query(6);

    for (index, byte) in bytes.iter().enumerate() {
        parser.feed(std::slice::from_ref(byte), instant);
        if index + 1 == bytes.len() {
            assert_eq!(
                drain(&mut parser),
                vec![Event::Osc52Clipboard("foobar".to_string())]
            );
        } else {
            assert_eq!(drain(&mut parser), Vec::<Event>::new(), "byte {index}");
        }
    }
}

#[test]
fn osc52_truncated_frames_expire_and_recover() {
    let instant = now();
    for partial in [
        b"\x1b]52;".as_slice(),
        b"\x1b]52;c;Zm".as_slice(),
        b"\x1b]52;c;Zm9v\x1b".as_slice(),
    ] {
        let mut parser = Parser::new(Mode::Input);
        parser.begin_osc52_query(3);
        parser.feed(partial, instant);
        assert_eq!(
            parser.pending_escape_deadline(),
            Some(instant + Duration::from_millis(100))
        );
        parser.expire_escape(instant + Duration::from_millis(100));
        assert_eq!(drain(&mut parser), Vec::<Event>::new());
        assert_eq!(parser.pending_escape_deadline(), None);
        parser.feed(b"x", instant + Duration::from_millis(100));
        assert_eq!(drain(&mut parser), vec![key(KeyCode::Char('x'), NONE)]);
    }
}

#[test]
fn osc52_cap_accepts_exact_and_discards_cap_plus_one() {
    assert_eq!(
        armed_events(b"\x1b]52;c;Zm9v\x07", 3),
        vec![Event::Osc52Clipboard("foo".to_string())]
    );
    assert_eq!(
        armed_events(b"\x1b]52;c;Zm9vYg==\x07z", 3),
        vec![key(KeyCode::Char('z'), NONE)]
    );
}

#[test]
fn osc52_malformed_unrecognized_and_empty_frames_are_bounded() {
    assert_eq!(
        armed_events(b"\x1b]52;c;Zm$v\x07x", 4),
        vec![key(KeyCode::Char('x'), NONE)]
    );
    assert_eq!(
        armed_events(b"\x1b]9;ignored\x1b\\y", 4),
        vec![key(KeyCode::Char('y'), NONE)]
    );
    assert_eq!(
        armed_events(b"\x1b]52;p;\x07", 0),
        vec![Event::Osc52Clipboard(String::new())]
    );
}

#[test]
fn osc52_escape_not_followed_by_backslash_discards_the_frame() {
    let instant = now();
    let mut parser = Parser::new(Mode::Input);
    parser.begin_osc52_query(3);
    parser.feed(b"\x1b]52;c;Zm9v\x1bXignored", instant);
    assert_eq!(drain(&mut parser), Vec::<Event>::new());
    parser.feed(b"\x07z", instant);
    assert_eq!(drain(&mut parser), vec![key(KeyCode::Char('z'), NONE)]);
}

#[test]
fn osc52_bytes_inside_bracketed_paste_stay_literal() {
    let instant = now();
    let mut parser = Parser::new(Mode::Input);
    parser.feed(b"\x1b[200~", instant);
    parser.begin_osc52_query(3);
    parser.feed(b"\x1b]52;c;Zm9v\x07", instant);
    parser.feed(PASTE_END, instant);
    assert_eq!(
        drain(&mut parser),
        vec![Event::Paste("\u{1b}]52;c;Zm9v\u{7}".to_string())]
    );
}

#[test]
fn unarmed_osc_bytes_keep_alt_bracket_and_parse_normally() {
    assert_eq!(
        input_events(b"\x1b]52;c;Zg==\x07"),
        vec![
            key(KeyCode::Char(']'), ALT),
            key(KeyCode::Char('5'), NONE),
            key(KeyCode::Char('2'), NONE),
            key(KeyCode::Char(';'), NONE),
            key(KeyCode::Char('c'), NONE),
            key(KeyCode::Char(';'), NONE),
            key(KeyCode::Char('Z'), SHIFT),
            key(KeyCode::Char('g'), NONE),
            key(KeyCode::Char('='), NONE),
            key(KeyCode::Char('='), NONE),
            key(KeyCode::Char('g'), CTRL),
        ]
    );
    assert!(
        input_events(b"\x1b]52;p;Zm9v\x1b\\")
            .iter()
            .all(|event| !matches!(event, Event::Osc52Clipboard(_)))
    );
}
