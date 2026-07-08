use std::collections::HashSet;
use std::str::FromStr;

use super::KeyStroke;
use crate::{CommandParseError, normalize_command_id};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileDialogKeymap {
    pub bindings: Vec<FileDialogKeyBinding>,
}

impl FileDialogKeymap {
    pub fn new(bindings: Vec<FileDialogKeyBinding>) -> Result<Self, FileDialogKeymapError> {
        let keymap = Self { bindings };
        keymap.validate()?;
        Ok(keymap)
    }

    pub fn default_file_dialog() -> Self {
        Self {
            bindings: vec![
                FileDialogKeyBinding::new("Esc", FileDialogAction::Cancel),
                FileDialogKeyBinding::new("Enter", FileDialogAction::Submit),
                FileDialogKeyBinding::new("Tab", FileDialogAction::CompleteForward),
                FileDialogKeyBinding::new("BackTab", FileDialogAction::CompleteBackward),
                FileDialogKeyBinding::new("Ctrl+H", FileDialogAction::ToggleHidden),
                FileDialogKeyBinding::new("Up", FileDialogAction::MoveSelectionUp),
                FileDialogKeyBinding::new("Down", FileDialogAction::MoveSelectionDown),
                FileDialogKeyBinding::new("PageUp", FileDialogAction::PageSelectionUp),
                FileDialogKeyBinding::new("PageDown", FileDialogAction::PageSelectionDown),
                FileDialogKeyBinding::new("Left", FileDialogAction::MoveInputLeft),
                FileDialogKeyBinding::new("Right", FileDialogAction::MoveInputRight),
                FileDialogKeyBinding::new("Home", FileDialogAction::MoveInputStart),
                FileDialogKeyBinding::new("End", FileDialogAction::MoveInputEnd),
                FileDialogKeyBinding::new("Backspace", FileDialogAction::DeleteBackward),
                FileDialogKeyBinding::new("Delete", FileDialogAction::DeleteForward),
            ],
        }
    }

    pub fn validate(&self) -> Result<(), FileDialogKeymapError> {
        let mut seen = HashSet::new();

        for binding in &self.bindings {
            if !seen.insert(binding.stroke) {
                return Err(FileDialogKeymapError::DuplicateBinding(binding.stroke));
            }
        }

        Ok(())
    }

    pub fn action_for_stroke(&self, stroke: KeyStroke) -> Option<FileDialogAction> {
        self.bindings
            .iter()
            .find(|binding| binding.stroke == stroke)
            .map(|binding| binding.action)
    }

    pub fn stroke_for_action(&self, action: FileDialogAction) -> Option<KeyStroke> {
        self.bindings
            .iter()
            .find(|binding| binding.action == action)
            .map(|binding| binding.stroke)
    }

    pub fn set_action_binding(&mut self, action: FileDialogAction, stroke: Option<KeyStroke>) {
        self.bindings.retain(|binding| binding.action != action);
        if let Some(stroke) = stroke {
            self.bindings.push(FileDialogKeyBinding { stroke, action });
        }
    }
}

impl Default for FileDialogKeymap {
    fn default() -> Self {
        Self::default_file_dialog()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FileDialogKeyBinding {
    pub stroke: KeyStroke,
    pub action: FileDialogAction,
}

impl FileDialogKeyBinding {
    pub fn new(stroke: &str, action: FileDialogAction) -> Self {
        Self {
            stroke: KeyStroke::from_str(stroke).expect("default file dialog key should be valid"),
            action,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FileDialogAction {
    Cancel,
    Submit,
    CompleteForward,
    CompleteBackward,
    ToggleHidden,
    MoveSelectionUp,
    MoveSelectionDown,
    PageSelectionUp,
    PageSelectionDown,
    MoveInputLeft,
    MoveInputRight,
    MoveInputStart,
    MoveInputEnd,
    DeleteBackward,
    DeleteForward,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FileDialogKeymapError {
    DuplicateBinding(KeyStroke),
}

pub fn file_dialog_action_id(action: FileDialogAction) -> &'static str {
    match action {
        FileDialogAction::Cancel => "file_dialog.cancel",
        FileDialogAction::Submit => "file_dialog.submit",
        FileDialogAction::CompleteForward => "file_dialog.complete_forward",
        FileDialogAction::CompleteBackward => "file_dialog.complete_backward",
        FileDialogAction::ToggleHidden => "file_dialog.toggle_hidden",
        FileDialogAction::MoveSelectionUp => "file_dialog.move_selection_up",
        FileDialogAction::MoveSelectionDown => "file_dialog.move_selection_down",
        FileDialogAction::PageSelectionUp => "file_dialog.page_selection_up",
        FileDialogAction::PageSelectionDown => "file_dialog.page_selection_down",
        FileDialogAction::MoveInputLeft => "file_dialog.move_input_left",
        FileDialogAction::MoveInputRight => "file_dialog.move_input_right",
        FileDialogAction::MoveInputStart => "file_dialog.move_input_start",
        FileDialogAction::MoveInputEnd => "file_dialog.move_input_end",
        FileDialogAction::DeleteBackward => "file_dialog.delete_backward",
        FileDialogAction::DeleteForward => "file_dialog.delete_forward",
    }
}

pub fn file_dialog_action_from_id(input: &str) -> Result<FileDialogAction, CommandParseError> {
    match normalize_command_id(input).as_str() {
        "cancel" | "file_dialog.cancel" => Ok(FileDialogAction::Cancel),
        "submit" | "file_dialog.submit" => Ok(FileDialogAction::Submit),
        "complete_forward" | "file_dialog.complete_forward" => {
            Ok(FileDialogAction::CompleteForward)
        }
        "complete_backward" | "file_dialog.complete_backward" => {
            Ok(FileDialogAction::CompleteBackward)
        }
        "toggle_hidden" | "file_dialog.toggle_hidden" => Ok(FileDialogAction::ToggleHidden),
        "move_selection_up" | "file_dialog.move_selection_up" => {
            Ok(FileDialogAction::MoveSelectionUp)
        }
        "move_selection_down" | "file_dialog.move_selection_down" => {
            Ok(FileDialogAction::MoveSelectionDown)
        }
        "page_selection_up" | "file_dialog.page_selection_up" => {
            Ok(FileDialogAction::PageSelectionUp)
        }
        "page_selection_down" | "file_dialog.page_selection_down" => {
            Ok(FileDialogAction::PageSelectionDown)
        }
        "move_input_left" | "file_dialog.move_input_left" => Ok(FileDialogAction::MoveInputLeft),
        "move_input_right" | "file_dialog.move_input_right" => Ok(FileDialogAction::MoveInputRight),
        "move_input_start" | "file_dialog.move_input_start" => Ok(FileDialogAction::MoveInputStart),
        "move_input_end" | "file_dialog.move_input_end" => Ok(FileDialogAction::MoveInputEnd),
        "delete_backward" | "file_dialog.delete_backward" => Ok(FileDialogAction::DeleteBackward),
        "delete_forward" | "file_dialog.delete_forward" => Ok(FileDialogAction::DeleteForward),
        _ => Err(CommandParseError::UnknownCommand(input.to_string())),
    }
}
