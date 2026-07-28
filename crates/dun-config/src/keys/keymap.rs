use std::collections::HashMap;
use std::str::FromStr;

use dun_core::{AppCommand, EditCommand, EditorCommand, FileCommand, WindowCommand};

use super::{KeyParseError, KeySequence, KeyStroke};
use crate::command_id;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Keymap {
    pub bindings: Vec<KeyBinding>,
}

impl Keymap {
    pub fn new(bindings: Vec<KeyBinding>) -> Result<Self, KeymapError> {
        let keymap = Self { bindings };
        keymap.validate()?;
        Ok(keymap)
    }

    pub fn empty() -> Self {
        Self {
            bindings: Vec::new(),
        }
    }

    pub fn default_editor() -> Self {
        Self {
            bindings: vec![
                KeyBinding::new("F1", EditorCommand::App(AppCommand::Help)),
                KeyBinding::new("F2", EditorCommand::App(AppCommand::StatusHistory)),
                KeyBinding::new("F5", EditorCommand::App(AppCommand::ReloadConfig)),
                KeyBinding::new("F6", EditorCommand::App(AppCommand::ConfigDiagnostics)),
                KeyBinding::new("Ctrl+Q", EditorCommand::App(AppCommand::Quit)),
                KeyBinding::new("Ctrl+P", EditorCommand::App(AppCommand::CommandLine)),
                KeyBinding::new("Ctrl+X,O", EditorCommand::App(AppCommand::RunCommand)),
                KeyBinding::new("Ctrl+X,S", EditorCommand::App(AppCommand::ShellEscape)),
                KeyBinding::new("Ctrl+N", EditorCommand::File(FileCommand::New)),
                KeyBinding::new("Ctrl+O", EditorCommand::File(FileCommand::Open)),
                KeyBinding::new("Ctrl+X,B", EditorCommand::File(FileCommand::SwitchBuffer)),
                KeyBinding::new("Ctrl+S", EditorCommand::File(FileCommand::Save)),
                KeyBinding::new("Ctrl+Shift+S", EditorCommand::File(FileCommand::SaveAs)),
                KeyBinding::new("Ctrl+X,E", EditorCommand::File(FileCommand::Reload)),
                KeyBinding::new("Ctrl+X,Q", EditorCommand::File(FileCommand::Close)),
                KeyBinding::new("Ctrl+Z", EditorCommand::Edit(EditCommand::Undo)),
                KeyBinding::new("Ctrl+Y", EditorCommand::Edit(EditCommand::Redo)),
                KeyBinding::new(
                    "Ctrl+X,Ctrl+C",
                    EditorCommand::Edit(EditCommand::CopyExternal),
                ),
                KeyBinding::new(
                    "Ctrl+X,Ctrl+V",
                    EditorCommand::Edit(EditCommand::PasteExternal),
                ),
                KeyBinding::new("Ctrl+W", EditorCommand::Edit(EditCommand::Find)),
                KeyBinding::new("F3", EditorCommand::Edit(EditCommand::FindNext)),
                KeyBinding::new("Shift+F3", EditorCommand::Edit(EditCommand::FindPrevious)),
                KeyBinding::new("Ctrl+R", EditorCommand::Edit(EditCommand::Replace)),
                KeyBinding::new("Ctrl+G", EditorCommand::Edit(EditCommand::GoToLine)),
                KeyBinding::new("Ctrl+A", EditorCommand::Edit(EditCommand::SelectAll)),
                KeyBinding::new("Ctrl+L", EditorCommand::Edit(EditCommand::SelectLine)),
                KeyBinding::new("Ctrl+X,Y", EditorCommand::Edit(EditCommand::CopyLine)),
                KeyBinding::new("Ctrl+K", EditorCommand::Edit(EditCommand::DeleteLine)),
                KeyBinding::new("Ctrl+X,U", EditorCommand::Edit(EditCommand::MoveLineUp)),
                KeyBinding::new("Ctrl+X,J", EditorCommand::Edit(EditCommand::MoveLineDown)),
                KeyBinding::new("Tab", EditorCommand::Edit(EditCommand::IndentLine)),
                KeyBinding::new("BackTab", EditorCommand::Edit(EditCommand::OutdentLine)),
                KeyBinding::new(
                    "Ctrl+X,T",
                    EditorCommand::Edit(EditCommand::TrimTrailingWhitespace),
                ),
                KeyBinding::new("Ctrl+X,Z", EditorCommand::Edit(EditCommand::ToggleWordWrap)),
                KeyBinding::new(
                    "Ctrl+X,.",
                    EditorCommand::Edit(EditCommand::ToggleVisibleWhitespace),
                ),
                KeyBinding::new("Ctrl+X,F", EditorCommand::Edit(EditCommand::ToggleFold)),
                KeyBinding::new("Ctrl+X,A", EditorCommand::Edit(EditCommand::UnfoldAll)),
                KeyBinding::new("Ctrl+X,K", EditorCommand::Edit(EditCommand::ToggleBookmark)),
                KeyBinding::new("Ctrl+X,N", EditorCommand::Edit(EditCommand::NextBookmark)),
                KeyBinding::new(
                    "Ctrl+X,L",
                    EditorCommand::Edit(EditCommand::PreviousBookmark),
                ),
                KeyBinding::new("Left", EditorCommand::Edit(EditCommand::MoveLeft)),
                KeyBinding::new("Right", EditorCommand::Edit(EditCommand::MoveRight)),
                KeyBinding::new("Up", EditorCommand::Edit(EditCommand::MoveUp)),
                KeyBinding::new("Down", EditorCommand::Edit(EditCommand::MoveDown)),
                KeyBinding::new("PageUp", EditorCommand::Edit(EditCommand::MovePageUp)),
                KeyBinding::new("PageDown", EditorCommand::Edit(EditCommand::MovePageDown)),
                // vi-style paging for keyboards without PageUp/PageDown keys.
                KeyBinding::new("Ctrl+B", EditorCommand::Edit(EditCommand::MovePageUp)),
                KeyBinding::new("Ctrl+F", EditorCommand::Edit(EditCommand::MovePageDown)),
                KeyBinding::new(
                    "Ctrl+Home",
                    EditorCommand::Edit(EditCommand::MoveDocumentStart),
                ),
                KeyBinding::new(
                    "Ctrl+End",
                    EditorCommand::Edit(EditCommand::MoveDocumentEnd),
                ),
                KeyBinding::new("Ctrl+X,[", EditorCommand::Edit(EditCommand::ScrollLeft)),
                KeyBinding::new("Ctrl+X,]", EditorCommand::Edit(EditCommand::ScrollRight)),
                KeyBinding::new(
                    "Shift+PageUp",
                    EditorCommand::Edit(EditCommand::ExtendSelectionPageUp),
                ),
                KeyBinding::new(
                    "Shift+PageDown",
                    EditorCommand::Edit(EditCommand::ExtendSelectionPageDown),
                ),
                KeyBinding::new("Ctrl+Left", EditorCommand::Edit(EditCommand::MoveWordLeft)),
                KeyBinding::new(
                    "Ctrl+Right",
                    EditorCommand::Edit(EditCommand::MoveWordRight),
                ),
                KeyBinding::new(
                    "Ctrl+Shift+Left",
                    EditorCommand::Edit(EditCommand::ExtendSelectionWordLeft),
                ),
                KeyBinding::new(
                    "Ctrl+Shift+Right",
                    EditorCommand::Edit(EditCommand::ExtendSelectionWordRight),
                ),
                KeyBinding::new("Home", EditorCommand::Edit(EditCommand::MoveLineStart)),
                KeyBinding::new("End", EditorCommand::Edit(EditCommand::MoveLineEnd)),
                KeyBinding::new("Enter", EditorCommand::Edit(EditCommand::InsertNewline)),
                KeyBinding::new(
                    "Backspace",
                    EditorCommand::Edit(EditCommand::DeleteBackward),
                ),
                KeyBinding::new("Delete", EditorCommand::Edit(EditCommand::DeleteForward)),
                KeyBinding::new(
                    "Ctrl+Backspace",
                    EditorCommand::Edit(EditCommand::DeleteWordBackward),
                ),
                KeyBinding::new(
                    "Ctrl+Delete",
                    EditorCommand::Edit(EditCommand::DeleteWordForward),
                ),
                KeyBinding::new(
                    "Ctrl+X,H",
                    EditorCommand::Window(WindowCommand::SplitHorizontal),
                ),
                KeyBinding::new(
                    "Ctrl+X,V",
                    EditorCommand::Window(WindowCommand::SplitVertical),
                ),
                KeyBinding::new(
                    "Ctrl+X,Left",
                    EditorCommand::Window(WindowCommand::FocusLeft),
                ),
                KeyBinding::new(
                    "Ctrl+X,Right",
                    EditorCommand::Window(WindowCommand::FocusRight),
                ),
                KeyBinding::new("Ctrl+X,Up", EditorCommand::Window(WindowCommand::FocusUp)),
                KeyBinding::new(
                    "Ctrl+X,Down",
                    EditorCommand::Window(WindowCommand::FocusDown),
                ),
                KeyBinding::new("Alt+Left", EditorCommand::Window(WindowCommand::FocusLeft)),
                KeyBinding::new(
                    "Alt+Right",
                    EditorCommand::Window(WindowCommand::FocusRight),
                ),
                KeyBinding::new("Alt+Up", EditorCommand::Window(WindowCommand::FocusUp)),
                KeyBinding::new("Alt+Down", EditorCommand::Window(WindowCommand::FocusDown)),
                KeyBinding::new(
                    "Ctrl+X,Shift+Left",
                    EditorCommand::Window(WindowCommand::ResizeLeft),
                ),
                KeyBinding::new(
                    "Ctrl+X,Shift+Right",
                    EditorCommand::Window(WindowCommand::ResizeRight),
                ),
                KeyBinding::new(
                    "Ctrl+X,Shift+Up",
                    EditorCommand::Window(WindowCommand::ResizeUp),
                ),
                KeyBinding::new(
                    "Ctrl+X,Shift+Down",
                    EditorCommand::Window(WindowCommand::ResizeDown),
                ),
                KeyBinding::new(
                    "Alt+Shift+Left",
                    EditorCommand::Window(WindowCommand::ResizeLeft),
                ),
                KeyBinding::new(
                    "Alt+Shift+Right",
                    EditorCommand::Window(WindowCommand::ResizeRight),
                ),
                KeyBinding::new(
                    "Alt+Shift+Up",
                    EditorCommand::Window(WindowCommand::ResizeUp),
                ),
                KeyBinding::new(
                    "Alt+Shift+Down",
                    EditorCommand::Window(WindowCommand::ResizeDown),
                ),
                KeyBinding::new("Ctrl+X,=", EditorCommand::Window(WindowCommand::Equalize)),
                KeyBinding::new(
                    "Ctrl+X,R",
                    EditorCommand::Window(WindowCommand::RotateSplit),
                ),
                KeyBinding::new(
                    "Ctrl+X,C",
                    EditorCommand::Window(WindowCommand::ToggleCollapse),
                ),
                KeyBinding::new("Ctrl+X,1", EditorCommand::Window(WindowCommand::Only)),
                KeyBinding::new("Ctrl+X,M", EditorCommand::Window(WindowCommand::Collapse)),
                KeyBinding::new("Ctrl+X,P", EditorCommand::Window(WindowCommand::Expand)),
                KeyBinding::new("Ctrl+X,X", EditorCommand::Window(WindowCommand::Close)),
            ],
        }
    }

