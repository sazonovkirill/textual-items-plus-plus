//! Opening a `.taskpaper` file: reading it from disk into the domain model (FR-STORE-01,
//! F-FILE-07).

use std::{io, path::Path};

use thiserror::Error;

use jiff::civil::DateTime;

use super::{
    BackupError, Backups, Version, disk,
    taskpaper::{DecodeError, Source, parse_bytes, read_document, serialize, write_document},
};
use crate::domain::Document;

/// Why a file could not be opened.
#[derive(Debug, Error)]
pub enum OpenError {
    /// Reading the file failed.
    #[error("cannot read {path}: {source}", path = .path.display())]
    Io {
        /// The file.
        path: Box<Path>,
        /// Why.
        source: io::Error,
    },
    /// The file is not text the app can read without risking it.
    #[error("cannot open {path}: {source}", path = .path.display())]
    Decode {
        /// The file.
        path: Box<Path>,
        /// Why.
        source: DecodeError,
    },
    /// The file on disk could not be kept in a conflict copy before recovered changes replace it.
    #[error("cannot keep a copy of {path} before opening it with the recovered changes: {source}", path = .path.display())]
    Copy {
        /// The file.
        path: Box<Path>,
        /// Why.
        source: BackupError,
    },
    /// The file does not exist and neither does its folder, so it could never be saved.
    #[error("cannot open {path}: the folder {folder} does not exist", path = .path.display(), folder = .folder.display())]
    MissingFolder {
        /// The file.
        path: Box<Path>,
        /// Its folder.
        folder: Box<Path>,
    },
}

/// A file as opened: the document it holds, the outline it was read from, which saving it back
/// needs, and the version read, `None` when there is no file yet.
#[derive(Debug)]
pub struct OpenFile {
    /// The document.
    pub document: Document,
    /// The outline it was read from.
    pub source: Source,
    /// The version of the file read.
    pub version: Option<Version>,
}

/// The file at `path` as [`OpenFile`] reads it; a file that does not exist in a folder that does
/// holds an empty document, and is created by the first save (F-FILE-01).
///
/// # Errors
///
/// [`OpenError::Io`] when the file cannot be read, [`OpenError::Decode`] when it is not UTF-8, and
/// [`OpenError::MissingFolder`] when neither it nor its folder exists.
pub fn open(path: &Path) -> Result<OpenFile, OpenError> {
    let bytes = read(path)?;
    let (document, source) = decode_file(path, bytes.as_deref().unwrap_or_default())?;
    Ok(OpenFile {
        document,
        source,
        version: bytes.as_deref().map(Version::of),
    })
}

/// The bytes of the file at `path`, `None` when it does not exist in a folder that does.
fn read(path: &Path) -> Result<Option<Vec<u8>>, OpenError> {
    let bytes = disk::read(path).map_err(|source| OpenError::Io {
        path: path.into(),
        source,
    })?;
    if bytes.is_none()
        && let Some(folder) = std::path::absolute(path)
            .ok()
            .and_then(|path| path.parent().map(Path::to_owned))
            .filter(|folder| !folder.is_dir())
    {
        return Err(OpenError::MissingFolder {
            path: path.into(),
            folder: folder.into(),
        });
    }
    Ok(bytes)
}

/// The document `bytes`, read from the file at `path`, hold, with the outline they were read as.
fn decode_file(path: &Path, bytes: &[u8]) -> Result<(Document, Source), OpenError> {
    decode(bytes).map_err(|source| OpenError::Decode {
        path: path.into(),
        source,
    })
}

/// The file at `path` opened with the document of `recovered`, one of its recovery files, as
/// changes the file lacks, and the version of the file on disk, which `backups` first keeps in a
/// conflict copy named by the local time `now`, since it may have changed after the recovered
/// document was read from it
/// ([file-io.md §4](../../../specs/reference/taskpaper-terminal/specs/file-io.md#4-crash-safety)).
///
/// # Errors
///
/// [`OpenError`] when either file cannot be opened or the copy fails.
pub fn recover(
    path: &Path,
    recovered: &Path,
    backups: &Backups,
    now: DateTime,
) -> Result<OpenFile, OpenError> {
    let on_disk = read(path)?;
    // Decoding the file says what is wrong with it, as opening it alone would.
    decode_file(path, on_disk.as_deref().unwrap_or_default())?;
    let bytes = std::fs::read(recovered).map_err(|source| OpenError::Io {
        path: recovered.into(),
        source,
    })?;
    let (document, source) = decode_file(recovered, &bytes)?;
    if let Some(bytes) = &on_disk {
        backups
            .conflict(path, bytes, now)
            .map_err(|source| OpenError::Copy {
                path: path.into(),
                source,
            })?;
    }
    Ok(OpenFile {
        document,
        source,
        version: on_disk.as_deref().map(Version::of),
    })
}

/// The document `bytes` of a file hold, with the outline they were read as.
///
/// # Errors
///
/// [`DecodeError`] when they are not UTF-8.
pub fn decode(bytes: &[u8]) -> Result<(Document, Source), DecodeError> {
    Ok(read_document(parse_bytes(bytes)?))
}

/// The text of a file holding `document`, read as `source`.
#[must_use]
pub fn encode(document: &Document, source: &Source) -> String {
    serialize(&write_document(document, source))
}

#[cfg(test)]
mod tests {
    use std::{fs, io};

