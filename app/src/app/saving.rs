//! Keeping the opened file and the document in step: saving every change through the
//! [`Autosave`] policy (FR-STORE-09, F-FILE-05), and what a change another program made to the
//! file does: a silent reload, or the conflict prompt with its conflict copies when the document
//! holds changes the file lacks (F-FILE-06,
//! [file-io.md §3](../../../specs/reference/taskpaper-terminal/specs/file-io.md#3-external-changes)).
//! The core only decides; the frontend reads, writes and copies the file and reports back.

use std::{path::PathBuf, time::Duration};

use jiff::civil::Date;

use super::{
    Effect, Refusal, SaveAsReport, State, attachments,
    autosave::{Autosave, Ending, SaveState, Step},
    go_to_symbol,
    move_to::MoveTo,
    navigator, text_path, view,
};
use crate::domain::Document;

/// What the frontend reports of disk work the core asked for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Report {
    /// The save asked for was written.
    Saved,
    /// The save failed, for this reason.
    SaveFailed(String),
    /// The save was not done, because the file on disk was no longer the version the core knows;
    /// what is there now came just before as [`OnDisk`].
    SaveRefused,
    /// The daily backup failed, for this reason; saving goes on (F-FILE-09).
    BackupFailed(String),
    /// This conflict copy asked for was written here.
    Copied(CopyId, PathBuf),
    /// This conflict copy asked for could not be written, for this reason.
    CopyFailed(CopyId, String),
    /// How writing the document to another file went, as Save As asked (FR-STORE-11).
    SaveAs(SaveAsReport),
}

/// Which conflict copy a [`Report`] tells of, as [`Effect::Copy`] asked for it, so that the
/// report of a copy whose choice no longer stands is not taken for a later one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CopyId(u64);

impl CopyId {
    /// Moves on to the copy after this one, and names it.
    fn next(&mut self) -> Self {
        self.0 += 1;
        *self
    }
}

/// What the file on disk turned out to hold when it was no longer the version the core last read
/// or wrote.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OnDisk {
    /// The file now holds this document.
    Changed(Document),
    /// The file now holds bytes that are not UTF-8, so it cannot be read (F-FILE-07).
    Unreadable,
    /// The file was deleted or moved away; the next save creates it again.
    Gone,
}

/// Which version of the file a conflict copy keeps.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConflictSide {
    /// The document as the app holds it.
    Mine,
    /// The file as another program left it on disk.
    Theirs,
}

/// Something that happened to the file or a command, for the status line to say until the next
/// command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Notice {
    /// The file changed on disk and was read again.
    Reloaded,
    /// Keep mine overwrote the file; theirs is in this conflict copy.
    KeptMine(PathBuf),
    /// Take theirs reloaded the file; mine is in this conflict copy.
    TookTheirs(PathBuf),
    /// Take theirs was refused: the file on disk is not UTF-8.
    CannotTakeTheirs,
    /// A conflict copy failed, for this reason, so its choice did not go ahead; after a quit,
    /// quitting again ends without saving.
    CopyFailed(String),
    /// The daily backup failed, for this reason.
    BackupFailed(String),
    /// Empty Trash asks to be given again to delete these many items for good (FR-VIEW-04).
    EmptyTrash(usize),
    /// Completing asks to be given again to complete projects that hold these many open tasks
    /// (FR-ORG-09, D-140).
    OpenTasks(usize),
    /// Indenting, outdenting, converting, grouping or moving was refused, for this reason
    /// (FR-OPS-03, FR-TASK-06, FR-OPS-07, FR-OPS-08, D-107).
    Refused(Refusal),
    /// The checklist was asked for on an item that has none: only tasks have one (FR-ATT-02).
    NoChecklist,
    /// A date was asked for on an area, which has none (FR-DATE-01, D-75).
    AreaUndated,
    /// Tags were asked for on an area, which has none (FR-TAG-01, D-75).
    AreaUntagged,
    /// Completing was asked for on an area, which is not completed (FR-ORG-09, D-81).
    AreaUncompleted,
    /// Later was asked for on areas or sections, which are never Later (FR-ORG-11, D-89).
    NotLater,
    /// Pinning was asked for on items none of which is a project, which only is pinned
    /// (FR-VIEW-12).
    NotAProject,
    /// Flagging was asked for on items none of which is a project or a task, which only are
    /// flagged (FR-ORG-12).
    NotFlaggable,
    /// Typing `@` at the start of a word of a title was refused: the word would read back as a
    /// tag, and tags go on from the tag picker (FR-TAG-02).
    TagInTitle,
    /// Something went wrong with the settings file, as the frontend said it (FR-SET-04).
    Settings(String),
    /// The weather was asked for in summer, which has none (FR-UI-08).
    NoWeather,
    /// The document was written to the file at this path, which saving follows from now on
    /// (FR-STORE-11).
    SavedAs(PathBuf),
    /// Something other than restoring or deleting was asked for in the Trash (FR-VIEW-04,
    /// D-113).
    InTrash,
    /// Quick Add added a task to the place with this path (FR-EDIT-07).
    QuickAdded(String),
    /// Quick Comment added a comment (FR-ATT-06).
    Commented,
    /// Quick Comment was asked for on a view, which takes no comments (FR-ATT-06).
    NoComments,
    /// Pasting in Today was refused: the clipboard holds something other than tasks (D-134).
    OnlyTasksInToday,
    /// Restoring put an item back inside the completed item with this title, so that no view
    /// shows it (FR-VIEW-04, FR-ORG-09, D-135).
    RestoredIntoCompleted(String),
    /// Restoring put these many items back inside completed items, so that no view shows them
    /// (FR-VIEW-04, FR-ORG-09, D-135).
    RestoredIntoCompletedItems(usize),
}

impl Notice {
    /// Whether it says that what was asked for was refused, rather than what happened or that
    /// there was nothing to act on (FR-UI-07).
    fn refuses(&self) -> bool {
        matches!(
            self,
            Self::Refused(_)
                | Self::TagInTitle
                | Self::CannotTakeTheirs
                | Self::AreaUndated
                | Self::AreaUntagged
                | Self::AreaUncompleted
                | Self::NotLater
                | Self::NotAProject
                | Self::InTrash
                | Self::OnlyTasksInToday
        )
    }
}

/// The conflict prompt as a frontend shows it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConflictView {
    /// Whether the document holds changes the file lacks; without them the prompt is open only
    /// because the file on disk cannot be read.
    pub unsaved: bool,
    /// Whether the file on disk can be read, so Take theirs can take it.
    pub readable: bool,
    /// Whether the diff from the file on disk to the document is shown.
    pub diff: bool,
}

/// The saving state of one opened document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Saving {
    autosave: Autosave,
    /// The frontend's clock, as it last told it.
    now: Duration,
    conflict: Option<Conflict>,
    gone: bool,
    notice: Option<Notice>,
    /// Whether the message being handled was refused, as a notice said.
    refused: bool,
    /// Where the document went when the session ended with the conflict prompt open.
    left_in: Option<PathBuf>,
    /// The conflict copy asked for last.
    last_copy: CopyId,
}

/// The file changed on disk while the document held changes it lacks, or became unreadable.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Conflict {
    /// The file on disk, `None` when it cannot be read.
    theirs: Option<Document>,
    diff: bool,
    /// The choice waiting for its conflict copy, and that copy.
    choosing: Option<(CopyId, Choice)>,
    /// Whether ending failed to keep the document, so quitting again ends without it.
    confirm_quit: bool,
}

/// How a conflict is resolved, each once the version it discards is in a conflict copy.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Choice {
    /// Overwrite the file with the document.
    KeepMine,
    /// Reload the file, holding this document, discarding the document's changes.
    TakeTheirs(Document),
    /// End the session this way, leaving the file on disk as it is.
    Leave(Ending),
}

impl Saving {
    /// Saving for a document at `revision` that the file holds, or, when not `saved`, one it
    /// lacks, as one recovered after a crash; nothing is ever saved unless `saveable`.
    pub(super) fn new(revision: u64, saved: bool, saveable: bool) -> Self {
        Self {
            autosave: if saved {
                Autosave::new(revision, saveable)
            } else {
                Autosave::unsaved(revision, saveable)
            },
            now: Duration::ZERO,
            conflict: None,
            gone: false,
            notice: None,
            refused: false,
            left_in: None,
            last_copy: CopyId(0),
        }
    }

