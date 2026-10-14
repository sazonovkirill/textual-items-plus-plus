//! The opened file at the edge: carries out the core's saving effects with Store, on the disk
//! worker, and turns what the worker and the watcher report into messages for the core
//! ([file-io.md](../../../specs/reference/taskpaper-terminal/specs/file-io.md)). Hides which
//! version of the file the app last read or wrote, and the outline it is written back with.

use std::{
    collections::VecDeque,
    path::{Path, PathBuf},
    sync::{Arc, mpsc::Sender},
};

use jiff::civil::{Date, DateTime};

use super::Input;
use crate::{
    app::{
        Animations, ConflictSide, CopyId, Layout, Message, OnDisk, Opened, Report, SaveAsReport,
        State,
    },
    domain::Document,
    store::{
        self, Backups, Disk, Job, Layouts, OpenError, OpenFile, Outcome, Recovery, RecoveryError,
        Version, Watcher, Written,
        taskpaper::{Source, parse, read_document},
    },
};

/// Where the app keeps its own files, the local clock, how much it animates unless its settings
/// say, and where the disk worker and the watcher report to the event loop.
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
    /// The home folder, which `~` stands for in a path typed (FR-STORE-11), `None` when there is
    /// none.
    pub home: Option<PathBuf>,
    /// The local time now.
    pub clock: fn() -> DateTime,
    /// How much the app animates when the settings file does not say (FR-UI-07).
    pub animations: Animations,
}

/// The file a session opened, as it is known on disk.
#[derive(Debug)]
pub(super) struct File {
    path: PathBuf,
    /// The outline the document is written back with, shared with the saves under way.
    source: Arc<Source>,
    /// The version of the file the app last read or wrote, `None` while there is none.
    known: Option<Version>,
    /// The file as another program left it, with the outline it reads as when it can be read,
    /// until the core takes it.
    theirs: Option<(Vec<u8>, Option<Source>)>,
    disk: Disk,
    /// The conflict copies queued on the worker, which reports them in this order.
    copies: VecDeque<CopyId>,
    /// Conflict copies the session ends with are written at once, not on the worker.
    backups: Backups,
    recovery: Recovery,
    layouts: Layouts,
    clock: fn() -> DateTime,
    /// Where the watcher reports.
    inputs: Sender<Input>,
    /// Watches the file while it is kept.
    watcher: Option<Watcher>,
    /// The file Save As is writing, which this becomes once it is written (FR-STORE-11).
    moving: Option<PathBuf>,
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
        let file = Self::start(path, source, version, today, edge);
        let state = match file.layouts.load(&file.path) {
            Some(layout) => state.with_layout(&layout),
            None => state,
        };
        Ok((file, state))
    }

    /// Writes `document`, which has no file yet, as the demo document has none, to a new file at
    /// `path`, or over the file there when told to `replace` it after keeping it in a conflict
    /// copy, and opens that file, which the document is saved to from then on (FR-STORE-11).
    ///
    /// # Errors
    ///
    /// How writing failed, as the core takes it.
    pub(super) fn create(
        path: PathBuf,
        document: &Document,
        replace: bool,
        today: Date,
        edge: &Edge,
    ) -> Result<Self, SaveAsReport> {
        let (_, source) = read_document(parse(""));
        let text = store::encode(document, &source);
        let backups = Backups::new(edge.backups.clone());
        match store::save_as(&path, &text, replace, (edge.clock)(), &backups) {
            Written::Saved(version) => Ok(Self::start(path, source, Some(version), today, edge)),
            Written::Exists => Err(SaveAsReport::Exists),
            Written::Failed(reason) => Err(SaveAsReport::Failed(reason)),
        }
    }

    /// The file at `path`, read as `source` at `version`, `None` when there is none: starts the
    /// disk worker, the watcher and, when the file exists, today's backup.
    fn start(
        path: PathBuf,
        source: Source,
        version: Option<Version>,
        today: Date,
        edge: &Edge,
    ) -> Self {
        let inputs = edge.inputs.clone();
        let disk = Disk::start(
            // The event loop has ended once nothing receives, so the report has nowhere to go.
            move |outcome| drop(inputs.send(Input::Disk(outcome))),
            Backups::new(edge.backups.clone()),
        );
        disk.submit(Job::Backup {
            path: path.clone(),
            today,
        });
        Self {
            watcher: watch(&path, &edge.inputs),
            path,
            source: Arc::new(source),
            known: version,
            theirs: None,
            disk,
            copies: VecDeque::new(),
            backups: Backups::new(edge.backups.clone()),
            recovery: Recovery::new(edge.recovery.clone()),
            layouts: Layouts::new(edge.layouts.clone()),
            clock: edge.clock,
            inputs: edge.inputs.clone(),
            moving: None,
        }
    }

    /// Writes `document` to the file at `path` after the disk work asked for so far, where there
    /// is none, or over the file there when told to `replace` it after keeping it in a conflict
    /// copy; once it is written, this is that file (FR-STORE-11). Until then the file is not
    /// checked, since the check of the file this leaves would be reported after it.
    pub(super) fn save_as(&mut self, path: PathBuf, document: &Document, replace: bool) {
        self.disk.submit(Job::SaveAs {
            path: path.clone(),
            document: document.clone(),
            source: Arc::clone(&self.source),
            replace,
            now: (self.clock)(),
        });
        self.moving = Some(path);
    }

    /// Saves `document` as the core asked, after making sure of the backup dated `today`.
    pub(super) fn save(&self, document: &Document, today: Date) {
        self.disk.submit(Job::Backup {
            path: self.path.clone(),
            today,
        });
        self.disk.submit(Job::Save {
            path: self.path.clone(),
            document: document.clone(),
            source: Arc::clone(&self.source),
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

    /// Asks the worker to read the file, as after the watcher saw a change, unless Save As is
    /// writing another.
    pub(super) fn check(&self) {
        if self.moving.is_none() {
            self.disk.submit(Job::Check {
                path: self.path.clone(),
            });
        }
    }

    /// Writes `side` to the conflict copy `id`: on the worker, or at once with a recovery file in
    /// its place when the copy fails as the session is `ending`, returning the report then.
    pub(super) fn copy(
        &mut self,
        side: ConflictSide,
        document: &Document,
        ending: bool,
        id: CopyId,
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
            self.copies.push_back(id);
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
            Ok(copy) => Report::Copied(id, copy),
            Err(reason) => Report::CopyFailed(id, reason),
        }))
    }

    /// From now on the document is written the way the file another program left was read.
    pub(super) fn reloaded(&mut self) {
        if let Some((_, Some(source))) = self.theirs.take() {
            self.source = Arc::new(source);
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
            Outcome::ConflictCopied { copy } => {
                vec![Message::Disk(Report::Copied(self.copy_queued(), copy))]
            }
            Outcome::ConflictCopyFailed { reason } => {
                vec![Message::Disk(Report::CopyFailed(
                    self.copy_queued(),
                    reason,
                ))]
            }
            Outcome::Checked { contents } => self.found(contents).into_iter().collect(),
            Outcome::SavedAs(written) => vec![Message::Disk(Report::SaveAs(self.moved(written)))],
        }
    }

    /// What writing the file Save As asked for tells the core: once it is written, this is that
    /// file, at the version written, watched in place of the one before; otherwise this file is
    /// read again.
    fn moved(&mut self, written: Written) -> SaveAsReport {
        let path = self
            .moving
            .take()
            .expect("the worker reports only the Save As asked of it");
        match written {
            Written::Saved(version) => {
                self.watcher = watch(&path, &self.inputs);
                self.path.clone_from(&path);
                self.known = Some(version);
                self.theirs = None;
                SaveAsReport::Saved(path)
            }
            // The watcher's news of this file went unchecked while writing.
            Written::Exists => {
                self.check();
                SaveAsReport::Exists
            }
            Written::Failed(reason) => {
                self.check();
                SaveAsReport::Failed(reason)
            }
        }
    }

    /// The conflict copy the worker just reported, the first of those queued.
    fn copy_queued(&mut self) -> CopyId {
        self.copies
            .pop_front()
            .expect("the worker reports only the conflict copies queued on it")
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
        store::encode(document, &self.source)
    }

    /// Writes `document` to a recovery file once the disk work asked for is done, so that a
    /// save under way cannot change the file after it, unless that save left the file holding
    /// it (N-REL-01, D-96); returns its path, or `None` when there is nothing to keep.
    pub(super) fn recover(&self, document: &Document) -> Result<Option<PathBuf>, RecoveryError> {
        self.disk.flush();
        self.recovery
            .keep(&self.path, &self.text(document), (self.clock)())
    }
}

