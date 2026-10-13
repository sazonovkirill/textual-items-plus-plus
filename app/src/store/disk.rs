//! The disk worker: does the disk work of saving in order on a thread of its own, so that the
//! event loop never waits on the disk (N-UX-01), and reports each result.

use std::{
    fs, io,
    path::{Path, PathBuf},
    sync::mpsc::{self, Receiver, Sender},
    thread::{self, JoinHandle},
};

use jiff::civil::Date;

use super::{Backups, Version, atomic};

/// Disk work, done in the order it is submitted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Job {
    /// Replace the file at `path` with `text` (F-FILE-04), unless just before that the file is no
    /// longer at `expected`, `None` meaning no file, because another program changed it
    /// (F-FILE-06). Reports [`Outcome::Saved`], [`Outcome::SaveRefused`] or
    /// [`Outcome::SaveFailed`].
    Save {
        /// The file.
        path: PathBuf,
        /// What it should hold.
        text: String,
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
    /// Read the file at `path`, to tell whether another program changed it (F-FILE-06). Reports
    /// [`Outcome::Checked`], or nothing when the file cannot be read, since the next save reports
    /// why.
    Check {
        /// The file.
        path: PathBuf,
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
    /// The file was read.
    Checked {
        /// Its bytes, or `None` when there is no file.
        contents: Option<Vec<u8>>,
    },
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
                    text,
                    expected,
                }) => Some(save(&path, &text, expected)),
                Request::Job(Job::Backup { path, today }) => backups
                    .ensure(&path, today)
                    .err()
                    .map(|error| Outcome::BackupFailed {
                        reason: error.to_string(),
                    }),
                Request::Job(Job::Check { path }) => read(&path)
                    .ok()
                    .map(|contents| Outcome::Checked { contents }),
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
fn read(path: &Path) -> io::Result<Option<Vec<u8>>> {
    match fs::read(path) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error),
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use jiff::civil::date;

    use super::*;

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

    /// A save of `text` to `path`, where the file holds `expected`, or nothing.
    fn save_over(path: &Path, text: &str, expected: Option<&str>) -> Job {
        Job::Save {
            path: path.to_owned(),
            text: text.to_owned(),
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
        disk.submit(save_over(&path, "one", None));
        disk.submit(save_over(
            &dir.path().join("missing/x.taskpaper"),
            "x",
            None,
        ));
        disk.submit(save_over(&path, "two", Some("one")));
        disk.flush();
        assert_eq!(
            taken(&outcomes),
            [
                saved("one"),
                Outcome::SaveFailed {
                    reason: format!(
                        "the folder {} does not exist",
                        dir.path().join("missing").display()
                    )
                },
                saved("two"),
            ]
        );
        assert_eq!(fs::read_to_string(&path)?, "two");
        Ok(())
    }

    // F-FILE-06
    #[test]
    fn a_save_is_refused_when_the_file_changed_and_leaves_it_alone() -> TestResult {
        let dir = tempfile::tempdir()?;
        let (disk, outcomes) = start(dir.path());
        let path = dir.path().join("todo.taskpaper");
        fs::write(&path, "theirs")?;
        disk.submit(save_over(&path, "mine", Some("known")));
        disk.submit(save_over(&path, "mine", None));
        let gone = dir.path().join("gone.taskpaper");
        disk.submit(save_over(&gone, "mine", Some("known")));
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
        disk.submit(save_over(&file.join("x.taskpaper"), "mine", None));
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
        disk.submit(save_over(&link, "mine", None));
        let folder = dir.path().join("folder.taskpaper");
        fs::create_dir(&folder)?;
        disk.submit(save_over(&folder, "mine", None));
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
        disk.submit(save_over(&path, "saved", Some("on disk")));
        drop(disk);
        assert_eq!(taken(&outcomes), [saved("saved")], "dropping waits");
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
