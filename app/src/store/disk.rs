//! The disk worker: does the disk work of saving in order on a thread of its own, so that the
//! event loop never waits on the disk (N-UX-01), and reports each result.

use std::{
    fs, io,
    path::{Path, PathBuf},
    sync::{
        Arc,
        mpsc::{self, Receiver, Sender},
    },
    thread::{self, JoinHandle},
};

use jiff::civil::{Date, DateTime};

use super::{Backups, Version, atomic, encode, taskpaper::Source};
use crate::domain::Document;

/// Disk work, done in the order it is submitted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Job {
    /// Replace the file at `path` with `document`, written back with `source` (F-FILE-04), unless
    /// just before that the file is no longer at `expected`, `None` meaning no file, because
    /// another program changed it (F-FILE-06). The worker writes the text, which for a large file
    /// takes longer than a frame may (N-PERF-02). Reports [`Outcome::Saved`],
    /// [`Outcome::SaveRefused`] or [`Outcome::SaveFailed`].
    Save {
        /// The file.
        path: PathBuf,
        /// What it should hold.
        document: Document,
        /// The outline it is written back with.
        source: Arc<Source>,
        /// The version the file should be at now.
        expected: Option<Version>,
    },
    /// Make sure the file at `path` has its backup dated `today`, the local date (F-FILE-09).
    /// Reports only [`Outcome::BackupFailed`].
    Backup {
        /// The file.
        path: PathBuf,
        /// The local date.
        today: Date,
    },
    /// Write `contents`, a version of the file at `path` that a conflict's choice discards, to a
    /// conflict copy named by the local time `now` (F-FILE-06). Reports
    /// [`Outcome::ConflictCopied`] or [`Outcome::ConflictCopyFailed`].
    ConflictCopy {
        /// The file.
        path: PathBuf,
        /// The version to keep.
        contents: Vec<u8>,
        /// The local time.
        now: DateTime,
    },
    /// Read the file at `path`, to tell whether another program changed it (F-FILE-06). Reports
    /// [`Outcome::Checked`], or nothing when the file cannot be read, since the next save reports
    /// why.
    Check {
        /// The file.
        path: PathBuf,
    },
    /// Write `document`, with `source`, to the file at `path`, another file than the one saved
    /// so far: only when there is none there, unless `replace`, which first keeps the file it
    /// replaces in a conflict copy named by the local time `now`, so that it is never lost
    /// (FR-STORE-11). Reports [`Outcome::SavedAs`].
    SaveAs {
        /// The file.
        path: PathBuf,
        /// What it should hold.
        document: Document,
        /// The outline it is written back with.
        source: Arc<Source>,
        /// Whether a file there is replaced.
        replace: bool,
        /// The local time.
        now: DateTime,
    },
}

/// How a [`Job`] went.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// The save was done, leaving the file at `version`.
    Saved {
        /// The version written.
        version: Version,
    },
    /// The save was not done, because the file was not at the version expected.
    SaveRefused {
        /// The file's bytes now, or `None` when there is no file.
        contents: Option<Vec<u8>>,
    },
    /// The save failed.
    SaveFailed {
        /// Why, in words for the status line.
        reason: String,
    },
    /// The backup failed.
    BackupFailed {
        /// Why, in words for the status line.
        reason: String,
    },
    /// The conflict copy was written.
    ConflictCopied {
        /// Where.
        copy: PathBuf,
    },
    /// The conflict copy could not be written.
    ConflictCopyFailed {
        /// Why, in words for the status line.
        reason: String,
    },
    /// The file was read.
    Checked {
        /// Its bytes, or `None` when there is no file.
        contents: Option<Vec<u8>>,
    },
    /// The file of a [`Job::SaveAs`] was written, or why not.
    SavedAs(Written),
}

/// How writing the document to another file went (FR-STORE-11).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Written {
    /// The file was written, at this version.
    Saved(Version),
    /// Nothing was written, because a file is there and was not to be replaced.
    Exists,
    /// The file could not be written, for this reason, in words for the status line.
    Failed(String),
}

/// The queue of disk work and the thread that does it. Dropping it waits for the queued work.
#[derive(Debug)]
pub struct Disk {
    /// `None` only while dropping, to close the queue before waiting for the thread.
    requests: Option<Sender<Request>>,
    worker: Option<JoinHandle<()>>,
}

#[derive(Debug)]
enum Request {
    Job(Job),
    /// Answers once every job before it is done and reported.
    Flush(Sender<()>),
}

