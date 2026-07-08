use std::fmt;
use std::str::FromStr;

use super::key::{KeyParseError, KeyStroke, key_stroke_text};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct KeySequence {
    pub strokes: Vec<KeyStroke>,
}

impl KeySequence {
    pub fn single(stroke: KeyStroke) -> Self {
        Self {
            strokes: vec![stroke],
        }
    }

    pub fn parse_lossy(input: &str) -> Self {
        Self::from_str(input).expect("default key sequence should be valid")
    }
}

impl FromStr for KeySequence {
    type Err = KeyParseError;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let trimmed = input.trim();
        if trimmed.is_empty() {
            return Err(KeyParseError::EmptySequence);
        }

        let mut strokes = Vec::new();
        for raw_stroke in trimmed.split(',') {
            let raw_stroke = raw_stroke.trim();
            if raw_stroke.is_empty() {
                return Err(KeyParseError::EmptyStroke);
            }
            strokes.push(KeyStroke::from_str(raw_stroke)?);
        }

        Ok(Self { strokes })
    }
}

impl fmt::Display for KeySequence {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", key_sequence_text(self))
    }
}

fn key_sequence_text(sequence: &KeySequence) -> String {
    sequence
        .strokes
        .iter()
        .map(key_stroke_text)
        .collect::<Vec<_>>()
        .join(",")
}