    /// The document at `revision` was written to another file, which is the one saved from now
    /// on, holding that revision (FR-STORE-11).
    pub(super) fn saved_as(&mut self, revision: u64) {
        self.autosave.saved_as(revision);
        self.gone = false;
    }

    /// The document went from `from` to `to` without changing, as an edit session that changes
    /// nothing does.
    pub(super) fn unchanged(&mut self, from: u64, to: u64) {
        self.autosave.unchanged(from, to);
    }
}

impl State {
    /// The app showing `document`, recovered after a crash, as [`new`](Self::new) shows it, but
    /// holding changes the file lacks, which are saved over it like any change (N-REL-01).
    #[must_use]
    pub fn recovered(document: Document, opened: super::Opened, today: Date) -> Self {
        let saveable = matches!(opened, super::Opened::File(_));
        let revision = document.revision();
        let mut state = Self::new(document, opened, today);
        state.saving = Saving::new(revision, false, saveable);
        state
    }

    /// What saving asks of the frontend now that a message changed the state: nothing while the
    /// conflict prompt is open, since a save would be refused again at once, nor in edit mode,
    /// whose title is not a change until editing ends, nor while Save As writes another file,
    /// which saving follows once it is written.
    pub(super) fn step_saving(&mut self) -> Vec<Effect> {
        if self.saving.conflict.is_some() || self.editing().is_some() || self.saving_as() {
            return Vec::new();
        }
        let saving = &mut self.saving;
        match saving.autosave.step(self.document.revision(), saving.now) {
            Step::Wait => Vec::new(),
            Step::Save(_) => vec![Effect::Save],
            Step::End => vec![Effect::Quit],
            Step::Recover => vec![Effect::Recover, Effect::Quit],
        }
    }

    /// The frontend's clock now reads `now`.
    pub(super) fn tick(&mut self, now: Duration) {
        self.saving.now = now;
    }

    /// Clears what the status line said, as each command does.
    pub(super) fn clear_notice(&mut self) {
        self.saving.notice = None;
    }

    /// Has the status line say `notice` until the next command.
    pub(super) fn notify(&mut self, notice: Notice) {
        self.saving.refused |= notice.refuses();
        self.saving.notice = Some(notice);
    }

    /// Forgets that the message before was refused, as each message starts.
    pub(super) fn forget_refusal(&mut self) {
        self.saving.refused = false;
    }

    /// Asks to end the session, saving first. With the conflict prompt open, the file on disk is
    /// left as it is and the document goes to a conflict copy instead, so that neither is lost,
    /// in place of a choice waiting for its copy; a quit after that copy failed ends without it.
    /// An edit under way is kept first, as is the edit dialog's draft (D-109).
    pub(super) fn end(&mut self, ending: Ending) -> Vec<Effect> {
        self.finish_editing();
        self.keep_edit_dialog();
        let saving = &mut self.saving;
        let Some(conflict) = &mut saving.conflict else {
            saving.autosave.end(ending);
            return Vec::new();
        };
        match (ending, &conflict.choosing) {
            (Ending::Quit, _) if conflict.confirm_quit => vec![Effect::Quit],
            (Ending::Quit, Some((_, Choice::Leave(_))))
            | (Ending::Terminate, Some((_, Choice::Leave(Ending::Terminate)))) => Vec::new(),
            _ => {
                let id = saving.last_copy.next();
                conflict.choosing = Some((id, Choice::Leave(ending)));
                vec![Effect::Copy {
                    side: ConflictSide::Mine,
                    ending: true,
                    id,
                }]
            }
        }
    }

    /// Takes in how disk work went.
    pub(super) fn disk_report(&mut self, report: Report) -> Vec<Effect> {
        let saving = &mut self.saving;
        match report {
            Report::Saved => {
                saving.autosave.saved();
                saving.gone = false;
            }
            Report::SaveFailed(reason) => saving.autosave.failed(reason),
            Report::SaveRefused => saving.autosave.refused(),
            Report::BackupFailed(reason) => saving.notice = Some(Notice::BackupFailed(reason)),
            Report::Copied(id, copy) => return self.copied(id, copy),
            Report::SaveAs(report) => return self.save_as_report(report),
            Report::CopyFailed(id, reason) => {
                let Some(conflict) = &mut saving.conflict else {
                    return Vec::new();
                };
                let Some((_, choice)) = conflict.choosing.take_if(|(asked, _)| *asked == id) else {
                    return Vec::new();
                };
                match choice {
                    Choice::Leave(Ending::Terminate) => return vec![Effect::Quit],
                    Choice::Leave(Ending::Quit) => conflict.confirm_quit = true,
                    Choice::KeepMine | Choice::TakeTheirs(_) => {}
                }
                saving.notice = Some(Notice::CopyFailed(reason));
            }
        }
        Vec::new()
    }

    /// The conflict copy `id` was written to `copy`, so its choice goes ahead if it still stands.
    /// Leaving keeps the prompt open as the session ends, so that nothing saves mine over theirs.
    fn copied(&mut self, id: CopyId, copy: PathBuf) -> Vec<Effect> {
        let Some((_, choice)) = self
            .saving
            .conflict
            .as_mut()
            .and_then(|conflict| conflict.choosing.take_if(|(asked, _)| *asked == id))
        else {
            return Vec::new();
        };
        if !matches!(choice, Choice::Leave(_)) {
            self.saving.conflict = None;
        }
        match choice {
            Choice::KeepMine => {
                self.saving.autosave.overwrite();
                self.saving.notice = Some(Notice::KeptMine(copy));
                Vec::new()
            }
            Choice::TakeTheirs(theirs) => {
                self.reload(theirs);
                self.saving.notice = Some(Notice::TookTheirs(copy));
                vec![Effect::Reloaded]
            }
            Choice::Leave(_) => {
                self.saving.left_in = Some(copy);
                vec![Effect::Quit]
            }
        }
    }

    /// Takes in what the file on disk now holds: a file that is gone is marked so; a changed one
    /// is reloaded when the document holds nothing the file lacks and it can be read, and opens
    /// the conflict prompt otherwise. With the prompt open, it is what the prompt offers from now
    /// on, and a choice waiting for its copy no longer stands; a prompt open only because the
    /// file could not be read closes and the file reloads once it can. An edit under way is kept first,
    /// so it counts as a change the file lacks.
    pub(super) fn on_disk(&mut self, change: OnDisk) -> Vec<Effect> {
        let Some(change) = self.held_back(change) else {
            return Vec::new();
        };
        self.finish_editing();
        let theirs = match change {
            OnDisk::Gone => {
                self.saving.gone = true;
                if let Some(conflict) = &mut self.saving.conflict {
                    cancel_choice(conflict);
                }
                return Vec::new();
            }
            OnDisk::Changed(document) => Some(document),
            OnDisk::Unreadable => None,
        };
        self.saving.gone = false;
        let saved = self.saving.autosave.state(self.document.revision()) == SaveState::Saved;
        if let Some(conflict) = &mut self.saving.conflict {
            cancel_choice(conflict);
            // A prompt open only because the file could not be read gives way once it can.
            let leaving = matches!(conflict.choosing, Some((_, Choice::Leave(_))));
            if !saved || theirs.is_none() || leaving {
                conflict.theirs = theirs;
                return Vec::new();
            }
            self.saving.conflict = None;
        }
        match theirs {
            Some(theirs) if saved => {
                self.reload(theirs);
                self.saving.notice = Some(Notice::Reloaded);
                vec![Effect::Reloaded]
            }
            theirs => {
                self.saving.conflict = Some(Conflict {
                    theirs,
                    diff: false,
                    choosing: None,
                    confirm_quit: false,
                });
                Vec::new()
            }
        }
    }

    /// Keeps the document: once the file on disk is in a conflict copy, the document overwrites
    /// it.
    pub(super) fn keep_mine(&mut self) -> Vec<Effect> {
        self.choose(Choice::KeepMine, ConflictSide::Theirs)
    }

    /// Takes the file on disk: once the document is in a conflict copy, the file is reloaded. A
    /// file that cannot be read cannot be taken.
    pub(super) fn take_theirs(&mut self) -> Vec<Effect> {
        let Some(conflict) = &self.saving.conflict else {
            return Vec::new();
        };
        if let Some(theirs) = conflict.theirs.clone() {
            self.choose(Choice::TakeTheirs(theirs), ConflictSide::Mine)
        } else {
            self.notify(Notice::CannotTakeTheirs);
            Vec::new()
        }
    }