impl Disk {
    /// Starts the worker, which makes backups with `backups` and gives each outcome to `report`
    /// on its own thread.
    #[must_use]
    pub fn start(report: impl FnMut(Outcome) + Send + 'static, backups: Backups) -> Self {
        let (requests, queue) = mpsc::channel();
        let worker = thread::spawn(move || work(&queue, report, backups));
        Self {
            requests: Some(requests),
            worker: Some(worker),
        }
    }

    /// Queues `job` behind the work queued before it.
    pub fn submit(&self, job: Job) {
        self.send(Request::Job(job));
    }

    /// Waits until the work queued so far is done and reported, as before writing a recovery
    /// file, so that a save under way cannot change the file after it.
    ///
    /// # Panics
    ///
    /// When `report` panicked on the worker's thread, which then stopped.
    pub fn flush(&self) {
        let (done, finished) = mpsc::channel();
        self.send(Request::Flush(done));
        finished
            .recv()
            .expect("the worker stopped: a report panicked");
    }

    fn send(&self, request: Request) {
        self.requests
            .as_ref()
            .and_then(|requests| requests.send(request).ok())
            .expect("the worker stopped: a report panicked");
    }
}

impl Drop for Disk {
    fn drop(&mut self) {
        drop(self.requests.take());
        if let Some(worker) = self.worker.take() {
            // A worker that panicked has had its panic reported on its own thread, and there is
            // nothing left to do here.
            let _ = worker.join();
        }
    }
}

/// Does the requests from `queue` until it closes.
fn work(queue: &Receiver<Request>, mut report: impl FnMut(Outcome), mut backups: Backups) {
    for request in queue {
        let outcome =
            match request {
                Request::Job(Job::Save {
                    path,
                    document,
                    source,
                    expected,
                }) => Some(save(&path, &encode(&document, &source), expected)),
                Request::Job(Job::Backup { path, today }) => backups
                    .ensure(&path, today)
                    .err()
                    .map(|error| Outcome::BackupFailed {
                        reason: error.to_string(),
                    }),
                Request::Job(Job::ConflictCopy {
                    path,
                    contents,
                    now,
                }) => Some(match backups.conflict(&path, &contents, now) {
                    Ok(copy) => Outcome::ConflictCopied { copy },
                    Err(error) => Outcome::ConflictCopyFailed {
                        reason: error.to_string(),
                    },
                }),
                Request::Job(Job::Check { path }) => read(&path)
                    .ok()
                    .map(|contents| Outcome::Checked { contents }),
                Request::Job(Job::SaveAs {
                    path,
                    document,
                    source,
                    replace,
                    now,
                }) => Some(Outcome::SavedAs(save_as(
                    &path,
                    &encode(&document, &source),
                    replace,
                    now,
                    &backups,
                ))),
                Request::Flush(done) => {
                    // The flusher waits for this answer and only stops waiting by being dropped.
                    let _ = done.send(());
                    None
                }
            };
        if let Some(outcome) = outcome {
            report(outcome);
        }
    }
}

/// Writes `text` to `path`, unless just before the rename the file is no longer at `expected`.
fn save(path: &Path, text: &str, expected: Option<Version>) -> Outcome {
    let saved = atomic::prepare(path, text.as_bytes()).and_then(|prepared| {
        let found = read(prepared.target())?;
        if found.as_deref().map(Version::of) == expected {
            prepared.commit()?;
            Ok(Outcome::Saved {
                version: Version::of(text.as_bytes()),
            })
        } else {
            Ok(Outcome::SaveRefused { contents: found })
        }
    });
    saved.unwrap_or_else(|error| Outcome::SaveFailed {
        reason: reason(path, &error),
    })
}

/// Writes `text` to the file at `path` when there is none, or, when told to `replace` it, over
/// the file there after keeping it in a conflict copy of `backups` named by the local time `now`
/// (FR-STORE-11). A file that another program creates meanwhile is not replaced unasked.
#[must_use]
pub fn save_as(
    path: &Path,
    text: &str,
    replace: bool,
    now: DateTime,
    backups: &Backups,
) -> Written {
    let saved = Written::Saved(Version::of(text.as_bytes()));
    if !replace {
        return match atomic::create(path, text.as_bytes(), None) {
            Ok(()) => saved,
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => Written::Exists,
            Err(error) => Written::Failed(reason(path, &error)),
        };
    }
    let replaced = atomic::prepare(path, text.as_bytes()).and_then(|prepared| {
        if let Some(bytes) = read(prepared.target())? {
            backups
                .conflict(prepared.target(), &bytes, now)
                .map_err(|error| {
                    io::Error::other(format!("cannot keep the file it replaces: {error}"))
                })?;
        }
        prepared.commit()
    });
    match replaced {
        Ok(()) => saved,
        Err(error) => Written::Failed(reason(path, &error)),
    }
}

