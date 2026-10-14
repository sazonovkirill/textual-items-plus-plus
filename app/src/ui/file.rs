//! The opened file at the edge: carries out the core's saving effects with Store, on the disk
//! worker, and turns what the worker and the watcher report into messages for the core
//! ([file-io.md](../../../specs/reference/taskpaper-terminal/specs/file-io.md)). Hides which
//! version of the file the app last read or wrote, and the outline it is written back with.

use std::{
    path::{Path, PathBuf},
    sync::mpsc::Sender,
};

use jiff::civil::{Date, DateTime};

use super::Input;
use crate::{
    app::{ConflictSide, Layout, Message, OnDisk, Opened, Report, State},
    domain::Document,
    store::{
        self, Backups, Disk, Job, Layouts, OpenError, OpenFile, Outcome, Recovery, RecoveryError,
        Version, Watcher,
        taskpaper::{Source, serialize, write_document},
    },
};

/// Where the app keeps its own files, the local clock, and where the disk worker and the watcher
/// report to the event loop.
#[derive(Debug, Clone)]
pub struct Edge {
    /// Where each report goes.
    pub inputs: Sender<Input>,
    /// The backups folder, `None` when there is none (D-32).
    pub backups: Option<PathBuf>,
    /// The recovery folder, `None` when there is none (D-32).
    pub recovery: Option<PathBuf>,
    /// The layouts file, `None` when there is none (FR-PANE-03).
    pub layouts: Option<PathBuf>,
    /// The settings file, `None` when there is none (FR-SET-01).
    pub config: Option<PathBuf>,
    /// The local time now.
    pub clock: fn() -> DateTime,
}

/// The file a session opened, as it is known on disk.
#[derive(Debug)]
pub(super) struct File {
    path: PathBuf,
    /// The outline the document is written back with.
    source: Source,
    /// The version of the file the app last read or wrote, `None` while there is none.
    known: Option<Version>,
    /// The file as another program left it, with the outline it reads as when it can be read,
    /// until the core takes it.
    theirs: Option<(Vec<u8>, Option<Source>)>,
    disk: Disk,
    /// Conflict copies the session ends with are written at once, not on the worker.
    backups: Backups,
    recovery: Recovery,
    layouts: Layouts,
    clock: fn() -> DateTime,
    _watcher: Option<Watcher>,
}

impl File {
    /// Opens the file at `path`, with the document of the recovery file `recovered` when one is
    /// given, as changes the file lacks, after keeping the file as it is on disk in a conflict
    /// copy, since it may have changed after the recovered document was read from it
    /// ([file-io.md §4](../../../specs/reference/taskpaper-terminal/specs/file-io.md#4-crash-safety)).
    /// The panes come back as the file's kept layout has them (FR-PANE-03). Starts the disk
    /// worker, the watcher and, when the file exists, today's backup.
    ///
    /// # Errors
    ///
    /// [`OpenError`] when the file or the recovery file cannot be read, or the copy fails.
    pub(super) fn open(
        path: PathBuf,
        recovered: Option<&Path>,
        today: Date,
        edge: &Edge,
    ) -> Result<(Self, State), OpenError> {
        let backups = Backups::new(edge.backups.clone());
        let opened = Opened::File(path.clone());
        let (state, source, version) = match recovered {
            None => {
                let OpenFile {
                    document,
                    source,
                    version,
                } = store::open(&path)?;
                (State::new(document, opened, today), source, version)
            }
            Some(recovered) => {
                let OpenFile {
                    document,
                    source,
                    version,
                } = store::recover(&path, recovered, &backups, (edge.clock)())?;
                (State::recovered(document, opened, today), source, version)
            }
        };
        let layouts = Layouts::new(edge.layouts.clone());
        let state = match layouts.load(&path) {
            Some(layout) => state.with_layout(&layout),
            None => state,
        };
        let inputs = edge.inputs.clone();
        let disk = Disk::start(
            // The event loop has ended once nothing receives, so the report has nowhere to go.
            move |outcome| drop(inputs.send(Input::Disk(outcome))),
            Backups::new(edge.backups.clone()),
        );
        let inputs = edge.inputs.clone();
        // A file that cannot be watched is still saved; its next save finds a change made
        // elsewhere.
        let watcher = store::watch(&path, move || drop(inputs.send(Input::Check))).ok();
        disk.submit(Job::Backup {
            path: path.clone(),
            today,
        });
        let file = Self {
            path,
            source,
            known: version,
            theirs: None,
            disk,
            backups,
            recovery: Recovery::new(edge.recovery.clone()),
            layouts,
            clock: edge.clock,
            _watcher: watcher,
        };
        Ok((file, state))
    }