    /// Starts `choice` with a conflict copy of `discarded`, unless no prompt is open or a choice
    /// is under way.
    fn choose(&mut self, choice: Choice, discarded: ConflictSide) -> Vec<Effect> {
        let saving = &mut self.saving;
        let Some(conflict) = saving
            .conflict
            .as_mut()
            .filter(|conflict| conflict.choosing.is_none())
        else {
            return Vec::new();
        };
        let id = saving.last_copy.next();
        conflict.choosing = Some((id, choice));
        vec![Effect::Copy {
            side: discarded,
            ending: false,
            id,
        }]
    }

    /// Shows the diff in the conflict prompt, or hides it again.
    pub(super) fn toggle_diff(&mut self) {
        if let Some(conflict) = &mut self.saving.conflict {
            conflict.diff = !conflict.diff;
        }
    }

    /// Replaces the document with `document`, read again from the file, keeping the pane on the
    /// container it showed, its focus, what is picked and its history on the items they were on,
    /// as far as their text paths find them, the Navigator open on the places now listed with its
    /// query, Go to Symbol open on the items now listed with its query, Move to open on the
    /// destinations now listed with its query while an item it moves is found again, the date
    /// prompt open while an item it dates is, Quick Add open on its place while that is found
    /// again, else on the Inbox, and each pane's attachments panel open while its item is and
    /// [stays](attachments::stays) open, on what it selected as far as that is found again, and
    /// the sidebar on its entry and its collapsed areas as they are found again. The tag filter
    /// stays while the container shown is found again as it was. The context menu and the command
    /// palette close, since what they offer was worked out from the document as it was (D-110,
    /// D-116). The undo history is cleared, since its steps fit only the document as it was.
    fn reload(&mut self, document: Document) {
        let old = std::mem::replace(&mut self.document, document);
        let new = &self.document;
        let nearest = |id| text_path::find(&old, new, id).map(|(found, _)| found);
        let exact =
            |id| text_path::find(&old, new, id).and_then(|(found, exact)| exact.then_some(found));
        let today = self.today;
        let panes = self
            .left
            .iter_mut()
            .chain([&mut self.pane])
            .chain(&mut self.right);
        for pane in panes {
            pane.reload(new, today, nearest, exact);
            pane.panel = pane
                .panel
                .and_then(|panel| attachments::rematched(panel, &old, new, &exact));
            pane.keep_panel(new);
        }
        if let Some(picker) = &self.navigator {
            let query = picker.view().query.text().to_owned();
            self.navigator = Some(navigator::reopen(new, self.shown(), &query));
        }
        if let Some(picker) = &self.go_to_symbol {
            let query = picker.view().query.text().to_owned();
            self.go_to_symbol = Some(go_to_symbol::reopen(new, &query));
        }
        if let Some(move_to) = &self.move_to {
            let query = move_to.view().query.text().to_owned();
            let items = move_to.items().iter().filter_map(|&id| exact(id)).collect();
            self.move_to = MoveTo::reopen(new, items, &query);
        }
        self.date_prompt = self
            .date_prompt
            .take()
            .and_then(|prompt| prompt.rematched(exact));
        if let Some(quick_add) = &mut self.quick_add {
            quick_add.rematch(exact);
        }
        self.quick_comment = self
            .quick_comment
            .take()
            .and_then(|quick_comment| quick_comment.rematched(exact));
        self.edit_dialog =
            (self.edit_dialog.take()).and_then(|dialog| dialog.rematched(new, exact));
        self.context_menu = None;
        self.palette = None;
        self.sidebar.rematch(exact);
        if let Some(tags) = &self.tags {
            let items = tags.items().iter().filter_map(|&id| exact(id)).collect();
            let query = self.pane().query.text();
            let rows = view::rows(new, self.shown(), self.today, self.shows_completed(), query);
            self.tags = tags.reopen(new, items, &rows, self.tag_filter());
        }
        self.history = super::History::default();
        self.saving.autosave.reloaded(new.revision());
    }

    /// Whether the file on disk holds every change, and why the last save failed if it did.
    #[must_use]
    pub fn save_state(&self) -> SaveState<'_> {
        self.saving.autosave.state(self.document.revision())
    }

    /// Whether the file was deleted or moved away; the next save creates it again.
    #[must_use]
    pub fn gone_from_disk(&self) -> bool {
        self.saving.gone
    }

    /// What the status line says about the file until the next command.
    #[must_use]
    pub fn notice(&self) -> Option<&Notice> {
        self.saving.notice.as_ref()
    }

    /// Whether the message just handled was refused, as the status line says, which a frontend
    /// shows (FR-UI-07).
    #[must_use]
    pub fn refused(&self) -> bool {
        self.saving.refused
    }

    /// Whether quitting was refused because the document could not be kept, so quitting again
    /// ends without saving it.
    #[must_use]
    pub fn confirming_quit(&self) -> bool {
        self.saving.autosave.confirming_quit()
            || self
                .saving
                .conflict
                .as_ref()
                .is_some_and(|conflict| conflict.confirm_quit)
    }

    /// The conflict prompt, while it is open.
    #[must_use]
    pub fn conflict(&self) -> Option<ConflictView> {
        let conflict = self.saving.conflict.as_ref()?;
        Some(ConflictView {
            unsaved: self.save_state() != SaveState::Saved,
            readable: conflict.theirs.is_some(),
            diff: conflict.diff,
        })
    }

    /// The conflict copy holding the document when the session ended with the conflict prompt
    /// open, the only place its changes are then.
    #[must_use]
    pub fn left_in(&self) -> Option<&std::path::Path> {
        self.saving.left_in.as_deref()
    }

    /// When saving is next due without anything else happening, so the frontend can wake then:
    /// at once for a change no message has stepped saving for yet, as an edit that ends when a
    /// panel comes to cover its list as a frontend draws it (D-136).
    #[must_use]
    pub fn deadline(&self) -> Option<Duration> {
        if self.saving.conflict.is_some() || self.editing().is_some() {
            return None;
        }
        if !self.saving.autosave.has_seen(self.document.revision()) {
            return Some(Duration::ZERO);
        }
        self.saving.autosave.deadline()
    }
}

/// Cancels the choice waiting for its copy, since the version it discards changed; leaving still
/// stands, as the session is ending.
fn cancel_choice(conflict: &mut Conflict) {
    conflict
        .choosing
        .take_if(|(_, choice)| !matches!(choice, Choice::Leave(_)));
}

#[cfg(test)]
mod tests {
    use jiff::civil::{Date, date};

    use super::*;
    use crate::app::{
        AttachmentsInput, Command, DateInput, Direction, EditDialogInput, EditInput, Focus, Go,
        History, Message, Opened, PickerInput, Prefill, QuickAddInput, Selected, Side, TextEdit,
        Whose,
        navigator::Stop,
        update,
        view::{Shown, View},
    };
    use crate::domain::{Item, ItemId, ItemKind, Node, PlacementError, dates::DateKind};

    const TODAY: Date = date(2026, 10, 14);

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    fn secs(seconds: u64) -> Duration {
        Duration::from_secs(seconds)
    }

    /// A document with the tasks `titles` in its Inbox, and the project `P` holding `p1` and
    /// `p2`.
    fn document(titles: &[&str]) -> Result<Document, PlacementError> {
        let mut document = Document::new();
        let inbox = document.inbox();
        for title in titles {
            document.push(Some(inbox), Node::Item(Item::new(ItemKind::Task, *title)))?;
        }
        let project = document.push(None, Node::Item(Item::new(ItemKind::Project, "P")))?;
        for title in ["p1", "p2"] {
            document.push(Some(project), Node::Item(Item::new(ItemKind::Task, title)))?;
        }
        Ok(document)
    }

    fn opened(titles: &[&str]) -> Result<State, PlacementError> {
        Ok(State::new(
            document(titles)?,
            Opened::File("todo.taskpaper".into()),
            TODAY,
        ))
    }

    fn send(state: &mut State, message: Message) -> Vec<Effect> {
        update(state, message, TODAY)
    }

    fn command(state: &mut State, command: Command) -> Vec<Effect> {
        send(state, Message::Command(command))
    }

    /// `state` with a change saved neither on disk nor yet asked to be.
    fn changed(titles: &[&str]) -> Result<State, PlacementError> {
        let mut state = opened(titles)?;
        command(&mut state, Command::ToggleCompleted);
        Ok(state)
    }

