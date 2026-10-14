//! Recovery files: when the app crashes, the document with the changes its file lacks goes to a
//! file of its own in the recovery folder, so that a panic never loses data (N-REL-01).

use std::{
    ffi::{OsStr, OsString},
    fs,
    io::{self, BufRead, Write},
    path::{Path, PathBuf},
};

use jiff::{SignedDuration, civil::DateTime};
use thiserror::Error;

use super::{atomic, backup};

/// What the recovery file of a document without a file is named after.
const UNTITLED: &str = "untitled";

/// The local time in a recovery file's name.
const STAMP: &str = "%Y%m%d-%H%M%S";

/// Why a recovery file could not be written.
#[derive(Debug, Error)]
pub enum RecoveryError {
    /// Neither `$XDG_STATE_HOME` nor a home folder says where recovery files go.
    #[error("no folder for recovery files: neither $XDG_STATE_HOME nor a home folder is set")]
    NoFolder,
    /// Writing the file failed.
    #[error(transparent)]
    Io(#[from] io::Error),
}

/// Where recovery files go.
#[derive(Debug, Clone)]
pub struct Recovery {
    folder: Option<PathBuf>,
}

impl Recovery {
    /// Recovery files in `folder`, or nowhere when there is none, which every write reports.
    #[must_use]
    pub fn new(folder: Option<PathBuf>) -> Self {
        Self { folder }
    }

    /// Writes `text`, the document opened from `file` or from no file, to a new recovery file
    /// `<name>-<YYYYMMDD-HHMMSS>.taskpaper` named by the local time `now`, or the first later
    /// second no recovery file is named by, so it never replaces another; returns its path.
    /// `<name>` is the name of the file's backup folder, which tells apart files with one name, or
    /// `untitled`. The recovery file is as private as the file. Recovery files are never deleted.
    ///
    /// # Errors
    ///
    /// [`RecoveryError`] when there is no recovery folder or writing failed.
    pub fn write(
        &self,
        file: Option<&Path>,
        text: &str,
        now: DateTime,
    ) -> Result<PathBuf, RecoveryError> {
        let folder = self.folder.as_deref().ok_or(RecoveryError::NoFolder)?;
        let name = match file {
            Some(file) => name(file)?,
            None => OsString::from(UNTITLED),
        };
        fs::create_dir_all(folder)?;
        let permissions = file
            .and_then(|file| fs::metadata(file).ok())
            .map(|metadata| metadata.permissions());
        let mut time = now;
        loop {
            let path = folder.join(file_name(&name, time));
            match atomic::create(&path, text.as_bytes(), permissions.clone()) {
                Ok(()) => return Ok(path),
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
                    time = time.saturating_add(SignedDuration::from_secs(1));
                }
                Err(error) => return Err(error.into()),
            }
        }
    }

    /// Writes `text`, the document opened from `file`, to a new recovery file as
    /// [`write`](Self::write) does, unless `file` holds it already, as when the save a session
    /// ended with landed; returns its path, or `None` when there is nothing to keep.
    ///
    /// # Errors
    ///
    /// [`RecoveryError`] when there is no recovery folder or writing failed.
    pub fn keep(
        &self,
        file: &Path,
        text: &str,
        now: DateTime,
    ) -> Result<Option<PathBuf>, RecoveryError> {
        if fs::read(file).is_ok_and(|held| held == text.as_bytes()) {
            return Ok(None);
        }
        self.write(Some(file), text, now).map(Some)
    }
}

impl Recovery {
    /// The path the recovery files of `file` are named from: its canonical path, or its folder's
    /// joined with its name while the file does not exist; `None` while its folder cannot be
    /// resolved or the path names no file. A session resolves its file while it is reachable,
    /// since a path with `..` or a symlink in it names another once the folder is gone (D-119).
    #[must_use]
    pub fn resolve(file: &Path) -> Option<PathBuf> {
        resolved(file).ok().flatten()
    }