    pub fn validate(&self) -> Result<(), KeymapError> {
        let mut seen: HashMap<&KeySequence, &EditorCommand> = HashMap::new();

        for binding in &self.bindings {
            if binding.sequence.strokes.is_empty() {
                return Err(KeymapError::EmptySequence);
            }

            // Report both claimants: a conflict is usually a config binding
            // landing on a default the user never wrote down.
            if let Some(existing) = seen.insert(&binding.sequence, &binding.command) {
                return Err(KeymapError::DuplicateBinding {
                    sequence: binding.sequence.clone(),
                    bound: command_id(existing),
                    rebound: command_id(&binding.command),
                });
            }
        }

        Ok(())
    }

    pub fn command_for_sequence(&self, sequence: &KeySequence) -> Option<&EditorCommand> {
        self.bindings
            .iter()
            .find(|binding| &binding.sequence == sequence)
            .map(|binding| &binding.command)
    }

    pub fn command_for_stroke(&self, stroke: KeyStroke) -> Option<&EditorCommand> {
        self.command_for_sequence(&KeySequence::single(stroke))
    }

    pub fn sequence_for_command(&self, command: &EditorCommand) -> Option<&KeySequence> {
        self.bindings
            .iter()
            .find(|binding| &binding.command == command)
            .map(|binding| &binding.sequence)
    }

