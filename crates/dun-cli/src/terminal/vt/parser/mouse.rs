use super::super::event::{KeyModifiers, MouseButton, MouseEvent, MouseEventKind};

pub(super) fn parse_sgr(body: &[u8]) -> Option<MouseEvent> {
    let (&final_byte, parameters) = body.split_last()?;
    if !matches!(final_byte, b'M' | b'm') {
        return None;
    }
    let parameters = parameters.strip_prefix(b"<")?;
    let mut parts = parameters.split(|&byte| byte == b';');
    let cb = u8::try_from(parse_decimal(parts.next()?)?).ok()?;
    let column = parse_decimal(parts.next()?)?.checked_sub(1)?;
    let row = parse_decimal(parts.next()?)?.checked_sub(1)?;
    if parts.next().is_some() {
        return None;
    }

    let (kind, modifiers) = parse_cb(cb)?;
    let kind = if final_byte == b'm' {
        match kind {
            MouseEventKind::Down(button) => MouseEventKind::Up(button),
            other => other,
        }
    } else {
        kind
    };

    Some(MouseEvent {
        kind,
        column,
        row,
        modifiers,
    })
}

fn parse_cb(cb: u8) -> Option<(MouseEventKind, KeyModifiers)> {
    let button = (cb & 0b0000_0011) | ((cb & 0b1100_0000) >> 4);
    let dragging = cb & 0b0010_0000 != 0;
    let kind = match (button, dragging) {
        (0, false) => MouseEventKind::Down(MouseButton::Left),
        (1, false) => MouseEventKind::Down(MouseButton::Middle),
        (2, false) => MouseEventKind::Down(MouseButton::Right),
        (0, true) => MouseEventKind::Drag(MouseButton::Left),
        (1, true) => MouseEventKind::Drag(MouseButton::Middle),
        (2, true) => MouseEventKind::Drag(MouseButton::Right),
        (3, false) => MouseEventKind::Up(MouseButton::Left),
        (3..=5, true) => MouseEventKind::Moved,
        (4, false) => MouseEventKind::ScrollUp,
        (5, false) => MouseEventKind::ScrollDown,
        (6, false) => MouseEventKind::ScrollLeft,
        (7, false) => MouseEventKind::ScrollRight,
        _ => return None,
    };

    let mut modifiers = KeyModifiers::NONE;
    if cb & 0b0000_0100 != 0 {
        modifiers |= KeyModifiers::SHIFT;
    }
    if cb & 0b0000_1000 != 0 {
        modifiers |= KeyModifiers::ALT;
    }
    if cb & 0b0001_0000 != 0 {
        modifiers |= KeyModifiers::CONTROL;
    }
    Some((kind, modifiers))
}

fn parse_decimal(bytes: &[u8]) -> Option<u16> {
    if bytes.is_empty() {
        return None;
    }
    bytes.iter().try_fold(0_u16, |value, &byte| {
        let digit = u16::from(byte.checked_sub(b'0')?);
        if digit > 9 {
            return None;
        }
        value.checked_mul(10)?.checked_add(digit)
    })
}
