//! Save As: the prompt that takes the path of another file, confirming a file it would replace,
//! and the switch to that file once the frontend has written the document there, which saving
//! follows from then on (FR-STORE-11), as taskpaper-terminal's Save As does
//! ([file-io.md §2](../../../specs/reference/taskpaper-terminal/specs/file-io.md#2-saving)). The
//! frontend expands the path typed and writes the file, reporting back as [`SaveAsReport`]s.

use std::path::PathBuf;

use super::{Effect, Notice, OnDisk, Opened, State, TextEdit, TextField};

/// How writing the document to another file went, as the frontend reports it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SaveAsReport {
    /// The file was written, at this path, the one typed as the frontend resolves it.
    Saved(PathBuf),
    /// Nothing was written, because a file is at the path and was not to be replaced.
    Exists,
    /// The file could not be written, for this reason.
    Failed(String),
}

/// What the user does in the open Save As prompt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SaveAsInput {
    /// Changes the path typed, going back to it from replacing a file.
    Edit(TextEdit),
    /// Writes the document to the path typed, or over the file there once asked.
    Apply,
    /// Goes back to the path from replacing a file, or closes the prompt.
    Cancel,
}

/// The open prompt as a frontend shows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SaveAsView<'a> {
    /// The path typed.
    pub field: &'a TextField,
    /// What the prompt is doing.
    pub status: SaveAsStatus<'a>,
}

/// What the Save As prompt is doing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SaveAsStatus<'a> {
    /// Taking the path.
    Typing,
    /// Taking the path again after writing failed, for this reason.
    Failed(&'a str),
    /// Asking whether to replace the file at the path.
    Replacing,
    /// Writing the file.
    Writing,
}

/// The open prompt: the path typed and what it is doing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct SaveAs {
    field: TextField,
    stage: Stage,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Stage {
    /// Taking the path, after a failure to write there when there is a reason.
    Typing { failure: Option<String> },
    /// Asking whether to replace the file at the path.
    Replacing,
    /// Writing the document at `revision`, holding back the latest change to the file the
    /// session leaves, which matters again only if writing does not succeed.
    Writing { revision: u64, held: Option<OnDisk> },
}