    fn conflicted() -> Result<State, PlacementError> {
        let mut state = changed(&["a"])?;
        assert_eq!(
            send(
                &mut state,
                Message::OnDisk(OnDisk::Changed(document(&["x"])?))
            ),
            []
        );
        Ok(state)
    }

    /// The first item of `state`'s document titled `title`, depth first from the Inbox.
    fn titled(state: &State, title: &str) -> Option<ItemId> {
        find(state.document(), title)
    }

    fn find(document: &Document, title: &str) -> Option<ItemId> {
        let mut stack = vec![document.inbox()];
        stack.extend(document.top_level().iter().rev());
        stack.reverse();
        while let Some(id) = stack.pop() {
            if document
                .node(id)
                .and_then(Node::item)
                .is_some_and(|item| item.title == title)
            {
                return Some(id);
            }
            stack.extend(document.children(id).iter().rev());
        }
        None
    }

    // FR-STORE-09, F-FILE-05
    #[test]
    fn a_change_is_saved_a_second_after_it_and_the_demo_never() -> TestResult {
        let mut state = opened(&["a"])?;
        assert_eq!(state.save_state(), SaveState::Saved);
        assert_eq!(state.deadline(), None);
        assert_eq!(command(&mut state, Command::ToggleCompleted), []);
        assert_eq!(state.save_state(), SaveState::Unsaved);
        assert_eq!(state.deadline(), Some(secs(1)));
        assert_eq!(send(&mut state, Message::Tick(secs(1))), [Effect::Save]);
        assert_eq!(send(&mut state, Message::Disk(Report::Saved)), []);
        assert_eq!(state.save_state(), SaveState::Saved);
        let mut demo = State::new(document(&["a"])?, Opened::Demo, TODAY);
        command(&mut demo, Command::ToggleCompleted);
        assert_eq!(demo.deadline(), None);
        assert_eq!(send(&mut demo, Message::Tick(secs(5))), []);
        Ok(())
    }

    // F-FILE-05
    #[test]
    fn quitting_and_terminating_save_first() -> TestResult {
        let mut state = changed(&["a"])?;
        assert_eq!(command(&mut state, Command::Quit), [Effect::Save]);
        assert_eq!(
            send(&mut state, Message::Disk(Report::Saved)),
            [Effect::Quit]
        );
        let mut state = changed(&["a"])?;
        assert_eq!(send(&mut state, Message::Terminate), [Effect::Save]);
        assert_eq!(
            send(&mut state, Message::Disk(Report::SaveFailed("full".into()))),
            [Effect::Recover, Effect::Quit]
        );
        Ok(())
    }

    // FR-STORE-09
    #[test]
    fn a_failed_save_or_backup_is_kept_for_the_status_line() -> TestResult {
        let mut state = changed(&["a"])?;
        send(&mut state, Message::Tick(secs(1)));
        send(
            &mut state,
            Message::Disk(Report::BackupFailed("denied".into())),
        );
        assert_eq!(state.notice(), Some(&Notice::BackupFailed("denied".into())));
        send(&mut state, Message::Disk(Report::SaveFailed("full".into())));
        assert_eq!(state.save_state(), SaveState::Failed { reason: "full" });
        command(&mut state, Command::ShowCompleted);
        assert_eq!(state.notice(), None, "a command clears it");
        Ok(())
    }

    // F-FILE-06
    #[test]
    fn an_outside_change_reloads_keeping_what_the_pane_shows_and_clearing_undo() -> TestResult {
        let mut state = opened(&["a", "b", "c"])?;
        command(&mut state, Command::ToggleCompleted);
        send(&mut state, Message::Tick(secs(1)));
        send(&mut state, Message::Disk(Report::Saved));
        command(&mut state, Command::ShowCompleted);
        command(&mut state, Command::Focus(crate::app::Focus::Next));
        assert_ne!(state.history, History::default());
        let theirs = document(&["x", "b", "c"])?;
        assert_eq!(
            send(&mut state, Message::OnDisk(OnDisk::Changed(theirs.clone()))),
            [Effect::Reloaded]
        );
        assert_eq!(state.document(), &theirs);
        assert_eq!(state.container(), Some(state.document().inbox()));
        assert_eq!(state.selection().focused(), titled(&state, "c"));
        assert_eq!(state.history, History::default());
        assert_eq!(state.notice(), Some(&Notice::Reloaded));
        assert_eq!(state.save_state(), SaveState::Saved);
        assert_eq!(state.deadline(), None, "nothing to save");
        Ok(())
    }

    // F-FILE-06, N-PERF-02
    #[test]
    fn a_reload_lists_the_document_read_again_at_whatever_revision() -> TestResult {
        let mut state = opened(&["a", "b", "c"])?;
        // The same changes in another order: the same revision, with other identifiers.
        let mut theirs = Document::new();
        let inbox = Some(theirs.inbox());
        let project = theirs.push(None, Node::Item(Item::new(ItemKind::Project, "P")))?;
        for title in ["a", "b", "c"] {
            theirs.push(inbox, Node::Item(Item::new(ItemKind::Task, title)))?;
        }
        for title in ["p1", "p2"] {
            theirs.push(Some(project), Node::Item(Item::new(ItemKind::Task, title)))?;
        }
        assert_eq!(theirs.revision(), state.document().revision());
        send(&mut state, Message::OnDisk(OnDisk::Changed(theirs.clone())));
        assert_eq!(
            state.rows(),
            crate::app::container::rows(&theirs, theirs.inbox(), false),
            "not the rows listed before"
        );
        Ok(())
    }

    // F-FILE-06, FR-OPS-01
    #[test]
    fn a_reload_keeps_the_picked_items_found_again() -> TestResult {
        let mut state = opened(&["a", "b", "c"])?;
        command(&mut state, Command::SelectAll);
        let theirs = document(&["x", "b", "c"])?;
        send(&mut state, Message::OnDisk(OnDisk::Changed(theirs)));
        let rows = state.rows();
        assert_eq!(
            state.selection().items(&rows),
            [titled(&state, "b"), titled(&state, "c")]
                .into_iter()
                .flatten()
                .collect::<Vec<_>>()
        );
        Ok(())
    }

    // F-FILE-06
    #[test]
    fn a_reload_falls_back_to_the_nearest_container_and_row_found() -> TestResult {
        let mut state = opened(&["a"])?;
        let project = titled(&state, "P").ok_or("P")?;
        state.pane.shown = Shown::Container(project);
        state.pane.selection.focus(titled(&state, "p2"));
        let mut theirs = document(&["a"])?;
        let p2 = find(&theirs, "p2").ok_or("p2")?;
        theirs.set(p2, Item::new(ItemKind::Task, "renamed"))?;
        send(&mut state, Message::OnDisk(OnDisk::Changed(theirs)));
        assert_eq!(state.container(), Some(titled(&state, "P").ok_or("P")?));
        assert_eq!(
            state.selection().focused(),
            titled(&state, "p1"),
            "the first row"
        );
        let mut state = opened(&["a"])?;
        state.pane.shown = Shown::Container(project);
        send(
            &mut state,
            Message::OnDisk(OnDisk::Changed(Document::new())),
        );
        assert_eq!(
            state.container(),
            Some(state.document().inbox()),
            "P is gone"
        );
        assert_eq!(state.selection().focused(), None);
        Ok(())
    }

    // F-FILE-06, FR-TAG-04
    #[test]
    fn a_reload_keeps_the_tag_filter_only_on_the_container_found_as_it_was() -> TestResult {
        let mut state = opened(&["a"])?;
        state.pane.shown = Shown::Container(titled(&state, "P").ok_or("P")?);
        state.pane.filter = vec!["x".to_owned()];
        send(
            &mut state,
            Message::OnDisk(OnDisk::Changed(document(&["b"])?)),
        );
        assert_eq!(state.tag_filter(), ["x"]);
        let mut theirs = document(&["b"])?;
        let project = find(&theirs, "P").ok_or("P")?;
        theirs.set(project, Item::new(ItemKind::Project, "Q"))?;
        send(&mut state, Message::OnDisk(OnDisk::Changed(theirs)));
        assert_eq!(
            state.tag_filter(),
            [] as [String; 0],
            "P is not found as it was"
        );
        Ok(())
    }

