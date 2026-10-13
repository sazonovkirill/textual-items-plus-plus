//! Recovery files: when the app crashes, the document with the changes its file lacks goes to a
//! file of its own in the recovery folder, so that a panic never loses data (N-REL-01).

use std::{
    ffi::{OsStr, OsString},
    fs, io,
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
}

/// What the recovery files of `file` are named after: its backup folder's name, from its
/// canonical path, or from its folder's canonical path while the file does not exist yet.
fn name(file: &Path) -> io::Result<OsString> {
    let source = fs::canonicalize(file).or_else(|error| {
        let absolute = std::path::absolute(file)?;
        match (absolute.parent(), absolute.file_name()) {
            (Some(folder), Some(name)) if error.kind() == io::ErrorKind::NotFound => {
                Ok(fs::canonicalize(folder)?.join(name))
            }
            _ => Err(error),
        }
    })?;
    Ok(backup::folder_name(&source))
}

fn file_name(name: &OsStr, time: DateTime) -> OsString {
    let mut file = name.to_owned();
    file.push(format!("-{}.taskpaper", time.strftime(STAMP)));
    file
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
}
