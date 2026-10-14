//! The clipboard the app copies items to and pastes them from, as TaskPaper text (FR-OPS-05,
//! [editing.md §4](../../../specs/reference/taskpaper-terminal/specs/editing.md#4-structural-operations-fr-ops)).

use std::fmt::Debug;

use crate::{domain::Branch, store::taskpaper};

/// The system clipboard as the app reaches it. `main.rs` gives the real one, so that tests never
/// touch it.
pub trait SystemClipboard: Debug {
    /// Puts `text` on the clipboard, as well as it can.
    fn set(&mut self, text: &str);
    /// The text on the clipboard, when it can be read and holds text.
    fn get(&mut self) -> Option<String>;
}

/// The system clipboard when there is one, and the text the app copied last, which pastes when
/// the system clipboard cannot be read.
#[derive(Debug, Default)]
pub(super) struct Clipboard {
    system: Option<Box<dyn SystemClipboard>>,
    copied: Option<String>,
}

impl Clipboard {
    /// The clipboard reaching `system`.
    pub(super) fn new(system: Box<dyn SystemClipboard>) -> Self {
        Self {
            system: Some(system),
            copied: None,
        }
    }

    /// Puts `branches` on the clipboard as TaskPaper text.
    pub(super) fn copy(&mut self, branches: &[Branch]) {
        let text = taskpaper::write_branches(branches);
        if let Some(system) = &mut self.system {
            system.set(&text);
        }
        self.copied = Some(text);
    }

    /// The text the clipboard holds: the system clipboard's, else the text the app copied last.
    pub(super) fn text(&mut self) -> Option<String> {
        self.system
            .as_mut()
            .and_then(|system| system.get())
            .or_else(|| self.copied.clone())
    }

    /// The items the clipboard's [`text`](Self::text) holds, read as checklist items, one for
    /// each line, for `checklist`.
    pub(super) fn branches(&mut self, checklist: bool) -> Vec<Branch> {
        let read = if checklist {
            taskpaper::read_checklist
        } else {
            taskpaper::read_branches
        };
        self.text().map(|text| read(&text)).unwrap_or_default()
    }
}