    // F-FILE-06, FR-VIEW-06, FR-TAG-04
    #[test]
    fn a_reload_keeps_a_view_shown_with_its_tag_filter() -> TestResult {
        let mut state = opened(&["a"])?;
        state.go(Go::View(View::Today));
        state.pane.filter = vec!["x".to_owned()];
        send(
            &mut state,
            Message::OnDisk(OnDisk::Changed(Document::new())),
        );
        assert_eq!(state.shown(), Shown::View(View::Today));
        assert_eq!(state.tag_filter(), ["x"]);
        Ok(())
    }

    // F-FILE-06, FR-OPS-04
    #[test]
    fn a_reload_keeps_move_to_open_while_an_item_it_moves_is_found_again() -> TestResult {
        let mut state = opened(&["a"])?;
        command(&mut state, Command::MoveTo);
        send(
            &mut state,
            Message::MoveTo(PickerInput::Edit(TextEdit::Insert('p'))),
        );
        send(
            &mut state,
            Message::OnDisk(OnDisk::Changed(document(&["a", "x"])?)),
        );
        assert_eq!(state.move_to().ok_or("open")?.query.text(), "p");
        send(&mut state, Message::MoveTo(PickerInput::Choose));
        let project = titled(&state, "P").ok_or("P")?;
        let a = titled(&state, "a").ok_or("a")?;
        assert_eq!(state.document().position(a), Some((Some(project), 2)));
        let mut state = opened(&["a"])?;
        command(&mut state, Command::MoveTo);
        send(
            &mut state,
            Message::OnDisk(OnDisk::Changed(document(&["y"])?)),
        );
        assert!(state.move_to().is_none(), "a, which it moved, is gone");
        Ok(())
    }

    // F-FILE-06, FR-DATE-02
    #[test]
    fn a_reload_keeps_the_date_prompt_open_while_an_item_it_dates_is_found_again() -> TestResult {
        let mut state = opened(&["a"])?;
        command(&mut state, Command::SetDate(DateKind::Due));
        send(
            &mut state,
            Message::DatePrompt(DateInput::Edit(TextEdit::Insert('+'))),
        );
        send(
            &mut state,
            Message::OnDisk(OnDisk::Changed(document(&["a", "x"])?)),
        );
        assert_eq!(state.date_prompt().ok_or("open")?.field.text(), "+");
        let mut state = opened(&["a"])?;
        command(&mut state, Command::SetDate(DateKind::Due));
        send(
            &mut state,
            Message::OnDisk(OnDisk::Changed(document(&["y"])?)),
        );
        assert!(state.date_prompt().is_none(), "a, which it dates, is gone");
        Ok(())
    }

    // F-FILE-06, FR-EDIT-08
    #[test]
    fn a_reload_keeps_the_edit_dialog_open_while_its_item_is_found_again() -> TestResult {
        let mut state = opened(&["a"])?;
        command(&mut state, Command::EditItem);
        send(
            &mut state,
            Message::EditDialog(EditDialogInput::Edit(TextEdit::Insert('!'))),
        );
        send(
            &mut state,
            Message::OnDisk(OnDisk::Changed(document(&["a", "x"])?)),
        );
        send(&mut state, Message::EditDialog(EditDialogInput::Accept));
        assert!(
            titled(&state, "a!").is_some(),
            "the draft applies to a found again"
        );
        let mut state = opened(&["a"])?;
        command(&mut state, Command::EditItem);
        for input in [EditDialogInput::Next, EditDialogInput::Next] {
            send(&mut state, Message::EditDialog(input));
        }
        send(
            &mut state,
            Message::EditDialog(EditDialogInput::Edit(TextEdit::Insert(' '))),
        );
        send(
            &mut state,
            Message::OnDisk(OnDisk::Changed(document(&["a", "x"])?)),
        );
        assert!(
            state.edit_dialog().ok_or("open")?.picker.is_some(),
            "with its tag picker"
        );
        let mut state = opened(&["a"])?;
        command(&mut state, Command::EditItem);
        send(
            &mut state,
            Message::OnDisk(OnDisk::Changed(document(&["y"])?)),
        );
        assert!(state.edit_dialog().is_none(), "a, which it edits, is gone");
        Ok(())
    }

    // F-FILE-06, FR-KEY-07
    #[test]
    fn a_reload_closes_the_context_menu() -> TestResult {
        let mut state = opened(&["a", "b"])?;
        command(&mut state, Command::Focus(Focus::Last));
        command(&mut state, Command::ContextMenu);
        assert!(state.context_menu().is_some());
        send(
            &mut state,
            Message::OnDisk(OnDisk::Changed(document(&["a"])?)),
        );
        assert!(
            state.context_menu().is_none(),
            "b, which it acts on, is gone"
        );
        let mut state = opened(&["a", "b"])?;
        command(&mut state, Command::ContextMenu);
        send(
            &mut state,
            Message::OnDisk(OnDisk::Changed(document(&["a", "b", "c"])?)),
        );
        assert!(
            state.context_menu().is_none(),
            "what it offers fits the document as it was"
        );
        Ok(())
    }

    // F-FILE-06, FR-ATT-01
    #[test]
    fn a_reload_keeps_the_attachments_panel_open_while_its_item_is_found_again() -> TestResult {
        let mut state = opened(&["a"])?;
        command(&mut state, Command::OpenNote(Whose::Focused));
        command(&mut state, Command::FocusLeft);
        command(&mut state, Command::OpenInNewPane);
        command(&mut state, Command::OpenComments(Whose::Focused));
        send(
            &mut state,
            Message::OnDisk(OnDisk::Changed(document(&["a", "x"])?)),
        );
        let a = titled(&state, "a").ok_or("a")?;
        let panels = |state: &State| -> Vec<_> {
            state
                .panes()
                .map(|pane| pane.attachments().map(|view| view.id))
                .collect()
        };
        assert_eq!(panels(&state), [Some(a), Some(a)], "each pane's panel");
        assert_eq!(state.attachments().map(|view| view.id), Some(a));
        send(
            &mut state,
            Message::OnDisk(OnDisk::Changed(document(&["y"])?)),
        );
        assert_eq!(
            panels(&state),
            [None, None],
            "a, which they are on, is gone"
        );
        assert!(state.attachments().is_none());
        Ok(())
    }

    // F-FILE-06, FR-ATT-01, FR-VIEW-04
    #[test]
    fn a_reload_closes_a_panel_whose_item_went_to_the_trash_outside_the_trash() -> TestResult {
        let mut state = opened(&["a"])?;
        command(&mut state, Command::OpenNote(Whose::Focused));
        command(&mut state, Command::FocusLeft);
        command(&mut state, Command::OpenInNewPane);
        command(&mut state, Command::OpenNote(Whose::Focused));
        command(&mut state, Command::FocusLeft);
        command(&mut state, Command::Go(Go::View(View::Trash)));
        let mut trashed = document(&["a"])?;
        let a = find(&trashed, "a").ok_or("a")?;
        let item = trashed.node(a).and_then(Node::item).ok_or("a")?;
        let item = Item {
            deleted: Some(TODAY),
            ..item.clone()
        };
        trashed.set(a, item)?;
        send(&mut state, Message::OnDisk(OnDisk::Changed(trashed)));
        let a = titled(&state, "a").ok_or("a")?;
        let panels: Vec<_> = state
            .panes()
            .map(|pane| pane.attachments().map(|view| view.id))
            .collect();
        assert_eq!(panels, [None, Some(a)], "only the Trash's panel stays");
        Ok(())
    }

    /// [`document`] with `comments` on `a`.
    fn commented(comments: &[&str]) -> Result<Document, PlacementError> {
        let mut document = document(&["a"])?;
        let a = find(&document, "a").ok_or(PlacementError::DoesNotFit)?;
        let mut item = document
            .node(a)
            .and_then(Node::item)
            .cloned()
            .ok_or(PlacementError::DoesNotFit)?;
        item.comments = comments.iter().map(|&comment| comment.to_owned()).collect();
        document.set(a, item)?;
        Ok(document)
    }