    /// Saves `document` as the core asked, after making sure of the backup dated `today`.
    pub(super) fn save(&self, document: &Document, today: Date) {
        self.disk.submit(Job::Backup {
            path: self.path.clone(),
            today,
        });
        self.disk.submit(Job::Save {
            path: self.path.clone(),
            text: self.text(document),
            expected: self.known,
        });
    }

    /// Keeps `layout` for the next launch of the file. A layout that cannot be kept only means
    /// the next launch starts at home, so the failure is not reported.
    pub(super) fn keep_layout(&self, layout: &Layout) {
        let _ = self.layouts.keep(&self.path, layout);
    }

    /// Waits until the disk work asked for so far is done and reported.
    pub(super) fn flush(&self) {
        self.disk.flush();
    }

    /// Asks the worker to read the file, as after the watcher saw a change.
    pub(super) fn check(&self) {
        self.disk.submit(Job::Check {
            path: self.path.clone(),
        });
    }

    /// Writes `side` to a conflict copy: on the worker, or at once with a recovery file in its
    /// place when the copy fails as the session is `ending`, returning the report then.
    pub(super) fn copy(
        &self,
        side: ConflictSide,
        document: &Document,
        ending: bool,
    ) -> Option<Message> {
        let contents = match side {
            ConflictSide::Mine => self.text(document).into_bytes(),
            ConflictSide::Theirs => self
                .theirs
                .as_ref()
                .map(|(bytes, _)| bytes.clone())
                .unwrap_or_default(),
        };
        let now = (self.clock)();
        if !ending {
            self.disk.submit(Job::ConflictCopy {
                path: self.path.clone(),
                contents,
                now,
            });
            return None;
        }
        let text = String::from_utf8_lossy(&contents);
        let kept = self
            .backups
            .conflict(&self.path, &contents, now)
            .map_err(|error| error.to_string())
            .or_else(|copy| {
                self.recovery
                    .write(Some(&self.path), &text, now)
                    .map_err(|error| format!("{copy}; nor a recovery file: {error}"))
            });
        Some(Message::Disk(match kept {
            Ok(copy) => Report::Copied(copy),
            Err(reason) => Report::CopyFailed(reason),
        }))
    }

    /// From now on the document is written the way the file another program left was read.
    pub(super) fn reloaded(&mut self) {
        if let Some((_, Some(source))) = self.theirs.take() {
            self.source = source;
        }
    }

    /// What the worker's `outcome` tells the core, in order.
    pub(super) fn outcome(&mut self, outcome: Outcome) -> Vec<Message> {
        match outcome {
            Outcome::Saved { version } => {
                self.known = Some(version);
                vec![Message::Disk(Report::Saved)]
            }
            // What is on disk comes first, so that the core does not save over it again at once.
            Outcome::SaveRefused { contents } => self
                .found(contents)
                .into_iter()
                .chain([Message::Disk(Report::SaveRefused)])
                .collect(),
            Outcome::SaveFailed { reason } => vec![Message::Disk(Report::SaveFailed(reason))],
            Outcome::BackupFailed { reason } => vec![Message::Disk(Report::BackupFailed(reason))],
            Outcome::ConflictCopied { copy } => vec![Message::Disk(Report::Copied(copy))],
            Outcome::ConflictCopyFailed { reason } => {
                vec![Message::Disk(Report::CopyFailed(reason))]
            }
            Outcome::Checked { contents } => self.found(contents).into_iter().collect(),
        }
    }

    /// What the file on disk holding `contents`, `None` when there is none, tells the core:
    /// nothing when it is the version known.
    fn found(&mut self, contents: Option<Vec<u8>>) -> Option<Message> {
        let version = contents.as_deref().map(Version::of);
        if version == self.known {
            return None;
        }
        self.known = version;
        let Some(bytes) = contents else {
            return Some(Message::OnDisk(OnDisk::Gone));
        };
        let (change, source) = match store::decode(&bytes) {
            Ok((document, source)) => (OnDisk::Changed(document), Some(source)),
            Err(_) => (OnDisk::Unreadable, None),
        };
        self.theirs = Some((bytes, source));
        Some(Message::OnDisk(change))
    }

    /// The file another program left on disk, as text, while the core has not taken it.
    pub(super) fn theirs(&self) -> Option<String> {
        let (bytes, _) = self.theirs.as_ref()?;
        Some(String::from_utf8_lossy(bytes).into_owned())
    }

    /// `document` as its file holds it.
    pub(super) fn text(&self, document: &Document) -> String {
        serialize(&write_document(document, &self.source))
    }

    /// Writes `document`, which holds changes the file lacks, to a recovery file, once the disk
    /// work asked for is done, so that a save under way cannot change the file after it
    /// (N-REL-01); returns its path.
    pub(super) fn recover(&self, document: &Document) -> Result<PathBuf, RecoveryError> {
        self.disk.flush();
        self.recovery
            .write(Some(&self.path), &self.text(document), (self.clock)())
    }
}