    use super::*;
    use crate::domain::{ItemKind, Node};
    use crate::store::taskpaper::{parse, serialize, write_document};

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    // FR-STORE-01
    #[test]
    fn opens_the_document_a_file_holds_and_what_saving_it_back_needs() -> TestResult {
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("todo.taskpaper");
        let text = "Inbox:\n\t- Call the bank @due(2026-10-07)\nnote\n";
        fs::write(&path, text)?;
        let OpenFile {
            document,
            source,
            version,
        } = open(&path)?;
        assert_eq!(version, Some(Version::of(text.as_bytes())));
        let task = document.children(document.inbox()).first().copied();
        let title = task
            .and_then(|task| document.node(task))
            .and_then(Node::item)
            .map(|item| (item.kind, item.title.as_str()));
        assert_eq!(title, Some((ItemKind::Task, "Call the bank")));
        assert_eq!(serialize(&write_document(&document, &source)), text);
        Ok(())
    }

    // FR-STORE-01
    #[test]
    fn a_missing_file_holds_an_empty_document() -> TestResult {
        let dir = tempfile::tempdir()?;
        let opened = open(&dir.path().join("new.taskpaper"))?;
        assert_eq!(opened.document, read_document(parse("")).0);
        assert_eq!(opened.version, None);
        assert_eq!(
            fs::read_dir(dir.path())?.count(),
            0,
            "nothing is created yet"
        );
        Ok(())
    }

    // F-FILE-01
    #[test]
    fn a_missing_file_in_a_missing_folder_is_refused() -> TestResult {
        let dir = tempfile::tempdir()?;
        let folder = dir.path().join("nowhere");
        let path = folder.join("new.taskpaper");
        let error = open(&path).expect_err("no folder");
        assert!(
            matches!(&error, OpenError::MissingFolder { .. }),
            "{error:?}"
        );
        assert_eq!(
            error.to_string(),
            format!(
                "cannot open {}: the folder {} does not exist",
                path.display(),
                folder.display()
            )
        );
        Ok(())
    }

    // N-REL-01
    #[test]
    fn recovering_opens_the_recovered_document_over_a_copy_of_the_file() -> TestResult {
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("todo.taskpaper");
        let recovered = dir.path().join("recovered.taskpaper");
        fs::write(&path, "- on disk\n")?;
        fs::write(&recovered, "Inbox:\n\t- unsaved\n")?;
        let backups = Backups::new(Some(dir.path().join("backups")));
        let now = jiff::civil::date(2026, 10, 5).at(9, 30, 0, 0);
        let opened = recover(&path, &recovered, &backups, now)?;
        assert_eq!(
            serialize(&write_document(&opened.document, &opened.source)),
            "Inbox:\n\t- unsaved\n"
        );
        assert_eq!(opened.version, Some(Version::of(b"- on disk\n")));
        let copies: Vec<_> = fs::read_dir(dir.path().join("backups"))?
            .next()
            .ok_or("a backup folder")??
            .path()
            .read_dir()?
            .map(|entry| Ok::<_, io::Error>(entry?.file_name()))
            .collect::<Result<_, _>>()?;
        assert!(
            copies
                .iter()
                .any(|name| name == "2026-10-05-093000-conflict.taskpaper"),
            "{copies:?}"
        );
        fs::remove_file(&path)?;
        let opened = recover(&path, &recovered, &backups, now)?;
        assert_eq!(opened.version, None, "a missing file needs no copy");
        let error = recover(&path, &recovered, &Backups::new(None), now);
        assert!(error.is_ok(), "nor a backups folder");
        fs::write(&path, "- on disk\n")?;
        let error = recover(&path, &recovered, &Backups::new(None), now).expect_err("no copy");
        assert!(matches!(&error, OpenError::Copy { .. }), "{error:?}");
        let gone = recover(&path, &dir.path().join("gone.taskpaper"), &backups, now);
        assert!(matches!(gone, Err(OpenError::Io { .. })), "{gone:?}");
        fs::write(&recovered, b"\xff")?;
        let binary = recover(&path, &recovered, &backups, now);
        assert!(
            matches!(binary, Err(OpenError::Decode { .. })),
            "{binary:?}"
        );
        Ok(())
    }

    // F-FILE-06
    #[test]
    fn decodes_the_bytes_of_a_file_or_says_why_not() {
        let (document, source) = decode(b"Inbox:\n\t- a\n").expect("UTF-8");
        assert_eq!(
            serialize(&write_document(&document, &source)),
            "Inbox:\n\t- a\n"
        );
        assert!(decode(b"\xff").is_err());
    }

    // F-FILE-07
    #[test]
    fn a_file_that_is_not_utf8_is_refused() -> TestResult {
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("binary.taskpaper");
        fs::write(&path, b"- a\n\xff")?;
        let error = open(&path).expect_err("not UTF-8");
        assert!(matches!(&error, OpenError::Decode { .. }), "{error:?}");
        assert_eq!(
            error.to_string(),
            format!(
                "cannot open {}: the file is not valid UTF-8 (first invalid byte on line 2, at byte offset 4)",
                path.display()
            )
        );
        Ok(())
    }

    #[test]
    fn a_file_that_cannot_be_read_is_refused() -> TestResult {
        let dir = tempfile::tempdir()?;
        let error = open(dir.path()).expect_err("a folder is not a file");
        assert!(matches!(&error, OpenError::Io { .. }), "{error:?}");
        assert!(
            error
                .to_string()
                .starts_with(&format!("cannot read {}: ", dir.path().display())),
            "{error}"
        );
        Ok(())
    }
}