impl State {
    /// The Save As prompt, while it is open.
    #[must_use]
    pub fn save_as(&self) -> Option<SaveAsView<'_>> {
        let prompt = self.save_as.as_ref()?;
        let status = match &prompt.stage {
            Stage::Typing { failure: None } => SaveAsStatus::Typing,
            Stage::Typing {
                failure: Some(reason),
            } => SaveAsStatus::Failed(reason),
            Stage::Replacing => SaveAsStatus::Replacing,
            Stage::Writing { .. } => SaveAsStatus::Writing,
        };
        Some(SaveAsView {
            field: &prompt.field,
            status,
        })
    }

    /// Opens the prompt with the open file's path typed and selected, so that typing replaces it,
    /// or nothing typed for the demo document.
    pub(super) fn open_save_as(&mut self) {
        let path = match &self.opened {
            Opened::File(path) => path.display().to_string(),
            Opened::Demo => String::new(),
        };
        self.save_as = Some(SaveAs {
            field: TextField::new(path).edited(TextEdit::SelectAll),
            stage: Stage::Typing { failure: None },
        });
    }

    /// Takes in what the user does in the open prompt, nothing while it writes the file, and
    /// returns the effects it asks for.
    pub(super) fn save_as_input(&mut self, input: SaveAsInput) -> Vec<Effect> {
        let revision = self.document.revision();
        let Some(prompt) = self.save_as.as_mut() else {
            return Vec::new();
        };
        let replace = match (input, &prompt.stage) {
            (_, Stage::Writing { .. }) => return Vec::new(),
            (SaveAsInput::Edit(edit), _) => {
                prompt.field = prompt.field.edited(edit);
                prompt.stage = Stage::Typing { failure: None };
                return Vec::new();
            }
            (SaveAsInput::Cancel, Stage::Replacing) => {
                prompt.stage = Stage::Typing { failure: None };
                return Vec::new();
            }
            (SaveAsInput::Cancel, Stage::Typing { .. }) => {
                self.save_as = None;
                return Vec::new();
            }
            (SaveAsInput::Apply, Stage::Typing { .. }) if prompt.field.text().trim().is_empty() => {
                return Vec::new();
            }
            (SaveAsInput::Apply, Stage::Typing { .. }) => false,
            (SaveAsInput::Apply, Stage::Replacing) => true,
        };
        prompt.stage = Stage::Writing {
            revision,
            held: None,
        };
        vec![Effect::SaveAs {
            path: prompt.field.text().to_owned(),
            replace,
        }]
    }

    /// Whether the prompt is writing the file, which saving waits for: until the frontend
    /// reports it, what saving would save to is the file the session leaves.
    pub(super) fn saving_as(&self) -> bool {
        self.save_as
            .as_ref()
            .is_some_and(|prompt| matches!(prompt.stage, Stage::Writing { .. }))
    }

    /// Holds `change` to the file the session leaves back while the prompt writes another,
    /// returning it otherwise.
    pub(super) fn held_back(&mut self, change: OnDisk) -> Option<OnDisk> {
        match self.save_as.as_mut().map(|prompt| &mut prompt.stage) {
            Some(Stage::Writing { held, .. }) => {
                *held = Some(change);
                None
            }
            _ => Some(change),
        }
    }

    /// Takes in how writing the file went, and returns the effects it asks for: written, the
    /// prompt closes and the file is the one saving follows, holding the document as it was when
    /// it was written; otherwise the prompt asks to replace the file there, or takes the path
    /// again saying why writing failed, and a change to the file the session stays on, held back
    /// while writing, is taken in.
    pub(super) fn save_as_report(&mut self, report: SaveAsReport) -> Vec<Effect> {
        let Some(prompt) = self.save_as.as_mut() else {
            return Vec::new();
        };
        let Stage::Writing { revision, held } = &mut prompt.stage else {
            return Vec::new();
        };
        let (revision, held) = (*revision, held.take());
        match report {
            SaveAsReport::Saved(path) => {
                self.save_as = None;
                self.saving.saved_as(revision);
                self.opened = Opened::File(path.clone());
                self.notify(Notice::SavedAs(path));
                return Vec::new();
            }
            SaveAsReport::Exists => prompt.stage = Stage::Replacing,
            SaveAsReport::Failed(reason) => {
                prompt.stage = Stage::Typing {
                    failure: Some(reason),
                };
            }
        }
        held.map_or_else(Vec::new, |change| self.on_disk(change))
    }
}

#[cfg(test)]
mod tests {
    use std::{path::Path, time::Duration};

    use jiff::civil::{Date, date};

    use super::*;
    use crate::{
        app::{Command, Message, OnDisk, Report, autosave::SaveState, update},
        domain::{Document, Item, ItemKind, Node},
    };

    const TODAY: Date = date(2026, 10, 14);
    const FILE: &str = "/notes/todo.taskpaper";
    const NEW: &str = "/notes/new.taskpaper";

    fn document() -> Document {
        let mut document = Document::new();
        let inbox = Some(document.inbox());
        document
            .push(inbox, Node::Item(Item::new(ItemKind::Task, "a")))
            .expect("the Inbox takes a task");
        document
    }

    fn send(state: &mut State, message: Message) -> Vec<Effect> {
        update(state, message, TODAY)
    }

    fn input(state: &mut State, input: SaveAsInput) -> Vec<Effect> {
        send(state, Message::SaveAs(input))
    }

    fn typed(state: &mut State, text: &str) {
        for c in text.chars() {
            assert_eq!(input(state, SaveAsInput::Edit(TextEdit::Insert(c))), []);
        }
    }

    fn report(state: &mut State, report: SaveAsReport) -> Vec<Effect> {
        send(state, Message::Disk(Report::SaveAs(report)))
    }

