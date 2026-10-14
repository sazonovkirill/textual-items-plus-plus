//! Daily backups: before the app first changes a file on a given day, a copy of the file as it is
//! on disk goes to a folder of its own under the backups folder, which keeps the newest copies
//! (F-FILE-09); conflict copies keep the version a conflict's choice discards beside them
//! ([file-io.md §3](../../../specs/reference/taskpaper-terminal/specs/file-io.md#3-external-changes)).

use std::{
    ffi::OsString,
    fs::{self, Permissions},
    io,
    path::{Path, PathBuf},
};

use jiff::{
    SignedDuration,
    civil::{Date, DateTime},
};
use thiserror::Error;

use super::{atomic, version::fnv1a};

/// How many daily copies each file keeps.
pub const KEEP: usize = 30;

/// The file in a file's backup folder that names the file.
const SOURCE: &str = "source-path.txt";

/// The local time in a conflict copy's name, and what follows it.
const CONFLICT_TIME: &str = "%Y-%m-%d-%H%M%S";
const CONFLICT_SUFFIX: &str = "-conflict.taskpaper";

/// Why a backup could not be made.
#[derive(Debug, Error)]
pub enum BackupError {
    /// Neither `$XDG_DATA_HOME` nor a home folder says where backups go.
    #[error("no folder for backups: neither $XDG_DATA_HOME nor a home folder is set")]
    NoFolder,
    /// Reading the file or writing the copy failed.
    #[error(transparent)]
    Io(#[from] io::Error),
}

/// The backups of the files one session opens: where they go, and which file was checked on
/// which day, so each file is checked once a day.
#[derive(Debug)]
pub struct Backups {
    folder: Option<PathBuf>,
    checked: Option<(PathBuf, Date)>,
}

impl Backups {
    /// Backups in `folder`, or nowhere when there is none, which every backup reports.
    #[must_use]
    pub fn new(folder: Option<PathBuf>) -> Self {
        Self {
            folder,
            checked: None,
        }
    }

    /// Makes sure the copy of `file` dated `today`, the local date, exists, unless `file` was
    /// checked on that day already. The copy is the file as it is on disk; none is made of a
    /// missing or empty file or of one equal to its newest copy, so a file left alone for weeks
    /// does not use up its copies. A failure is not retried until the next day.
    ///
    /// # Errors
    ///
    /// [`BackupError`] when the copy could not be made.
    pub fn ensure(&mut self, file: &Path, today: Date) -> Result<(), BackupError> {
        let check = (file.to_owned(), today);
        if self.checked.as_ref() == Some(&check) {
            return Ok(());
        }
        self.checked = Some(check);
        let folder = self.folder.as_deref().ok_or(BackupError::NoFolder)?;
        Ok(daily(file, folder, today)?)
    }

    /// Writes `contents`, a version of `file` a conflict's choice discards, to a conflict copy
    /// `<YYYY-MM-DD>-<HHMMSS>-conflict.taskpaper` in the file's folder, named by the local time
    /// `now`, or the first later second no copy is named by, and returns its path. The newest
    /// [`KEEP`] conflict copies are kept, counted apart from the daily copies. A file gone from
    /// disk still has its folder.
    ///
    /// # Errors
    ///
    /// [`BackupError`] when there is no backups folder or the copy could not be written.
    pub fn conflict(
        &self,
        file: &Path,
        contents: &[u8],
        now: DateTime,
    ) -> Result<PathBuf, BackupError> {
        let backups = self.folder.as_deref().ok_or(BackupError::NoFolder)?;
        let (source, permissions) = locate(file)?;
        let folder = backups.join(folder_name(&source));
        fs::create_dir_all(&folder)?;
        let mut time = now;
        while fs::exists(folder.join(conflict_name(time)))? {
            time = time.saturating_add(SignedDuration::from_secs(1));
        }
        let copy = folder.join(conflict_name(time));
        write_copy(&source, &copy, contents, permissions)?;
        let mut times = conflict_copies(&folder)?;
        times.sort_unstable_by(|a, b| b.cmp(a));
        for old in times.into_iter().skip(KEEP) {
            fs::remove_file(folder.join(conflict_name(old)))?;
        }
        Ok(copy)
    }
}

/// The canonical path of `file` and its permissions, or, when it is gone, the canonical path of
/// its folder joined with its name, and no permissions.
fn locate(file: &Path) -> io::Result<(PathBuf, Option<Permissions>)> {
    let source = match fs::canonicalize(file) {
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            let file = std::path::absolute(file)?;
            let folder = fs::canonicalize(file.parent().unwrap_or(&file))?;
            return Ok((folder.join(file.file_name().unwrap_or_default()), None));
        }
        source => source?,
    };
    let permissions = fs::metadata(&source)?.permissions();
    Ok((source, Some(permissions)))
}