    pub fn has_sequence_prefix(&self, sequence: &KeySequence) -> bool {
        if sequence.strokes.is_empty() {
            return false;
        }

        self.bindings.iter().any(|binding| {
            binding.sequence.strokes.len() > sequence.strokes.len()
                && binding.sequence.strokes.starts_with(&sequence.strokes)
        })
    }

    pub fn set_command_binding(&mut self, command: EditorCommand, sequence: Option<KeySequence>) {
        self.bindings.retain(|binding| binding.command != command);
        if let Some(sequence) = sequence {
            self.bindings.push(KeyBinding { sequence, command });
        }
    }
}

impl Default for Keymap {
    fn default() -> Self {
        Self::default_editor()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeyBinding {
    pub sequence: KeySequence,
    pub command: EditorCommand,
}

impl KeyBinding {
    pub fn new(sequence: &str, command: EditorCommand) -> Self {
        Self {
            sequence: KeySequence::parse_lossy(sequence),
            command,
        }
    }

    pub fn try_new(sequence: &str, command: EditorCommand) -> Result<Self, KeyParseError> {
        Ok(Self {
            sequence: KeySequence::from_str(sequence)?,
            command,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum KeymapError {
    /// One key sequence claimed by two commands. `bound` already held it
    /// (often a default the user never wrote), `rebound` tried to take it.
    DuplicateBinding {
        sequence: KeySequence,
        bound: &'static str,
        rebound: &'static str,
    },
    EmptySequence,
}