    // F-FILE-06, FR-ATT-04
    #[test]
    fn a_reload_finds_the_selected_comment_again_as_the_same_line() -> TestResult {
        let mut state = State::new(
            commented(&["x", "y"])?,
            Opened::File("todo.taskpaper".into()),
            TODAY,
        );
        command(&mut state, Command::OpenComments(Whose::Focused));
        send(
            &mut state,
            Message::Attachments(AttachmentsInput::Select(Focus::Last)),
        );
        let selected = |state: &State| state.attachments().map(|view| view.selected);
        assert_eq!(selected(&state), Some(Selected::Comment(Some(1))));
        send(
            &mut state,
            Message::OnDisk(OnDisk::Changed(commented(&["new", "x", "y"])?)),
        );
        assert_eq!(
            selected(&state),
            Some(Selected::Comment(Some(2))),
            "y again"
        );
        send(
            &mut state,
            Message::OnDisk(OnDisk::Changed(commented(&["new", "x", "z"])?)),
        );
        assert_eq!(
            selected(&state),
            Some(Selected::Comment(Some(2))),
            "y is gone"
        );
        send(
            &mut state,
            Message::OnDisk(OnDisk::Changed(commented(&[])?)),
        );
        assert_eq!(selected(&state), Some(Selected::Comment(None)), "none left");
        send(
            &mut state,
            Message::OnDisk(OnDisk::Changed(commented(&["back"])?)),
        );
        assert_eq!(
            selected(&state),
            Some(Selected::Comment(Some(0))),
            "one again"
        );
        Ok(())
    }

    // F-FILE-06, FR-TAG-02
    #[test]
    fn a_reload_keeps_the_tag_picker_open_while_an_item_it_tags_is_found_again() -> TestResult {
        let mut state = opened(&["a"])?;
        command(&mut state, Command::Tags);
        send(
            &mut state,
            Message::Tags(PickerInput::Edit(TextEdit::Insert('t'))),
        );
        send(
            &mut state,
            Message::OnDisk(OnDisk::Changed(document(&["a", "x"])?)),
        );
        assert_eq!(state.tags().ok_or("open")?.picker.query.text(), "t");
        send(&mut state, Message::Tags(PickerInput::Choose));
        let a = titled(&state, "a").ok_or("a")?;
        let tags = &state
            .document()
            .node(a)
            .and_then(Node::item)
            .ok_or("a")?
            .tags;
        assert_eq!(tags.first().map(|tag| tag.name.as_str()), Some("t"));
        let mut state = opened(&["a"])?;
        command(&mut state, Command::Tags);
        send(
            &mut state,
            Message::OnDisk(OnDisk::Changed(document(&["y"])?)),
        );
        assert!(state.tags().is_none(), "a, which it tagged, is gone");
        Ok(())
    }

    // F-FILE-06, FR-NAV-05
    #[test]
    fn a_reload_keeps_go_to_symbol_open_with_its_query_on_the_items_now_listed() -> TestResult {
        let mut state = opened(&["a"])?;
        command(&mut state, Command::GoToSymbol);
        send(
            &mut state,
            Message::GoToSymbol(PickerInput::Edit(TextEdit::Insert('x'))),
        );
        assert_eq!(state.go_to_symbol().map(|view| view.entries.len()), Some(0));
        send(
            &mut state,
            Message::OnDisk(OnDisk::Changed(document(&["x"])?)),
        );
        assert_eq!(
            state
                .go_to_symbol()
                .map(|view| view.query.text().to_owned()),
            Some("x".to_owned())
        );
        send(&mut state, Message::GoToSymbol(PickerInput::Choose));
        assert_eq!(state.selection().focused(), titled(&state, "x"));
        Ok(())
    }

    // F-FILE-06, FR-VIEW-10
    #[test]
    fn a_reload_keeps_the_sidebar_on_its_entry_and_its_areas_collapsed() -> TestResult {
        let areas = |titles: &[&str]| -> Result<Document, PlacementError> {
            let mut document = Document::new();
            let inbox = document.inbox();
            for title in titles {
                document.push(Some(inbox), Node::Item(Item::new(ItemKind::Task, *title)))?;
            }
            for (area, project) in [("Alpha", "A1"), ("Beta", "B1")] {
                let area = document.push(None, Node::Item(Item::new(ItemKind::Area, area)))?;
                let project = Item::new(ItemKind::Project, project);
                document.push(Some(area), Node::Item(project))?;
            }
            Ok(document)
        };
        let mut state = State::new(
            areas(&["t1", "t2"])?,
            Opened::File("todo.taskpaper".into()),
            TODAY,
        );
        command(&mut state, Command::Sidebar);
        command(&mut state, Command::Focus(Focus::Last));
        for _ in 0..3 {
            command(&mut state, Command::Focus(Focus::Previous));
        }
        command(&mut state, Command::Collapse);
        send(&mut state, Message::OnDisk(OnDisk::Changed(areas(&[])?)));
        let sidebar = state.sidebar().ok_or("shown")?;
        let alpha = titled(&state, "Alpha").ok_or("Alpha")?;
        let focused = sidebar.focused.and_then(|at| sidebar.entries.get(at));
        assert!(
            matches!(focused, Some(Stop::Place(place)) if place.id == alpha),
            "{focused:?}"
        );
        assert_eq!(sidebar.collapsed, [alpha]);
        Ok(())
    }

    // F-FILE-06
    #[test]
    fn a_reload_keeps_the_navigator_open_with_its_query() -> TestResult {
        let mut state = opened(&["a"])?;
        command(&mut state, Command::Navigator);
        send(
            &mut state,
            Message::Navigator(PickerInput::Edit(TextEdit::Insert('p'))),
        );
        send(
            &mut state,
            Message::OnDisk(OnDisk::Changed(document(&["x"])?)),
        );
        let navigator = state.navigator().ok_or("open")?;
        assert_eq!(navigator.query.text(), "p");
        send(&mut state, Message::Navigator(PickerInput::Choose));
        assert_eq!(state.container(), Some(titled(&state, "P").ok_or("P")?));
        send(
            &mut state,
            Message::OnDisk(OnDisk::Changed(document(&["y"])?)),
        );
        command(&mut state, Command::Go(crate::app::Go::Back));
        assert_eq!(
            state.container(),
            Some(state.document().inbox()),
            "back where it was"
        );
        Ok(())
    }

    // F-FILE-06, FR-ATT-06
    #[test]
    fn a_reload_keeps_quick_comment_on_its_item_found_again_and_closes_it_once_gone() -> TestResult
    {
        let mut state = opened(&["a", "b"])?;
        command(&mut state, Command::QuickComment);
        send(
            &mut state,
            Message::OnDisk(OnDisk::Changed(document(&["z", "a"])?)),
        );
        assert_eq!(state.quick_comment().map(|view| view.title), Some("a"));
        send(
            &mut state,
            Message::OnDisk(OnDisk::Changed(document(&["x"])?)),
        );
        assert!(state.quick_comment().is_none());
        Ok(())
    }

    // F-FILE-06, FR-EDIT-07
    #[test]
    fn a_reload_keeps_quick_add_open_on_its_place_found_again_and_closes_its_picker() -> TestResult
    {
        let mut state = opened(&["a"])?;
        command(&mut state, Command::QuickAdd(Prefill::Nothing));
        for c in "/P".chars() {
            send(
                &mut state,
                Message::QuickAdd(QuickAddInput::Edit(TextEdit::Insert(c))),
            );
        }
        send(&mut state, Message::QuickAddPicker(PickerInput::Choose));
        send(
            &mut state,
            Message::QuickAdd(QuickAddInput::Edit(TextEdit::Insert('/'))),
        );
        send(
            &mut state,
            Message::OnDisk(OnDisk::Changed(document(&["x"])?)),
        );
        assert!(state.quick_add_picker().is_none());
        let place = state.quick_add().map(|view| view.place);
        assert_eq!(place, Some(vec!["P".to_owned()]));
        Ok(())
    }

    // F-FILE-06
    #[test]
    fn an_outside_change_to_unsaved_changes_asks_and_pauses_saving() -> TestResult {
        let mut state = conflicted()?;
        let view = ConflictView {
            unsaved: true,
            readable: true,
            diff: false,
        };
        assert_eq!(state.conflict(), Some(view));
        assert_eq!(state.deadline(), None);
        assert_eq!(send(&mut state, Message::Tick(secs(5))), []);
        command(&mut state, Command::ShowDiff);
        assert_eq!(state.conflict(), Some(ConflictView { diff: true, ..view }));
        command(&mut state, Command::ShowDiff);
        assert_eq!(state.conflict(), Some(view));
        Ok(())
    }