    fn status(state: &State) -> Option<SaveAsStatus<'_>> {
        state.save_as().map(|view| view.status)
    }

    fn save_as(replace: bool) -> Effect {
        Effect::SaveAs {
            path: NEW.to_owned(),
            replace,
        }
    }

    /// The file [`FILE`] opened, with [`NEW`] typed in the open prompt and applied.
    fn writing() -> State {
        let mut state = State::new(document(), Opened::File(FILE.into()), TODAY);
        send(&mut state, Message::Command(Command::SaveAs));
        typed(&mut state, NEW);
        assert_eq!(input(&mut state, SaveAsInput::Apply), [save_as(false)]);
        assert_eq!(status(&state), Some(SaveAsStatus::Writing));
        state
    }

    // FR-STORE-11
    #[test]
    fn opens_with_the_files_path_selected_and_nothing_for_the_demo() {
        let mut state = State::new(document(), Opened::File(FILE.into()), TODAY);
        assert_eq!(state.save_as(), None);
        assert!(state.takes(Command::SaveAs));
        send(&mut state, Message::Command(Command::SaveAs));
        let view = state.save_as().expect("the prompt opens");
        assert_eq!(view.field.text(), FILE);
        assert_eq!(view.field.selection(), Some(0..FILE.len()));
        assert_eq!(view.status, SaveAsStatus::Typing);
        assert!(!state.takes(Command::SaveAs), "it has the keys");
        assert!(state.takes(Command::Quit));
        typed(&mut state, "~/a");
        assert_eq!(state.save_as().map(|view| view.field.text()), Some("~/a"));
        let mut demo = State::new(document(), Opened::Demo, TODAY);
        send(&mut demo, Message::Command(Command::SaveAs));
        assert_eq!(demo.save_as().map(|view| view.field.text()), Some(""));
        assert_eq!(
            input(&mut demo, SaveAsInput::Apply),
            [],
            "nothing to save to"
        );
        typed(&mut demo, "  ");
        assert_eq!(input(&mut demo, SaveAsInput::Apply), [], "nor here");
        assert_eq!(input(&mut demo, SaveAsInput::Cancel), []);
        assert_eq!(demo.save_as(), None);
        assert_eq!(input(&mut demo, SaveAsInput::Apply), [], "no prompt");
    }

    // FR-STORE-11
    #[test]
    fn a_file_there_is_replaced_only_once_asked() {
        let mut state = writing();
        assert_eq!(report(&mut state, SaveAsReport::Exists), []);
        assert_eq!(status(&state), Some(SaveAsStatus::Replacing));
        assert_eq!(input(&mut state, SaveAsInput::Cancel), []);
        assert_eq!(
            status(&state),
            Some(SaveAsStatus::Typing),
            "back to the path"
        );
        assert_eq!(input(&mut state, SaveAsInput::Apply), [save_as(false)]);
        report(&mut state, SaveAsReport::Exists);
        typed(&mut state, "2");
        assert_eq!(status(&state), Some(SaveAsStatus::Typing), "another path");
        input(&mut state, SaveAsInput::Edit(TextEdit::DeleteBackward));
        assert_eq!(input(&mut state, SaveAsInput::Apply), [save_as(false)]);
        report(&mut state, SaveAsReport::Exists);
        assert_eq!(input(&mut state, SaveAsInput::Apply), [save_as(true)]);
        assert_eq!(status(&state), Some(SaveAsStatus::Writing));
    }

    // FR-STORE-11
    #[test]
    fn a_failure_takes_the_path_again_saying_why() {
        let mut state = writing();
        let reason = "the folder /notes does not exist";
        assert_eq!(
            report(&mut state, SaveAsReport::Failed(reason.to_owned())),
            []
        );
        assert_eq!(status(&state), Some(SaveAsStatus::Failed(reason)));
        assert_eq!(state.opened(), &Opened::File(FILE.into()));
        typed(&mut state, "x");
        assert_eq!(status(&state), Some(SaveAsStatus::Typing));
    }

    // FR-STORE-11, FR-STORE-04: a change to the file left unwritten over is taken in, the latest.
    #[test]
    fn a_change_on_disk_while_writing_waits_until_the_file_stays_open() {
        let mut state = writing();
        let theirs = Document::new();
        for document in [document(), theirs.clone()] {
            assert_eq!(
                send(&mut state, Message::OnDisk(OnDisk::Changed(document))),
                []
            );
        }
        assert_eq!(state.document(), &document(), "not yet");
        assert_eq!(
            report(&mut state, SaveAsReport::Exists),
            [Effect::Reloaded],
            "the document was saved, so reloaded"
        );
        assert_eq!(state.document(), &theirs);
        input(&mut state, SaveAsInput::Apply);
        assert_eq!(
            report(&mut state, SaveAsReport::Failed("full".to_owned())),
            [],
            "nothing held this time"
        );
    }

    // FR-STORE-11
    #[test]
    fn writing_ignores_keys_and_waits_to_be_reported() {
        let mut state = writing();
        assert_eq!(
            input(&mut state, SaveAsInput::Edit(TextEdit::Insert('x'))),
            []
        );
        assert_eq!(input(&mut state, SaveAsInput::Cancel), []);
        assert_eq!(input(&mut state, SaveAsInput::Apply), []);
        assert_eq!(state.save_as().map(|view| view.field.text()), Some(NEW));
        send(&mut state, Message::Command(Command::Navigator));
        assert_eq!(
            status(&state),
            Some(SaveAsStatus::Writing),
            "the Navigator leaves it"
        );
        send(&mut state, Message::Command(Command::Navigator));
        assert_eq!(
            report(&mut state, SaveAsReport::Saved(NEW.into())),
            [],
            "nothing to save"
        );
        assert_eq!(state.opened(), &Opened::File(NEW.into()));
        send(&mut state, Message::Command(Command::SaveAs));
        send(&mut state, Message::Command(Command::Navigator));
        assert_eq!(state.save_as(), None, "the Navigator closes it otherwise");
        assert_eq!(
            report(&mut state, SaveAsReport::Saved(FILE.into())),
            [],
            "a report without a prompt changes nothing"
        );
        assert_eq!(state.opened(), &Opened::File(NEW.into()));
        send(&mut state, Message::Command(Command::SaveAs));
        assert_eq!(
            report(&mut state, SaveAsReport::Exists),
            [],
            "nor one before writing"
        );
        assert_eq!(status(&state), Some(SaveAsStatus::Typing));
    }

    // FR-STORE-11
    #[test]
    fn once_written_the_new_file_is_saved_to_and_the_old_one_left() {
        let mut state = writing();
        send(&mut state, Message::Command(Command::ToggleCompleted));
        assert_eq!(
            send(&mut state, Message::Tick(Duration::from_secs(5))),
            [],
            "the old file is no longer saved to"
        );
        assert_eq!(
            send(
                &mut state,
                Message::OnDisk(OnDisk::Changed(Document::new()))
            ),
            [],
            "nor reloaded"
        );
        assert_eq!(report(&mut state, SaveAsReport::Saved(NEW.into())), []);
        assert_eq!(state.save_as(), None);
        assert_eq!(state.opened(), &Opened::File(NEW.into()));
        assert_eq!(state.notice(), Some(&Notice::SavedAs(NEW.into())));
        assert_eq!(
            state.save_state(),
            SaveState::Unsaved,
            "the change after it"
        );
        assert_eq!(
            send(&mut state, Message::Tick(Duration::from_secs(6))),
            [Effect::Save]
        );
        send(&mut state, Message::Disk(Report::Saved));
        assert_eq!(state.save_state(), SaveState::Saved);
    }

    // FR-STORE-11, FR-STORE-01: the demo document is saved once it has a file.
    #[test]
    fn the_demo_is_saved_once_written_and_a_quit_waits_for_it() {
        let mut demo = State::new(document(), Opened::Demo, TODAY);
        send(&mut demo, Message::Command(Command::ToggleCompleted));
        send(&mut demo, Message::Command(Command::SaveAs));
        typed(&mut demo, NEW);
        assert_eq!(input(&mut demo, SaveAsInput::Apply), [save_as(false)]);
        assert_eq!(
            send(&mut demo, Message::Command(Command::Quit)),
            [],
            "waits"
        );
        assert_eq!(
            report(&mut demo, SaveAsReport::Saved(Path::new(NEW).into())),
            [Effect::Quit]
        );
        assert_eq!(demo.save_state(), SaveState::Saved);
    }
}
