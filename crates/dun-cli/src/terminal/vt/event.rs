use std::ops::{BitOr, BitOrAssign};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum KeyCode {
    Backspace,
    Enter,
    Left,
    Right,
    Up,
    Down,
    Home,
    End,
    PageUp,
    PageDown,
    Tab,
    BackTab,
    Delete,
    Insert,
    F(u8),
    Char(char),
    Esc,
    Null,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct KeyModifiers(u8);

impl KeyModifiers {
    pub(crate) const NONE: Self = Self(0);
    pub(crate) const SHIFT: Self = Self(1 << 0);
    pub(crate) const CONTROL: Self = Self(1 << 1);
    pub(crate) const ALT: Self = Self(1 << 2);

    pub(crate) const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }
}

impl BitOr for KeyModifiers {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl BitOrAssign for KeyModifiers {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum KeyEventKind {
    Press,
    Repeat,
    Release,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct KeyEvent {
    pub code: KeyCode,
    pub modifiers: KeyModifiers,
    pub kind: KeyEventKind,
}

impl KeyEvent {
    pub(crate) const fn new(code: KeyCode, modifiers: KeyModifiers) -> Self {
        Self {
            code,
            modifiers,
            kind: KeyEventKind::Press,
        }
    }

    pub(crate) const fn new_with_kind(
        code: KeyCode,
        modifiers: KeyModifiers,
        kind: KeyEventKind,
    ) -> Self {
        Self {
            kind,
            ..Self::new(code, modifiers)
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum MouseButton {
    Left,
    Middle,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum MouseEventKind {
    Down(MouseButton),
    Up(MouseButton),
    Drag(MouseButton),
    Moved,
    ScrollUp,
    ScrollDown,
    ScrollLeft,
    ScrollRight,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct MouseEvent {
    pub kind: MouseEventKind,
    pub column: u16,
    pub row: u16,
    pub modifiers: KeyModifiers,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) enum Event {
    Key(KeyEvent),
    Mouse(MouseEvent),
    Paste(String),
    Resize(u16, u16),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_event_constructors_set_the_requested_kind() {
        assert_eq!(
            KeyEvent::new(KeyCode::Char('x'), KeyModifiers::ALT),
            KeyEvent {
                code: KeyCode::Char('x'),
                modifiers: KeyModifiers::ALT,
                kind: KeyEventKind::Press,
            }
        );
        assert_eq!(
            KeyEvent::new_with_kind(KeyCode::Enter, KeyModifiers::CONTROL, KeyEventKind::Repeat,),
            KeyEvent {
                code: KeyCode::Enter,
                modifiers: KeyModifiers::CONTROL,
                kind: KeyEventKind::Repeat,
            }
        );
    }

    #[test]
    fn key_event_equality_is_structural() {
        assert_ne!(
            KeyEvent::new(KeyCode::Char('A'), KeyModifiers::NONE),
            KeyEvent::new(KeyCode::Char('a'), KeyModifiers::SHIFT)
        );
    }

    #[test]
    fn key_modifier_contains_and_bit_operations_preserve_each_bit() {
        let mut modifiers = KeyModifiers::SHIFT | KeyModifiers::CONTROL;

        assert!(modifiers.contains(KeyModifiers::SHIFT));
        assert!(modifiers.contains(KeyModifiers::CONTROL));
        assert!(!modifiers.contains(KeyModifiers::ALT));
        assert!(modifiers.contains(KeyModifiers::NONE));

        modifiers |= KeyModifiers::ALT;

        assert_eq!(
            modifiers,
            KeyModifiers::SHIFT | KeyModifiers::CONTROL | KeyModifiers::ALT
        );
    }
}