    // F-FILE-06
    #[test]
    fn a_refused_save_does_not_save_over_the_file_that_refused_it() -> TestResult {
        let mut state = changed(&["a"])?;
        assert_eq!(send(&mut state, Message::Tick(secs(1))), [Effect::Save]);
        let theirs = OnDisk::Changed(document(&["x"])?);
        assert_eq!(send(&mut state, Message::OnDisk(theirs)), []);
        assert_eq!(send(&mut state, Message::Disk(Report::SaveRefused)), []);
        assert!(state.conflict().is_some());
        let mut state = changed(&["a"])?;
        send(&mut state, Message::Tick(secs(1)));
        send(&mut state, Message::OnDisk(OnDisk::Gone));
        assert_eq!(
            send(&mut state, Message::Disk(Report::SaveRefused)),
            [Effect::Save],
            "a file gone is created again"
        );
        assert_eq!(command(&mut state, Command::TakeTheirs), [], "no prompt");
        assert_eq!(titled(&state, "nowhere"), None);
        Ok(())
    }

    // F-FILE-06
    #[test]
    fn keeping_mine_copies_theirs_and_then_overwrites_the_file() -> TestResult {
        let mut state = conflicted()?;
        let mine = state.document().clone();
        assert_eq!(
            command(&mut state, Command::KeepMine),
            [Effect::Copy {
                side: ConflictSide::Theirs,
                ending: false,
                id: CopyId(1),
            }]
        );
        assert_eq!(command(&mut state, Command::KeepMine), [], "one at a time");
        assert_eq!(command(&mut state, Command::TakeTheirs), []);
        let copy = PathBuf::from("theirs.taskpaper");
        assert_eq!(
            send(
                &mut state,
                Message::Disk(Report::Copied(CopyId(1), copy.clone()))
            ),
            []
        );
        assert_eq!(send(&mut state, Message::Tick(secs(1))), [Effect::Save]);
        assert_eq!(state.conflict(), None);
        assert_eq!(state.document(), &mine);
        assert_eq!(state.notice(), Some(&Notice::KeptMine(copy)));
        assert_eq!(
            send(
                &mut state,
                Message::Disk(Report::Copied(CopyId(1), "x".into()))
            ),
            []
        );
        Ok(())
    }

    // F-FILE-06
    #[test]
    fn taking_theirs_copies_mine_and_then_reloads() -> TestResult {
        let mut state = conflicted()?;
        assert_eq!(
            command(&mut state, Command::TakeTheirs),
            [Effect::Copy {
                side: ConflictSide::Mine,
                ending: false,
                id: CopyId(1),
            }]
        );
        let copy = PathBuf::from("mine.taskpaper");
        assert_eq!(
            send(
                &mut state,
                Message::Disk(Report::Copied(CopyId(1), copy.clone()))
            ),
            [Effect::Reloaded]
        );
        assert_eq!(state.document(), &document(&["x"])?);
        assert_eq!(state.save_state(), SaveState::Saved);
        assert_eq!(state.notice(), Some(&Notice::TookTheirs(copy)));
        Ok(())
    }

    // F-FILE-06
    #[test]
    fn a_failed_copy_leaves_the_prompt_open_to_choose_again() -> TestResult {
        let mut state = conflicted()?;
        command(&mut state, Command::KeepMine);
        assert_eq!(
            send(
                &mut state,
                Message::Disk(Report::CopyFailed(CopyId(1), "full".into()))
            ),
            []
        );
        assert_eq!(state.notice(), Some(&Notice::CopyFailed("full".into())));
        assert!(state.conflict().is_some());
        assert_eq!(command(&mut state, Command::KeepMine).len(), 1);
        let mut state = opened(&["a"])?;
        assert_eq!(
            send(
                &mut state,
                Message::Disk(Report::CopyFailed(CopyId(1), "late".into()))
            ),
            [],
            "nothing was waiting for it"
        );
        assert_eq!(state.notice(), None);
        Ok(())
    }

    // F-FILE-06, F-FILE-07
    #[test]
    fn an_unreadable_file_asks_even_without_changes_and_cannot_be_taken() -> TestResult {
        let mut state = opened(&["a"])?;
        assert_eq!(send(&mut state, Message::OnDisk(OnDisk::Unreadable)), []);
        assert_eq!(
            state.conflict(),
            Some(ConflictView {
                unsaved: false,
                readable: false,
                diff: false
            })
        );
        assert_eq!(command(&mut state, Command::TakeTheirs), []);
        assert_eq!(state.notice(), Some(&Notice::CannotTakeTheirs));
        command(&mut state, Command::Quit);
        assert_eq!(
            send(
                &mut state,
                Message::OnDisk(OnDisk::Changed(document(&["x"])?))
            ),
            [],
            "leaving stands"
        );
        let mut state = opened(&["a"])?;
        send(&mut state, Message::OnDisk(OnDisk::Unreadable));
        command(&mut state, Command::KeepMine);
        assert_eq!(
            send(
                &mut state,
                Message::OnDisk(OnDisk::Changed(document(&["x"])?))
            ),
            [Effect::Reloaded],
            "readable again"
        );
        assert_eq!(state.conflict(), None);
        assert_eq!(
            send(
                &mut state,
                Message::Disk(Report::Copied(CopyId(1), "x".into()))
            ),
            []
        );
        let mut state = opened(&["a"])?;
        send(&mut state, Message::OnDisk(OnDisk::Unreadable));
        command(&mut state, Command::KeepMine);
        send(
            &mut state,
            Message::Disk(Report::Copied(CopyId(1), "theirs".into())),
        );
        assert_eq!(
            send(&mut state, Message::Tick(secs(1))),
            [Effect::Save],
            "the file is overwritten though the document is unchanged"
        );
        Ok(())
    }

    // F-FILE-06
    #[test]
    fn a_new_change_on_disk_cancels_the_choice_under_way() -> TestResult {
        let mut state = conflicted()?;
        command(&mut state, Command::KeepMine);
        send(&mut state, Message::OnDisk(OnDisk::Unreadable));
        assert_eq!(
            send(
                &mut state,
                Message::Disk(Report::Copied(CopyId(1), "x".into()))
            ),
            []
        );
        assert_eq!(state.conflict().map(|view| view.readable), Some(false));
        command(&mut state, Command::KeepMine);
        send(&mut state, Message::OnDisk(OnDisk::Gone));
        assert!(state.gone_from_disk());
        assert_eq!(
            send(
                &mut state,
                Message::Disk(Report::Copied(CopyId(2), "x".into()))
            ),
            []
        );
        assert!(state.conflict().is_some());
        Ok(())
    }

    // F-FILE-06
    #[test]
    fn a_copy_for_a_cancelled_choice_is_not_taken_for_a_later_one() -> TestResult {
        let mut state = conflicted()?;
        let mine = state.document().clone();
        command(&mut state, Command::KeepMine);
        send(
            &mut state,
            Message::OnDisk(OnDisk::Changed(document(&["y"])?)),
        );
        command(&mut state, Command::TakeTheirs);
        assert_eq!(
            send(
                &mut state,
                Message::Disk(Report::Copied(CopyId(1), "theirs".into()))
            ),
            [],
            "the copy of theirs does not keep mine"
        );
        assert_eq!(state.document(), &mine);
        send(
            &mut state,
            Message::Disk(Report::CopyFailed(CopyId(2), "full".into())),
        );
        assert_eq!(state.notice(), Some(&Notice::CopyFailed("full".into())));
        assert!(state.conflict().is_some());
        assert_eq!(state.document(), &mine);
        command(&mut state, Command::TakeTheirs);
        send(
            &mut state,
            Message::Disk(Report::CopyFailed(CopyId(2), "late".into())),
        );
        assert_eq!(state.notice(), None, "a failure of no choice that stands");
        assert_eq!(
            send(
                &mut state,
                Message::Disk(Report::Copied(CopyId(3), "mine".into()))
            ),
            [Effect::Reloaded]
        );
        Ok(())
    }

    // F-FILE-06
    #[test]
    fn a_file_gone_is_created_again_by_the_next_save() -> TestResult {
        let mut state = changed(&["a"])?;
        assert_eq!(send(&mut state, Message::OnDisk(OnDisk::Gone)), []);
        assert!(state.gone_from_disk());
        assert_eq!(send(&mut state, Message::Tick(secs(1))), [Effect::Save]);
        send(&mut state, Message::Disk(Report::Saved));
        assert!(!state.gone_from_disk());
        send(&mut state, Message::OnDisk(OnDisk::Gone));
        send(
            &mut state,
            Message::OnDisk(OnDisk::Changed(document(&["a"])?)),
        );
        assert!(!state.gone_from_disk(), "it came back");
        Ok(())
    }