/// Watches the file at `path`, telling `inputs` to check it. A file that cannot be watched is
/// still saved; its next save finds a change made elsewhere.
fn watch(path: &Path, inputs: &Sender<Input>) -> Option<Watcher> {
    let inputs = inputs.clone();
    store::watch(path, move || drop(inputs.send(Input::Check))).ok()
}

/// The path `text` names: `~` alone or before a `/` stands for `home` when there is one, and a
/// relative path is taken from the working folder (FR-STORE-11).
pub(super) fn expand(text: &str, home: Option<&Path>) -> PathBuf {
    let path = match (text.strip_prefix('~'), home) {
        (Some(""), Some(home)) => home.to_owned(),
        (Some(rest), Some(home)) if rest.starts_with('/') => {
            home.join(rest.trim_start_matches('/'))
        }
        _ => PathBuf::from(text),
    };
    std::path::absolute(&path).unwrap_or(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    // FR-STORE-11
    #[test]
    fn a_path_typed_stands_for_the_home_folder_with_a_tilde_and_is_made_absolute() {
        let home = Path::new("/home/ann");
        assert_eq!(expand("~", Some(home)), home);
        assert_eq!(
            expand("~/notes/a.taskpaper", Some(home)),
            Path::new("/home/ann/notes/a.taskpaper")
        );
        assert_eq!(
            expand("/tmp/a.taskpaper", Some(home)),
            Path::new("/tmp/a.taskpaper")
        );
        let here = std::env::current_dir().expect("a working folder");
        assert_eq!(
            expand("~ann/a", Some(home)),
            here.join("~ann/a"),
            "not a home"
        );
        assert_eq!(expand("~/a", None), here.join("~/a"), "no home to expand");
        assert_eq!(expand("a.taskpaper", Some(home)), here.join("a.taskpaper"));
    }
}