/// Copies `file` as it is on disk to `<YYYY-MM-DD>.taskpaper`, dated `today`, in its folder under
/// `backups`, unless that copy exists, the file is missing or empty, or it equals the newest copy;
/// then drops the oldest copies beyond the newest [`KEEP`].
fn daily(file: &Path, backups: &Path, today: Date) -> io::Result<()> {
    let bytes = match fs::read(file) {
        Ok(bytes) if !bytes.is_empty() => bytes,
        Ok(_) => return Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error),
    };
    let (source, permissions) = locate(file)?;
    let folder = backups.join(folder_name(&source));
    fs::create_dir_all(&folder)?;
    let copy = folder.join(daily_name(today));
    if fs::exists(&copy)? {
        return Ok(());
    }
    let mut dates = daily_copies(&folder)?;
    if let Some(&newest) = dates.first()
        && fs::read(folder.join(daily_name(newest)))? == bytes
    {
        return Ok(());
    }
    write_copy(&source, &copy, &bytes, permissions)?;
    dates.push(today);
    dates.sort_unstable_by(|a, b| b.cmp(a));
    for old in dates.into_iter().skip(KEEP) {
        fs::remove_file(folder.join(daily_name(old)))?;
    }
    Ok(())
}

/// Writes `contents` to `copy` with `permissions`, so a copy is as private as its file, or those
/// of a new file without them, and records the path of `source` in the folder's [`SOURCE`] file, so a person can tell which file
/// the folder belongs to.
fn write_copy(
    source: &Path,
    copy: &Path,
    contents: &[u8],
    permissions: Option<Permissions>,
) -> io::Result<()> {
    let folder = copy.parent().unwrap_or(copy);
    let path = source.as_os_str().as_encoded_bytes();
    atomic::replace(&folder.join(SOURCE), path, permissions.clone())?;
    atomic::replace(copy, contents, permissions)
}

/// `<file name>-<hash>`, where the hash of the canonical path tells apart files with one name.
pub(super) fn folder_name(source: &Path) -> OsString {
    let mut name = source.file_name().unwrap_or_default().to_os_string();
    name.push(format!(
        "-{:016x}",
        fnv1a(source.as_os_str().as_encoded_bytes())
    ));
    name
}

fn daily_name(date: Date) -> String {
    format!("{date}.taskpaper")
}

fn conflict_name(time: DateTime) -> String {
    format!("{}{CONFLICT_SUFFIX}", time.strftime(CONFLICT_TIME))
}

/// The times of the conflict copies in `folder`; other files are not conflict copies.
fn conflict_copies(folder: &Path) -> io::Result<Vec<DateTime>> {
    let mut times = Vec::new();
    for entry in fs::read_dir(folder)? {
        let name = entry?.file_name();
        let time = name
            .to_str()
            .and_then(|name| name.strip_suffix(CONFLICT_SUFFIX))
            .and_then(|time| DateTime::strptime(CONFLICT_TIME, time).ok())
            .filter(|&time| name == conflict_name(time).as_str());
        times.extend(time);
    }
    Ok(times)
}

/// The dates of the daily copies in `folder`, newest first; other files are not copies.
fn daily_copies(folder: &Path) -> io::Result<Vec<Date>> {
    let mut dates = Vec::new();
    for entry in fs::read_dir(folder)? {
        let name = entry?.file_name();
        let date = name
            .to_str()
            .and_then(|name| name.strip_suffix(".taskpaper"))
            .and_then(|date| date.parse::<Date>().ok())
            .filter(|&date| name == daily_name(date).as_str());
        dates.extend(date);
    }
    dates.sort_unstable_by(|a, b| b.cmp(a));
    Ok(dates)
}