    /// The newest recovery file of `file` that is no older than `file`, or the newest of any when
    /// `file` does not exist. Equal times count, because the clock that dates files can be too
    /// coarse to tell apart a save and a crash right after it; of two recovery files written at
    /// one time, the later name wins.
    ///
    /// # Errors
    ///
    /// The I/O error that kept the recovery folder or `file` from being read.
    pub fn find(&self, file: &Path) -> io::Result<Option<PathBuf>> {
        let Some(folder) = &self.folder else {
            return Ok(None);
        };
        let entries = match fs::read_dir(folder) {
            Ok(entries) => entries,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error),
        };
        let name = name(file)?;
        // A file that does not exist, or whose time cannot be read, is older than any recovery
        // file; opening it then reports what is wrong with it.
        let changed = fs::metadata(file)
            .and_then(|metadata| metadata.modified())
            .ok();
        let mut found = Vec::new();
        for entry in entries {
            let entry = entry?;
            if is_recovery_of(&name, &entry.file_name()) {
                found.push((entry.metadata()?.modified()?, entry.path()));
            }
        }
        Ok(found
            .into_iter()
            .filter(|&(written, _)| changed.is_none_or(|changed| written >= changed))
            .max()
            .map(|(_, path)| path))
    }

    /// The recovery file to open `file` with, if the user wants one: when [`find`](Self::find)
    /// finds one, asks on `ask` whether to open `file` with its changes and reads the answer, yes
    /// or no, from `answer`
    /// ([file-io.md §4](../../../specs/reference/taskpaper-terminal/specs/file-io.md#4-crash-safety)).
    ///
    /// # Errors
    ///
    /// The I/O error of looking for recovery files or of asking.
    pub fn offer(
        &self,
        file: &Path,
        answer: &mut dyn BufRead,
        ask: &mut dyn Write,
    ) -> io::Result<Option<PathBuf>> {
        let Some(found) = self.find(file)? else {
            return Ok(None);
        };
        write!(
            ask,
            "{} holds changes to {} that were not saved when tipp stopped.\nOpen the file with these changes? [y/N] ",
            found.display(),
            file.display()
        )?;
        ask.flush()?;
        let mut reply = String::new();
        answer.read_line(&mut reply)?;
        let yes = matches!(reply.trim().to_lowercase().as_str(), "y" | "yes");
        Ok(yes.then_some(found))
    }
}

/// What the recovery files of `file` are named after: its backup folder's name, from the path
/// [`Recovery::resolve`] gives, or from its absolute path while its folder cannot be resolved, as
/// when it was removed or unmounted, so that the document can still be kept (D-119). A path
/// `resolve` gave earlier names the same file once its folder is gone.
fn name(file: &Path) -> io::Result<OsString> {
    let source = match resolved(file)? {
        Some(resolved) => resolved,
        None => std::path::absolute(file)?,
    };
    Ok(backup::folder_name(&source))
}