/// Why writing the file at `path` failed with `error`: a missing folder in plain words, rather
/// than the error about the temporary file beside the file.
fn reason(path: &Path, error: &io::Error) -> String {
    let folder = std::path::absolute(path)
        .ok()
        .and_then(|path| path.parent().map(Path::to_owned))
        .filter(|folder| !folder.is_dir());
    match folder {
        Some(folder) if error.kind() == io::ErrorKind::NotFound => {
            format!("the folder {} does not exist", folder.display())
        }
        _ => error.to_string(),
    }
}

/// The bytes of the file at `path`, or `None` when there is none.
pub(super) fn read(path: &Path) -> io::Result<Option<Vec<u8>>> {
    match fs::read(path) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error),
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use jiff::civil::date;

    use super::*;
    use crate::store::decode;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    const TODAY: Date = date(2026, 10, 5);

    /// A worker with backups in `dir/backups`, and the outcomes it reported so far.
    fn start(dir: &Path) -> (Disk, Arc<Mutex<Vec<Outcome>>>) {
        let outcomes = Arc::new(Mutex::new(Vec::new()));
        let reported = Arc::clone(&outcomes);
        let report = move |outcome| reported.lock().expect("no panic").push(outcome);
        let disk = Disk::start(report, Backups::new(Some(dir.join("backups"))));
        (disk, outcomes)
    }

    fn taken(outcomes: &Mutex<Vec<Outcome>>) -> Vec<Outcome> {
        std::mem::take(&mut *outcomes.lock().expect("no panic"))
    }

    /// A save of `text`, which writes back as it reads, to `path`, where the file holds
    /// `expected`, or nothing.
    fn save_over(path: &Path, text: &str, expected: Option<&str>) -> Job {
        let (document, source) = decode(text.as_bytes()).expect("the text is UTF-8");
        assert_eq!(encode(&document, &source), text);
        Job::Save {
            path: path.to_owned(),
            document,
            source: Arc::new(source),
            expected: expected.map(|text| Version::of(text.as_bytes())),
        }
    }

    fn saved(text: &str) -> Outcome {
        Outcome::Saved {
            version: Version::of(text.as_bytes()),
        }
    }

    // F-FILE-04, N-UX-01
    #[test]
    fn saves_in_order_and_reports_each_result() -> TestResult {
        let dir = tempfile::tempdir()?;
        let (disk, outcomes) = start(dir.path());
        let path = dir.path().join("todo.taskpaper");
        disk.submit(save_over(&path, "Inbox:\none", None));
        disk.submit(save_over(
            &dir.path().join("missing/x.taskpaper"),
            "Inbox:\n",
            None,
        ));
        disk.submit(save_over(&path, "Inbox:\ntwo", Some("Inbox:\none")));
        disk.flush();
        assert_eq!(
            taken(&outcomes),
            [
                saved("Inbox:\none"),
                Outcome::SaveFailed {
                    reason: format!(
                        "the folder {} does not exist",
                        dir.path().join("missing").display()
                    )
                },
                saved("Inbox:\ntwo"),
            ]
        );
        assert_eq!(fs::read_to_string(&path)?, "Inbox:\ntwo");
        Ok(())
    }

    // F-FILE-06
    #[test]
    fn a_save_is_refused_when_the_file_changed_and_leaves_it_alone() -> TestResult {
        let dir = tempfile::tempdir()?;
        let (disk, outcomes) = start(dir.path());
        let path = dir.path().join("todo.taskpaper");
        fs::write(&path, "theirs")?;
        disk.submit(save_over(&path, "Inbox:\n", Some("known")));
        disk.submit(save_over(&path, "Inbox:\n", None));
        let gone = dir.path().join("gone.taskpaper");
        disk.submit(save_over(&gone, "Inbox:\n", Some("known")));
        disk.flush();
        let theirs = Some(b"theirs".to_vec());
        assert_eq!(
            taken(&outcomes),
            [
                Outcome::SaveRefused {
                    contents: theirs.clone()
                },
                Outcome::SaveRefused { contents: theirs },
                Outcome::SaveRefused { contents: None },
            ]
        );
        assert_eq!(fs::read_to_string(&path)?, "theirs");
        assert!(!fs::exists(&gone)?);
        assert_eq!(
            fs::read_dir(dir.path())?.count(),
            1,
            "no temporary file is left"
        );
        Ok(())
    }

    #[test]
    fn a_save_under_a_file_says_what_the_system_reported() -> TestResult {
        let dir = tempfile::tempdir()?;
        let (disk, outcomes) = start(dir.path());
        let file = dir.path().join("file");
        fs::write(&file, "")?;
        disk.submit(save_over(&file.join("x.taskpaper"), "Inbox:\n", None));
        disk.flush();
        let outcomes = taken(&outcomes);
        assert!(
            matches!(
                outcomes.as_slice(),
                [Outcome::SaveFailed { reason }] if reason.starts_with("Not a directory")
            ),
            "only a folder that is missing is reported as missing: {outcomes:?}"
        );
        Ok(())
    }

    #[test]
    fn a_save_fails_when_the_file_cannot_be_read() -> TestResult {
        let dir = tempfile::tempdir()?;
        let (disk, outcomes) = start(dir.path());
        let link = dir.path().join("loop.taskpaper");
        std::os::unix::fs::symlink(&link, &link)?;
        disk.submit(save_over(&link, "Inbox:\n", None));
        let folder = dir.path().join("folder.taskpaper");
        fs::create_dir(&folder)?;
        disk.submit(save_over(&folder, "Inbox:\n", None));
        disk.flush();
        let outcomes = taken(&outcomes);
        assert!(
            matches!(
                outcomes.as_slice(),
                [Outcome::SaveFailed { reason: looped }, Outcome::SaveFailed { reason: folder }]
                    if looped.contains("symbolic links") && folder.contains("directory")
            ),
            "{outcomes:?}"
        );
        assert!(folder.is_dir(), "a folder is never replaced");
        Ok(())
    }

    // F-FILE-09
    #[test]
    fn backs_up_before_a_save_queued_after_it() -> TestResult {
        let dir = tempfile::tempdir()?;
        let (disk, outcomes) = start(dir.path());
        let path = dir.path().join("todo.taskpaper");
        fs::write(&path, "on disk")?;
        disk.submit(Job::Backup {
            path: path.clone(),
            today: TODAY,
        });
        disk.submit(save_over(&path, "Inbox:\nsaved", Some("on disk")));
        drop(disk);
        assert_eq!(taken(&outcomes), [saved("Inbox:\nsaved")], "dropping waits");
        let folder = fs::read_dir(dir.path().join("backups"))?
            .next()
            .ok_or("a backup folder")??
            .path();
        assert_eq!(
            fs::read_to_string(folder.join("2026-10-05.taskpaper"))?,
            "on disk"
        );
        Ok(())
    }

    // F-FILE-09
    #[test]
    fn reports_a_failed_backup() -> TestResult {
        let dir = tempfile::tempdir()?;
        let (disk, outcomes) = start(dir.path());
        disk.submit(Job::Backup {
            path: dir.path().to_owned(),
            today: TODAY,
        });
        disk.flush();
        assert_eq!(
            taken(&outcomes),
            [Outcome::BackupFailed {
                reason: "Is a directory (os error 21)".to_owned()
            }]
        );
        Ok(())
    }

    // F-FILE-06
    #[test]
    fn writes_conflict_copies_and_reports_where_or_why_not() -> TestResult {
        let dir = tempfile::tempdir()?;
        let (disk, outcomes) = start(dir.path());
        let path = dir.path().join("todo.taskpaper");
        fs::write(&path, "on disk")?;
        let now = TODAY.at(9, 30, 0, 0);
        disk.submit(Job::ConflictCopy {
            path: path.clone(),
            contents: b"theirs".to_vec(),
            now,
        });
        disk.submit(Job::ConflictCopy {
            path: dir.path().join("missing/todo.taskpaper"),
            contents: b"mine".to_vec(),
            now,
        });
        disk.flush();
        let outcomes = taken(&outcomes);
        assert!(
            matches!(
                outcomes.as_slice(),
                [Outcome::ConflictCopied { copy }, Outcome::ConflictCopyFailed { reason }]
                    if fs::read_to_string(copy).is_ok_and(|copied| copied == "theirs")
                        && reason.contains("No such file")
            ),
            "{outcomes:?}"
        );
        Ok(())
    }

    // F-FILE-06
    #[test]
    fn checks_report_the_file_or_its_absence() -> TestResult {
        let dir = tempfile::tempdir()?;
        let (disk, outcomes) = start(dir.path());
        let path = dir.path().join("todo.taskpaper");
        fs::write(&path, "on disk")?;
        disk.submit(Job::Check { path });
        disk.submit(Job::Check {
            path: dir.path().join("gone.taskpaper"),
        });
        disk.submit(Job::Check {
            path: dir.path().to_owned(),
        });
        disk.flush();
        assert_eq!(
            taken(&outcomes),
            [
                Outcome::Checked {
                    contents: Some(b"on disk".to_vec())
                },
                Outcome::Checked { contents: None },
            ],
            "a path that cannot be read reports nothing"
        );
        Ok(())
    }

    /// Writing `text`, which writes back as it reads, to `path` as another file, replacing one
    /// there only when told to.
    fn save_as_job(path: &Path, text: &str, replace: bool) -> Job {
        let (document, source) = decode(text.as_bytes()).expect("the text is UTF-8");
        Job::SaveAs {
            path: path.to_owned(),
            document,
            source: Arc::new(source),
            replace,
            now: TODAY.at(9, 30, 0, 0),
        }
    }

    fn saved_as(text: &str) -> Outcome {
        Outcome::SavedAs(Written::Saved(Version::of(text.as_bytes())))
    }

    /// The contents of each file in the folders of `backups`.
    fn kept(backups: &Path) -> io::Result<Vec<String>> {
        let mut kept = Vec::new();
        for folder in fs::read_dir(backups)? {
            for file in fs::read_dir(folder?.path())? {
                let path = file?.path();
                if path
                    .extension()
                    .is_some_and(|extension| extension == "taskpaper")
                {
                    kept.push(fs::read_to_string(path)?);
                }
            }
        }
        Ok(kept)
    }

    // FR-STORE-11
    #[test]
    fn saves_as_a_new_file_replacing_one_only_when_told_after_keeping_it() -> TestResult {
        let dir = tempfile::tempdir()?;
        let (disk, outcomes) = start(dir.path());
        let path = dir.path().join("new.taskpaper");
        let missing = dir.path().join("missing/x.taskpaper");
        disk.submit(save_as_job(&path, "Inbox:\none", false));
        disk.submit(save_as_job(&path, "Inbox:\ntwo", false));
        disk.submit(save_as_job(&path, "Inbox:\ntwo", true));
        disk.submit(save_as_job(&missing, "Inbox:\n", false));
        disk.submit(save_as_job(&missing, "Inbox:\n", true));
        disk.flush();
        let folder = format!(
            "the folder {} does not exist",
            dir.path().join("missing").display()
        );
        assert_eq!(
            taken(&outcomes),
            [
                saved_as("Inbox:\none"),
                Outcome::SavedAs(Written::Exists),
                saved_as("Inbox:\ntwo"),
                Outcome::SavedAs(Written::Failed(folder.clone())),
                Outcome::SavedAs(Written::Failed(folder)),
            ]
        );
        assert_eq!(fs::read_to_string(&path)?, "Inbox:\ntwo");
        assert_eq!(kept(&dir.path().join("backups"))?, ["Inbox:\none"]);
        let fresh = dir.path().join("fresh.taskpaper");
        disk.submit(save_as_job(&fresh, "Inbox:\n", true));
        disk.flush();
        assert_eq!(
            taken(&outcomes),
            [saved_as("Inbox:\n")],
            "nothing to replace"
        );
        Ok(())
    }

    // FR-STORE-11: the file it replaces is never lost.
    #[test]
    fn a_file_that_cannot_be_kept_is_not_replaced() -> TestResult {
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("todo.taskpaper");
        fs::write(&path, "theirs")?;
        let written = save_as(
            &path,
            "Inbox:\n",
            true,
            TODAY.at(9, 30, 0, 0),
            &Backups::new(None),
        );
        assert!(
            matches!(&written, Written::Failed(reason) if reason.starts_with("cannot keep the file it replaces: ")),
            "{written:?}"
        );
        assert_eq!(fs::read_to_string(&path)?, "theirs");
        Ok(())
    }

    #[test]
    #[should_panic(expected = "the worker stopped: a report panicked")]
    fn flushing_a_worker_whose_report_panicked_panics() {
        let dir = tempfile::tempdir().expect("a folder");
        let disk = Disk::start(|_| panic!("report"), Backups::new(None));
        disk.submit(Job::Check {
            path: dir.path().join("gone.taskpaper"),
        });
        disk.flush();
    }
}