#[cfg(test)]
mod tests {
    use std::os::unix::fs::PermissionsExt as _;

    use jiff::civil::date;

    use super::*;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    const TODAY: Date = date(2026, 10, 5);

    /// A file `todo.taskpaper` holding `text`, and backups in a folder that does not exist yet,
    /// both in `dir`.
    fn setup(dir: &Path, text: &str) -> io::Result<(PathBuf, Backups, PathBuf)> {
        let file = dir.join("todo.taskpaper");
        fs::write(&file, text)?;
        let folder = dir.join("backups");
        Ok((file, Backups::new(Some(folder.clone())), folder))
    }

    /// The backup folder of `file` under `backups`.
    fn backup_folder(file: &Path, backups: &Path) -> io::Result<PathBuf> {
        Ok(backups.join(folder_name(&fs::canonicalize(file)?)))
    }

    /// The names in `folder`, sorted.
    fn names(folder: &Path) -> io::Result<Vec<String>> {
        let mut names = fs::read_dir(folder)?
            .map(|entry| Ok(entry?.file_name().to_string_lossy().into_owned()))
            .collect::<io::Result<Vec<_>>>()?;
        names.sort();
        Ok(names)
    }

    // F-FILE-09
    #[test]
    fn copies_the_file_into_a_folder_named_after_it() -> TestResult {
        let dir = tempfile::tempdir()?;
        let (file, mut backups, folder) = setup(dir.path(), "- a\n")?;
        fs::set_permissions(&file, fs::Permissions::from_mode(0o600))?;
        backups.ensure(&file, TODAY)?;
        let source = fs::canonicalize(&file)?;
        let own = folder.join(format!(
            "todo.taskpaper-{:016x}",
            fnv1a(source.as_os_str().as_encoded_bytes())
        ));
        assert_eq!(names(&own)?, ["2026-10-05.taskpaper", SOURCE]);
        let copy = own.join("2026-10-05.taskpaper");
        assert_eq!(fs::read_to_string(&copy)?, "- a\n");
        assert_eq!(
            fs::metadata(&copy)?.permissions().mode() & 0o777,
            0o600,
            "a copy is as private as its file"
        );
        assert_eq!(
            fs::read_to_string(own.join(SOURCE))?,
            source.to_string_lossy()
        );
        assert_eq!(
            fs::read_to_string(&file)?,
            "- a\n",
            "the file is left alone"
        );
        Ok(())
    }

    // F-FILE-09
    #[test]
    fn files_with_one_name_in_different_folders_get_different_folders() -> TestResult {
        let dir = tempfile::tempdir()?;
        let folder = dir.path().join("backups");
        let mut backups = Backups::new(Some(folder.clone()));
        for sub in ["a", "b"] {
            fs::create_dir(dir.path().join(sub))?;
            let file = dir.path().join(sub).join("todo.taskpaper");
            fs::write(&file, sub)?;
            backups.ensure(&file, TODAY)?;
        }
        assert_eq!(names(&folder)?.len(), 2);
        Ok(())
    }

    // F-FILE-09
    #[test]
    fn a_link_is_backed_up_as_the_file_it_points_to() -> TestResult {
        let dir = tempfile::tempdir()?;
        let (file, mut backups, folder) = setup(dir.path(), "- a\n")?;
        let link = dir.path().join("link.taskpaper");
        std::os::unix::fs::symlink(&file, &link)?;
        backups.ensure(&link, TODAY)?;
        let copy = backup_folder(&file, &folder)?.join("2026-10-05.taskpaper");
        assert_eq!(fs::read_to_string(copy)?, "- a\n");
        Ok(())
    }

    // F-FILE-09
    #[test]
    fn keeps_the_first_copy_of_the_day_and_copies_again_the_next_day() -> TestResult {
        let dir = tempfile::tempdir()?;
        let (file, _, folder) = setup(dir.path(), "first")?;
        Backups::new(Some(folder.clone())).ensure(&file, TODAY)?;
        fs::write(&file, "second")?;
        Backups::new(Some(folder.clone())).ensure(&file, TODAY)?;
        let own = backup_folder(&file, &folder)?;
        assert_eq!(
            fs::read_to_string(own.join("2026-10-05.taskpaper"))?,
            "first",
            "even in another session"
        );
        Backups::new(Some(folder)).ensure(&file, date(2026, 10, 6))?;
        assert_eq!(
            fs::read_to_string(own.join("2026-10-06.taskpaper"))?,
            "second"
        );
        Ok(())
    }

