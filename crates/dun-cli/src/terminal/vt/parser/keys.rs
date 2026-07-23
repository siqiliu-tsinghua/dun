use super::super::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

pub(super) fn parse_ss3(byte: u8) -> Option<KeyEvent> {
    let code = match byte {
        b'A' => KeyCode::Up,
        b'B' => KeyCode::Down,
        b'C' => KeyCode::Right,
        b'D' => KeyCode::Left,
        b'H' => KeyCode::Home,
        b'F' => KeyCode::End,
        b'P'..=b'S' => KeyCode::F(byte - b'P' + 1),
        _ => return None,
    };
    Some(key(code, KeyModifiers::NONE))
}

pub(super) fn parse_csi(body: &[u8]) -> Option<KeyEvent> {
    let (&final_byte, parameters) = body.split_last()?;

    if let Some(code) = navigation_code(final_byte) {
        return Some(key(code, navigation_modifiers(parameters)?));
    }
    if matches!(final_byte, b'P'..=b'S') {
        let modifiers = function_modifiers(parameters)?;
        return Some(key(KeyCode::F(final_byte - b'P' + 1), modifiers));
    }
    if final_byte == b'Z' && parameters.is_empty() {
        return Some(key(KeyCode::BackTab, KeyModifiers::SHIFT));
    }
    if final_byte != b'~' {
        return None;
    }

    let (number, modifiers) = tilde_parameters(parameters)?;
    let code = match number {
        1 | 7 => KeyCode::Home,
        2 => KeyCode::Insert,
        3 => KeyCode::Delete,
        4 | 8 => KeyCode::End,
        5 => KeyCode::PageUp,
        6 => KeyCode::PageDown,
        11..=15 => KeyCode::F(u8::try_from(number - 10).ok()?),
        17..=21 => KeyCode::F(u8::try_from(number - 11).ok()?),
        23..=26 => KeyCode::F(u8::try_from(number - 12).ok()?),
        28..=29 => KeyCode::F(u8::try_from(number - 13).ok()?),
        31..=34 => KeyCode::F(u8::try_from(number - 14).ok()?),
        _ => return None,
    };
    Some(key(code, modifiers))
}

pub(super) fn parse_legacy_double_bracket(byte: u8) -> Option<KeyEvent> {
    match byte {
        b'A'..=b'E' => Some(key(KeyCode::F(byte - b'A' + 1), KeyModifiers::NONE)),
        _ => None,
    }
}

fn navigation_code(byte: u8) -> Option<KeyCode> {
    match byte {
        b'A' => Some(KeyCode::Up),
        b'B' => Some(KeyCode::Down),
        b'C' => Some(KeyCode::Right),
        b'D' => Some(KeyCode::Left),
        b'H' => Some(KeyCode::Home),
        b'F' => Some(KeyCode::End),
        _ => None,
    }
}

fn navigation_modifiers(parameters: &[u8]) -> Option<KeyModifiers> {
    match parameters {
        [] => Some(KeyModifiers::NONE),
        [mask] => modifiers(*mask),
        [b'1', b';', mask] => modifiers(*mask),
        _ => None,
    }
}

fn function_modifiers(parameters: &[u8]) -> Option<KeyModifiers> {
    match parameters {
        [] => Some(KeyModifiers::NONE),
        [b'1', b';', mask] => modifiers(*mask),
        _ => None,
    }
}

fn tilde_parameters(parameters: &[u8]) -> Option<(u16, KeyModifiers)> {
    let mut parts = parameters.split(|&byte| byte == b';');
    let number = parse_decimal(parts.next()?)?;
    let modifiers = match parts.next() {
        None => KeyModifiers::NONE,
        Some([mask]) => modifiers(*mask)?,
        Some(_) => return None,
    };
    if parts.next().is_some() {
        return None;
    }
    Some((number, modifiers))
}

fn modifiers(mask: u8) -> Option<KeyModifiers> {
    let bits = mask.checked_sub(b'1')?;
    if bits > 7 {
        return None;
    }

    let mut modifiers = KeyModifiers::NONE;
    if bits & 1 != 0 {
        modifiers |= KeyModifiers::SHIFT;
    }
    if bits & 2 != 0 {
        modifiers |= KeyModifiers::ALT;
    }
    if bits & 4 != 0 {
        modifiers |= KeyModifiers::CONTROL;
    }
    Some(modifiers)
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

const fn key(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
    KeyEvent {
        code,
        modifiers,
        kind: KeyEventKind::Press,
    }
}