    // F-FILE-05, F-FILE-06
    #[test]
    fn ending_with_the_prompt_open_keeps_mine_in_a_copy_and_leaves_theirs() -> TestResult {
        let mine = |id| Effect::Copy {
            side: ConflictSide::Mine,
            ending: true,
            id,
        };
        let mut state = conflicted()?;
        assert_eq!(command(&mut state, Command::Quit), [mine(CopyId(1))]);
        assert_eq!(
            command(&mut state, Command::Quit),
            [],
            "the copy is under way"
        );
        assert_eq!(send(&mut state, Message::Terminate), [mine(CopyId(2))]);
        assert_eq!(send(&mut state, Message::Terminate), []);
        send(&mut state, Message::OnDisk(OnDisk::Unreadable));
        let copy = PathBuf::from("mine.taskpaper");
        assert_eq!(
            send(
                &mut state,
                Message::Disk(Report::Copied(CopyId(2), copy.clone()))
            ),
            [Effect::Quit],
            "leaving stands whatever the file does"
        );
        assert_eq!(state.left_in(), Some(copy.as_path()));
        for choice in [Command::KeepMine, Command::TakeTheirs] {
            let mut state = conflicted()?;
            command(&mut state, choice);
            assert_eq!(
                command(&mut state, Command::Quit),
                [mine(CopyId(2))],
                "quitting leaves the choice under way"
            );
            send(
                &mut state,
                Message::Disk(Report::Copied(CopyId(1), "theirs".into())),
            );
            assert_eq!(
                send(
                    &mut state,
                    Message::Disk(Report::Copied(CopyId(2), copy.clone()))
                ),
                [Effect::Quit]
            );
            assert_eq!(state.left_in(), Some(copy.as_path()));
        }
        Ok(())
    }

    // F-FILE-05, F-FILE-06
    #[test]
    fn ending_with_the_prompt_open_never_saves_mine_over_theirs() -> TestResult {
        for ending in [Message::Command(Command::Quit), Message::Terminate] {
            let mut state = conflicted()?;
            send(&mut state, Message::Tick(secs(5)));
            send(&mut state, ending);
            assert_eq!(
                send(
                    &mut state,
                    Message::Disk(Report::Copied(CopyId(1), "mine".into()))
                ),
                [Effect::Quit],
                "mine is in its copy, so theirs stays on disk"
            );
            assert_eq!(send(&mut state, Message::Tick(secs(10))), []);
        }
        Ok(())
    }

    // F-FILE-05
    #[test]
    fn a_failed_copy_as_the_session_ends_asks_to_quit_again_or_ends_anyway() -> TestResult {
        let mut state = conflicted()?;
        command(&mut state, Command::Quit);
        assert!(!state.confirming_quit());
        assert_eq!(
            send(
                &mut state,
                Message::Disk(Report::CopyFailed(CopyId(1), "full".into()))
            ),
            []
        );
        assert!(state.confirming_quit());
        assert_eq!(command(&mut state, Command::Quit), [Effect::Quit]);
        let mut state = conflicted()?;
        send(&mut state, Message::Terminate);
        assert_eq!(
            send(
                &mut state,
                Message::Disk(Report::CopyFailed(CopyId(1), "full".into()))
            ),
            [Effect::Quit]
        );
        Ok(())
    }

    // FR-EDIT-01, F-FILE-05
    #[test]
    fn a_title_being_edited_is_saved_once_editing_ends_or_the_session_does() -> TestResult {
        let mut state = opened(&["a"])?;
        command(&mut state, Command::EditTitle);
        send(
            &mut state,
            Message::Edit(EditInput::Text(TextEdit::Insert('b'))),
        );
        assert_eq!(state.deadline(), None);
        assert_eq!(
            send(&mut state, Message::Tick(secs(5))),
            [],
            "not while typing"
        );
        assert_eq!(send(&mut state, Message::Edit(EditInput::Done)), []);
        assert_eq!(send(&mut state, Message::Tick(secs(6))), [Effect::Save]);
        send(&mut state, Message::Disk(Report::Saved));
        command(&mut state, Command::EditTitle);
        send(
            &mut state,
            Message::Edit(EditInput::Text(TextEdit::Insert('c'))),
        );
        assert_eq!(send(&mut state, Message::Terminate), [Effect::Save]);
        assert_eq!(state.editing(), None);
        assert!(titled(&state, "abc").is_some());
        Ok(())
    }

    // FR-VIEW-06, FR-VIEW-13, F-FILE-05, F-FILE-06
    #[test]
    fn the_evening_and_today_s_order_leave_the_saved_state_as_it_was() -> TestResult {
        let mut started = Document::new();
        for title in ["a", "b"] {
            let mut task = Item::new(ItemKind::Task, title);
            task.start = Some(TODAY);
            started.push(Some(started.inbox()), Node::Item(task))?;
        }
        let in_today = || {
            let mut state = State::new(
                started.clone(),
                Opened::File("todo.taskpaper".into()),
                TODAY,
            );
            command(&mut state, Command::Go(Go::View(View::Today)));
            state
        };
        let mut state = in_today();
        let steps = [
            Command::Move(Direction::Down),
            Command::Undo,
            Command::Redo,
            Command::ToggleEvening,
            Command::Undo,
            Command::Redo,
        ];
        for step in steps {
            let before = state.document().clone();
            assert_eq!(command(&mut state, step), [], "{step:?}");
            assert_ne!(
                state.document(),
                &before,
                "{step:?} changes the app's state"
            );
            assert_eq!(state.save_state(), SaveState::Saved, "{step:?}");
            assert_eq!(state.deadline(), None, "{step:?}");
        }
        assert_eq!(send(&mut state, Message::Tick(secs(5))), []);
        assert_eq!(
            send(
                &mut state,
                Message::OnDisk(OnDisk::Changed(document(&["x"])?))
            ),
            [Effect::Reloaded],
            "not a conflict"
        );
        let mut state = in_today();
        command(&mut state, Command::ToggleFlagged);
        command(&mut state, Command::ToggleEvening);
        assert_eq!(state.save_state(), SaveState::Unsaved, "still unsaved");
        assert_eq!(send(&mut state, Message::Tick(secs(6))), [Effect::Save]);
        command(&mut state, Command::ToggleEvening);
        send(&mut state, Message::Disk(Report::Saved));
        assert_eq!(state.save_state(), SaveState::Saved, "saved as it is");
        assert_eq!(send(&mut state, Message::Tick(secs(9))), []);
        Ok(())
    }

    // FR-EDIT-01, F-FILE-06
    #[test]
    fn an_outside_change_ends_editing_first() -> TestResult {
        let mut state = opened(&["a"])?;
        command(&mut state, Command::EditTitle);
        assert_eq!(
            send(
                &mut state,
                Message::OnDisk(OnDisk::Changed(document(&["a"])?))
            ),
            [Effect::Reloaded],
            "an edit that changed nothing"
        );
        assert_eq!(state.editing(), None);
        for edit in [Command::EditTitle, Command::New(Side::Below)] {
            command(&mut state, edit);
            send(
                &mut state,
                Message::Edit(EditInput::Text(TextEdit::Insert('b'))),
            );
            send(
                &mut state,
                Message::Edit(EditInput::Text(TextEdit::DeleteBackward)),
            );
            assert_eq!(
                send(
                    &mut state,
                    Message::OnDisk(OnDisk::Changed(document(&["a"])?))
                ),
                [Effect::Reloaded],
                "an edit typed and erased changed nothing"
            );
        }
        command(&mut state, Command::EditTitle);
        send(
            &mut state,
            Message::Edit(EditInput::Text(TextEdit::Insert('b'))),
        );
        send(
            &mut state,
            Message::OnDisk(OnDisk::Changed(document(&["a"])?)),
        );
        assert_eq!(state.editing(), None);
        assert!(state.conflict().is_some());
        assert!(titled(&state, "ab").is_some());
        Ok(())
    }

    // N-REL-01
    #[test]
    fn a_recovered_document_is_saved_over_the_file() -> TestResult {
        let mut state = State::recovered(document(&["a"])?, Opened::File("todo".into()), TODAY);
        assert_eq!(state.save_state(), SaveState::Unsaved);
        assert_eq!(send(&mut state, Message::Tick(secs(1))), [Effect::Save]);
        let state = State::recovered(document(&["a"])?, Opened::Demo, TODAY);
        assert_eq!(state.deadline(), None);
        Ok(())
    }
}