    // F-FILE-09
    #[test]
    fn checks_each_file_once_a_day() -> TestResult {
        let dir = tempfile::tempdir()?;
        let (file, mut backups, folder) = setup(dir.path(), "")?;
        backups.ensure(&file, TODAY)?;
        fs::write(&file, "changed since")?;
        backups.ensure(&file, TODAY)?;
        assert!(
            !fs::exists(&folder)?,
            "the file was empty when first checked today"
        );
        backups.ensure(&file, date(2026, 10, 6))?;
        assert_eq!(names(&backup_folder(&file, &folder)?)?.len(), 2);
        let other = dir.path().join("other.taskpaper");
        fs::write(&other, "other")?;
        backups.ensure(&other, TODAY)?;
        assert_eq!(names(&folder)?.len(), 2, "another file is checked too");
        Ok(())
    }

    #[test]
    fn skips_a_file_equal_to_its_newest_copy() -> TestResult {
        let dir = tempfile::tempdir()?;
        let (file, mut backups, folder) = setup(dir.path(), "same")?;
        backups.ensure(&file, date(2026, 9, 1))?;
        backups.ensure(&file, TODAY)?;
        let own = backup_folder(&file, &folder)?;
        assert_eq!(names(&own)?, ["2026-09-01.taskpaper", SOURCE]);
        Ok(())
    }

    #[test]
    fn skips_a_missing_or_empty_file() -> TestResult {
        let dir = tempfile::tempdir()?;
        let (file, mut backups, folder) = setup(dir.path(), "")?;
        backups.ensure(&file, TODAY)?;
        backups.ensure(&dir.path().join("new.taskpaper"), TODAY)?;
        assert!(!fs::exists(&folder)?);
        Ok(())
    }

    // F-FILE-09
    #[test]
    fn keeps_the_newest_copies_and_leaves_other_files_alone() -> TestResult {
        let dir = tempfile::tempdir()?;
        let (file, mut backups, folder) = setup(dir.path(), "today")?;
        let own = backup_folder(&file, &folder)?;
        fs::create_dir_all(&own)?;
        let first = date(2026, 1, 1);
        for day in 0..KEEP {
            let old = first.checked_add(jiff::Span::new().days(i64::try_from(day)?))?;
            fs::write(own.join(daily_name(old)), format!("{day}"))?;
        }
        let others = [
            "2026-01-01-093000-conflict.taskpaper",
            "notes.txt",
            "1.taskpaper",
            "2026-1-1.taskpaper",
        ];
        for other in others {
            fs::write(own.join(other), "")?;
        }
        backups.ensure(&file, TODAY)?;
        let names = names(&own)?;
        assert!(!names.contains(&"2026-01-01.taskpaper".to_owned()));
        assert!(names.contains(&"2026-01-02.taskpaper".to_owned()));
        assert!(names.contains(&"2026-10-05.taskpaper".to_owned()));
        assert_eq!(names.len(), KEEP + others.len() + 1, "{names:?}");
        Ok(())
    }

    const NOW: DateTime = TODAY.at(9, 30, 0, 0);

    // F-FILE-06
    #[test]
    fn a_conflict_copy_goes_beside_the_daily_copies_and_never_replaces_another() -> TestResult {
        let dir = tempfile::tempdir()?;
        let (file, mut backups, folder) = setup(dir.path(), "on disk")?;
        fs::set_permissions(&file, fs::Permissions::from_mode(0o600))?;
        backups.ensure(&file, TODAY)?;
        let first = backups.conflict(&file, b"theirs", NOW)?;
        let second = backups.conflict(&file, b"mine", NOW)?;
        let own = backup_folder(&file, &folder)?;
        assert_eq!(first, own.join("2026-10-05-093000-conflict.taskpaper"));
        assert_eq!(second, own.join("2026-10-05-093001-conflict.taskpaper"));
        assert_eq!(fs::read_to_string(&first)?, "theirs");
        assert_eq!(fs::read_to_string(&second)?, "mine");
        assert_eq!(fs::metadata(&first)?.permissions().mode() & 0o777, 0o600);
        assert_eq!(
            fs::read_to_string(own.join("2026-10-05.taskpaper"))?,
            "on disk"
        );
        assert_eq!(
            fs::read_to_string(&file)?,
            "on disk",
            "the file is left alone"
        );
        Ok(())
    }