/// [`Recovery::resolve`], or the error that says `file` cannot name a file at all, as when a
/// file stands where its path needs a folder.
fn resolved(file: &Path) -> io::Result<Option<PathBuf>> {
    let error = match fs::canonicalize(file) {
        Ok(canonical) => return Ok(Some(canonical)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => error,
        Err(error) => return Err(error),
    };
    let absolute = std::path::absolute(file)?;
    let (Some(folder), Some(name)) = (absolute.parent(), absolute.file_name()) else {
        return Err(error);
    };
    Ok(fs::canonicalize(folder)
        .ok()
        .map(|folder| folder.join(name)))
}

fn file_name(name: &OsStr, time: DateTime) -> OsString {
    let mut file = name.to_owned();
    file.push(format!("-{}.taskpaper", time.strftime(STAMP)));
    file
}

/// Whether `file` is the name of a recovery file of the file named `name`.
fn is_recovery_of(name: &OsStr, file: &OsStr) -> bool {
    file.as_encoded_bytes()
        .strip_prefix(name.as_encoded_bytes())
        .and_then(|rest| str::from_utf8(rest).ok())
        .and_then(|rest| rest.strip_prefix('-')?.strip_suffix(".taskpaper"))
        .and_then(|stamp| DateTime::strptime(STAMP, stamp).ok())
        .is_some_and(|time| file_name(name, time) == file)
}

#[cfg(test)]
mod tests {
    use std::os::unix::fs::PermissionsExt as _;

    use jiff::civil::date;

    use super::*;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    const CRASH: DateTime = date(2026, 10, 5).at(9, 7, 3, 0);

    /// Recovery files in `dir/recovery`.
    fn recovery(dir: &Path) -> Recovery {
        Recovery::new(Some(dir.join("recovery")))
    }

    fn names(folder: &Path) -> io::Result<Vec<String>> {
        let mut names = fs::read_dir(folder)?
            .map(|entry| Ok(entry?.file_name().to_string_lossy().into_owned()))
            .collect::<io::Result<Vec<_>>>()?;
        names.sort();
        Ok(names)
    }

    // N-REL-01
    #[test]
    fn writes_the_text_to_a_file_named_after_the_file_and_the_time() -> TestResult {
        let dir = tempfile::tempdir()?;
        let file = dir.path().join("todo.taskpaper");
        fs::write(&file, "- on disk\n")?;
        fs::set_permissions(&file, fs::Permissions::from_mode(0o600))?;
        let written = recovery(dir.path()).write(Some(&file), "- unsaved\n", CRASH)?;
        let name = backup::folder_name(&fs::canonicalize(&file)?);
        let expected = format!("{}-20261005-090703.taskpaper", name.to_string_lossy());
        assert_eq!(written, dir.path().join("recovery").join(&expected));
        assert_eq!(fs::read_to_string(&written)?, "- unsaved\n");
        assert_eq!(fs::read_to_string(&file)?, "- on disk\n");
        assert_eq!(
            fs::metadata(&written)?.permissions().mode() & 0o777,
            0o600,
            "a recovery file is as private as its file"
        );
        assert_eq!(names(&dir.path().join("recovery"))?, [expected]);
        Ok(())
    }

    // N-REL-01
    #[test]
    fn a_second_recovery_in_one_second_takes_the_next_one_and_keeps_the_first() -> TestResult {
        let dir = tempfile::tempdir()?;
        let file = dir.path().join("todo.taskpaper");
        fs::write(&file, "- on disk\n")?;
        let recovery = recovery(dir.path());
        let name = name(&file)?.to_string_lossy().into_owned();
        for (text, stamp) in [
            ("- first\n", "090703"),
            ("- second\n", "090704"),
            ("- third\n", "090705"),
        ] {
            let written = recovery.write(Some(&file), text, CRASH)?;
            assert_eq!(
                written.file_name(),
                Some(OsStr::new(&format!("{name}-20261005-{stamp}.taskpaper")))
            );
            assert_eq!(fs::read_to_string(&written)?, text);
        }
        assert_eq!(names(&dir.path().join("recovery"))?.len(), 3);
        Ok(())
    }

    // N-REL-01, D-96
    #[test]
    fn keeping_writes_a_recovery_file_only_when_the_file_lacks_the_text() -> TestResult {
        let dir = tempfile::tempdir()?;
        let file = dir.path().join("todo.taskpaper");
        fs::write(&file, "- saved\n")?;
        let recovery = recovery(dir.path());
        assert_eq!(recovery.keep(&file, "- saved\n", CRASH)?, None);
        assert!(!dir.path().join("recovery").exists());
        let kept = recovery
            .keep(&file, "- unsaved\n", CRASH)?
            .expect("written");
        assert_eq!(fs::read_to_string(&kept)?, "- unsaved\n");
        let missing = dir.path().join("new.taskpaper");
        let kept = recovery.keep(&missing, "- new\n", CRASH)?.expect("written");
        assert_eq!(fs::read_to_string(&kept)?, "- new\n");
        Ok(())
    }

    // N-REL-01
    #[test]
    fn a_document_without_a_file_or_with_a_new_one_is_recovered_too() -> TestResult {
        let dir = tempfile::tempdir()?;
        let recovery = recovery(dir.path());
        let untitled = recovery.write(None, "- a\n", CRASH)?;
        assert_eq!(
            untitled.file_name(),
            Some(OsStr::new("untitled-20261005-090703.taskpaper"))
        );
        let new = dir.path().join("new.taskpaper");
        let written = recovery.write(Some(&new), "- b\n", CRASH)?;
        assert_eq!(fs::read_to_string(&written)?, "- b\n");
        assert_eq!(
            written.file_name(),
            Some(OsStr::new(&format!(
                "{}-20261005-090703.taskpaper",
                backup::folder_name(&fs::canonicalize(dir.path())?.join("new.taskpaper"))
                    .to_string_lossy()
            )))
        );
        assert!(!new.exists(), "the file is left alone");
        Ok(())
    }

    #[test]
    fn writing_fails_without_a_folder_or_when_it_cannot_be_made() -> TestResult {
        let dir = tempfile::tempdir()?;
        let file = dir.path().join("todo.taskpaper");
        let error = Recovery::new(None)
            .write(Some(&file), "x", CRASH)
            .unwrap_err();
        assert!(matches!(error, RecoveryError::NoFolder));
        assert_eq!(
            error.to_string(),
            "no folder for recovery files: neither $XDG_STATE_HOME nor a home folder is set"
        );
        fs::write(dir.path().join("recovery"), "a file where the folder goes")?;
        let error = recovery(dir.path())
            .write(Some(&file), "x", CRASH)
            .unwrap_err();
        assert!(matches!(error, RecoveryError::Io(_)), "{error:?}");
        assert_eq!(error.to_string(), "File exists (os error 17)");
        let dir = tempfile::tempdir()?;
        fs::write(dir.path().join("todo.taskpaper"), "-")?;
        let inside_a_file = dir.path().join("todo.taskpaper/x");
        assert!(matches!(
            recovery(dir.path()).write(Some(&inside_a_file), "x", CRASH),
            Err(RecoveryError::Io(_))
        ));
        assert!(matches!(
            recovery(dir.path()).write(Some(Path::new("")), "x", CRASH),
            Err(RecoveryError::Io(_))
        ));
        // A name the file system takes whose recovery file's name is too long for it: a failure
        // that is no taken name, which permissions cannot cause for root.
        let dir = tempfile::tempdir()?;
        let long = dir.path().join("x".repeat(240));
        let written = recovery(dir.path()).write(Some(&long), "x", CRASH);
        assert!(
            matches!(written, Err(RecoveryError::Io(ref error)) if error.kind() == io::ErrorKind::InvalidFilename),
            "{written:?}"
        );
        Ok(())
    }

    /// Sets the time `file` last changed to `seconds` after the Unix epoch.
    fn touch(file: &Path, seconds: u64) -> io::Result<()> {
        fs::File::options().append(true).open(file)?.set_modified(
            std::time::SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(seconds),
        )
    }

    /// Asks with `recovery` whether to open `file` with a recovery file, answering `reply`;
    /// returns the recovery file accepted and the question.
    fn offer(
        recovery: &Recovery,
        file: &Path,
        reply: &str,
    ) -> io::Result<(Option<PathBuf>, String)> {
        let mut asked = Vec::new();
        let accepted = recovery.offer(file, &mut reply.as_bytes(), &mut asked)?;
        Ok((accepted, String::from_utf8_lossy(&asked).into_owned()))
    }

    // N-REL-01
    #[test]
    fn finds_the_newest_recovery_file_newer_than_the_file() -> TestResult {
        let dir = tempfile::tempdir()?;
        let file = dir.path().join("todo.taskpaper");
        fs::write(&file, "- a\n")?;
        touch(&file, 2000)?;
        let folder = dir.path().join("recovery");
        fs::create_dir(&folder)?;
        let name = name(&file)?.to_string_lossy().into_owned();
        let copy = |stamp: &str, seconds| -> io::Result<PathBuf> {
            let path = folder.join(format!("{name}-{stamp}.taskpaper"));
            fs::write(&path, stamp)?;
            touch(&path, seconds)?;
            Ok(path)
        };
        let recovery = recovery(dir.path());
        copy("20261001-120000", 1000)?;
        assert_eq!(recovery.find(&file)?, None, "older than the file");
        let as_old = copy("20261002-120000", 2000)?;
        assert_eq!(recovery.find(&file)?, Some(as_old), "as old as the file");
        let older = copy("20261003-120000", 3000)?;
        let newest = copy("20261004-120000", 4000)?;
        copy("20261005-120000", 2500)?;
        assert_eq!(recovery.find(&file)?, Some(newest.clone()));
        fs::remove_file(&newest)?;
        assert_eq!(recovery.find(&file)?, Some(older));
        let later_name = copy("20261003-120001", 3000)?;
        assert_eq!(
            recovery.find(&file)?,
            Some(later_name),
            "a tie goes to the later name"
        );
        fs::remove_file(&file)?;
        assert!(recovery.find(&file)?.is_some(), "any is newer than no file");
        Ok(())
    }

    #[test]
    fn ignores_the_recovery_files_of_other_files_and_other_names() -> TestResult {
        let dir = tempfile::tempdir()?;
        let recovery = recovery(dir.path());
        for sub in ["a", "b"] {
            fs::create_dir(dir.path().join(sub))?;
        }
        let file = dir.path().join("a/todo.taskpaper");
        let other = dir.path().join("b/todo.taskpaper");
        fs::write(&file, "- a\n")?;
        touch(&file, 1000)?;
        recovery.write(Some(&other), "- b\n", CRASH)?;
        let folder = dir.path().join("recovery");
        let name = name(&file)?.to_string_lossy().into_owned();
        for odd in [
            format!("{name}-20261005-0907.taskpaper"),
            format!("{name}-20261005-090703.txt"),
            format!("{name}20261005-090703.taskpaper"),
            format!("{name}-2026105-090703.taskpaper"),
            format!("{name}-x-20261005-090703.taskpaper"),
            name.clone(),
        ] {
            fs::write(folder.join(odd), "-")?;
        }
        assert_eq!(recovery.find(&file)?, None);
        Ok(())
    }

    #[test]
    fn finds_nothing_without_a_folder() -> TestResult {
        let dir = tempfile::tempdir()?;
        let file = dir.path().join("todo.taskpaper");
        assert_eq!(Recovery::new(None).find(&file)?, None);
        assert_eq!(recovery(dir.path()).find(&file)?, None);
        Ok(())
    }

    #[test]
    fn looking_fails_when_the_folder_or_the_file_cannot_be_read() -> TestResult {
        let dir = tempfile::tempdir()?;
        let file = dir.path().join("todo.taskpaper");
        fs::write(dir.path().join("recovery"), "a file where the folder goes")?;
        assert!(recovery(dir.path()).find(&file).is_err());
        let dir = tempfile::tempdir()?;
        recovery(dir.path()).write(None, "-", CRASH)?;
        fs::write(dir.path().join("todo.taskpaper"), "-")?;
        let inside_a_file = dir.path().join("todo.taskpaper/x");
        assert!(recovery(dir.path()).find(&inside_a_file).is_err());
        let up_from_nothing = dir.path().join("missing/..");
        assert!(recovery(dir.path()).find(&up_from_nothing).is_err());
        Ok(())
    }

    // N-REL-01, D-119
    #[test]
    fn a_file_whose_folder_is_gone_is_recovered_and_found_by_its_absolute_path() -> TestResult {
        let dir = tempfile::tempdir()?;
        let file = dir.path().join("gone/deeper/todo.taskpaper");
        let recovery = recovery(dir.path());
        assert_eq!(recovery.find(&file)?, None);
        let written = recovery.write(Some(&file), "- unsaved\n", CRASH)?;
        assert_eq!(
            written.file_name(),
            Some(OsStr::new(&format!(
                "{}-20261005-090703.taskpaper",
                backup::folder_name(&std::path::absolute(&file)?).to_string_lossy()
            )))
        );
        assert_eq!(fs::read_to_string(&written)?, "- unsaved\n");
        assert_eq!(recovery.find(&file)?, Some(written));
        Ok(())
    }

    // N-REL-01, D-119
    #[test]
    fn a_path_resolved_while_its_folder_was_there_names_the_file_found_once_it_is_back()
    -> TestResult {
        let dir = tempfile::tempdir()?;
        let real = dir.path().join("real");
        fs::create_dir_all(dir.path().join("work"))?;
        std::os::unix::fs::symlink(&real, dir.path().join("link"))?;
        let recovery = recovery(dir.path());
        for file in [
            dir.path().join("link/todo.taskpaper"),
            dir.path().join("work/../real/todo.taskpaper"),
        ] {
            fs::create_dir_all(&real)?;
            fs::write(real.join("todo.taskpaper"), "- a\n")?;
            let resolved = Recovery::resolve(&file).expect("resolved");
            assert_eq!(resolved, fs::canonicalize(&file)?);
            fs::remove_dir_all(&real)?;
            assert_eq!(Recovery::resolve(&file), None, "the folder is gone");
            let written = recovery.write(Some(&resolved), "- unsaved\n", CRASH)?;
            fs::create_dir(&real)?;
            fs::write(real.join("todo.taskpaper"), "- a\n")?;
            touch(&real.join("todo.taskpaper"), 1000)?;
            assert_eq!(recovery.find(&file)?, Some(written.clone()));
            fs::remove_dir_all(&real)?;
            fs::remove_file(written)?;
        }
        let new = dir.path().join("work/../work/new.taskpaper");
        assert_eq!(
            Recovery::resolve(&new),
            Some(fs::canonicalize(dir.path().join("work"))?.join("new.taskpaper")),
            "a file not written yet"
        );
        Ok(())
    }

    // N-REL-01
    #[test]
    fn offers_a_newer_recovery_file_and_takes_it_on_yes() -> TestResult {
        let dir = tempfile::tempdir()?;
        let file = dir.path().join("todo.taskpaper");
        fs::write(&file, "- a\n")?;
        touch(&file, 1000)?;
        let recovery = recovery(dir.path());
        let found = recovery.write(Some(&file), "- b\n", CRASH)?;
        let (accepted, asked) = offer(&recovery, &file, "y\n")?;
        assert_eq!(accepted.as_ref(), Some(&found));
        assert_eq!(
            asked,
            format!(
                "{} holds changes to {} that were not saved when tipp stopped.\nOpen the file with these changes? [y/N] ",
                found.display(),
                file.display()
            )
        );
        for yes in ["yes\n", " Y "] {
            assert_eq!(
                offer(&recovery, &file, yes)?.0.as_ref(),
                Some(&found),
                "{yes:?}"
            );
        }
        for no in ["n\n", "\n", "", "yep\n"] {
            assert_eq!(offer(&recovery, &file, no)?.0, None, "{no:?}");
        }
        Ok(())
    }

    #[test]
    fn asks_nothing_without_a_newer_recovery_file() -> TestResult {
        let dir = tempfile::tempdir()?;
        let file = dir.path().join("todo.taskpaper");
        fs::write(&file, "- a\n")?;
        assert_eq!(
            offer(&recovery(dir.path()), &file, "y\n")?,
            (None, String::new())
        );
        Ok(())
    }

    #[test]
    fn a_failed_question_is_an_error() -> TestResult {
        let dir = tempfile::tempdir()?;
        let file = dir.path().join("todo.taskpaper");
        let recovery = recovery(dir.path());
        recovery.write(Some(&file), "- b\n", CRASH)?;
        let mut full: &mut [u8] = &mut [];
        let error = recovery
            .offer(&file, &mut "y\n".as_bytes(), &mut full)
            .unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::WriteZero);
        let mut unreadable: &[u8] = b"\xff\n";
        assert!(
            recovery
                .offer(&file, &mut unreadable, &mut Vec::new())
                .is_err()
        );
        Ok(())
    }
}