    // F-FILE-06
    #[test]
    fn a_file_gone_from_disk_still_gets_its_conflict_copies() -> TestResult {
        let dir = tempfile::tempdir()?;
        let (file, backups, folder) = setup(dir.path(), "x")?;
        let own = backup_folder(&file, &folder)?;
        fs::remove_file(&file)?;
        let copy = backups.conflict(&file, b"mine", NOW)?;
        assert_eq!(copy, own.join("2026-10-05-093000-conflict.taskpaper"));
        assert_eq!(
            fs::read_to_string(own.join(SOURCE))?,
            fs::canonicalize(dir.path())?
                .join("todo.taskpaper")
                .to_string_lossy()
        );
        Ok(())
    }

    #[test]
    fn keeps_the_newest_conflict_copies_apart_from_the_daily_ones() -> TestResult {
        let dir = tempfile::tempdir()?;
        let (file, backups, folder) = setup(dir.path(), "x")?;
        let own = backup_folder(&file, &folder)?;
        fs::create_dir_all(&own)?;
        let first = date(2026, 1, 1).at(8, 0, 0, 0);
        for minute in 0..KEEP {
            let old = first.checked_add(jiff::Span::new().minutes(i64::try_from(minute)?))?;
            fs::write(own.join(conflict_name(old)), "")?;
        }
        let others = ["2026-01-01.taskpaper", "2026-1-1-080000-conflict.taskpaper"];
        for other in others {
            fs::write(own.join(other), "")?;
        }
        backups.conflict(&file, b"new", NOW)?;
        let names = names(&own)?;
        assert!(!names.contains(&"2026-01-01-080000-conflict.taskpaper".to_owned()));
        assert!(names.contains(&"2026-01-01-080100-conflict.taskpaper".to_owned()));
        assert!(names.contains(&"2026-10-05-093000-conflict.taskpaper".to_owned()));
        assert_eq!(names.len(), KEEP + others.len() + 1, "{names:?}");
        Ok(())
    }

    #[test]
    fn a_conflict_copy_fails_without_a_folder_or_when_it_cannot_be_written() -> TestResult {
        let dir = tempfile::tempdir()?;
        let (file, _, folder) = setup(dir.path(), "x")?;
        let error = Backups::new(None).conflict(&file, b"x", NOW).unwrap_err();
        assert!(matches!(error, BackupError::NoFolder));
        fs::write(&folder, "a file where the folder goes")?;
        let error = Backups::new(Some(folder)).conflict(&file, b"x", NOW);
        assert!(matches!(error, Err(BackupError::Io(_))), "{error:?}");
        Ok(())
    }

    #[test]
    fn fails_when_the_file_cannot_be_read_or_copied() -> TestResult {
        let dir = tempfile::tempdir()?;
        let (file, _, folder) = setup(dir.path(), "x")?;
        let unreadable = Backups::new(Some(folder.clone())).ensure(dir.path(), TODAY);
        assert!(matches!(unreadable, Err(BackupError::Io(_))));
        fs::write(&folder, "a file where the folder goes")?;
        let error = Backups::new(Some(folder)).ensure(&file, TODAY).unwrap_err();
        assert!(matches!(&error, BackupError::Io(_)));
        assert_eq!(error.to_string(), "Not a directory (os error 20)");
        Ok(())
    }

    #[test]
    fn fails_once_a_day_without_a_folder() -> TestResult {
        let dir = tempfile::tempdir()?;
        let (file, _, _) = setup(dir.path(), "x")?;
        let mut backups = Backups::new(None);
        let error = backups.ensure(&file, TODAY).unwrap_err();
        assert!(matches!(error, BackupError::NoFolder));
        assert_eq!(
            error.to_string(),
            "no folder for backups: neither $XDG_DATA_HOME nor a home folder is set"
        );
        backups.ensure(&file, TODAY)?;
        Ok(())
    }
}
