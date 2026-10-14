//! Opening a `.taskpaper` file: reading it from disk into the domain model (FR-STORE-01,
//! F-FILE-07).

use std::{io, path::Path};

use thiserror::Error;

use super::{
    disk,
    taskpaper::{DecodeError, Source, parse_bytes, read_document},
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
}

/// The document the file at `path` holds, with the outline it was read from, which saving it
/// back needs; a file that does not exist holds an empty document.
///
/// # Errors
///
/// [`OpenError::Io`] when the file cannot be read, and [`OpenError::Decode`] when it is not
/// UTF-8.
pub fn open(path: &Path) -> Result<(Document, Source), OpenError> {
    let bytes = disk::read(path).map_err(|source| OpenError::Io {
        path: path.into(),
        source,
    })?;
    let outline = parse_bytes(&bytes.unwrap_or_default()).map_err(|source| OpenError::Decode {
        path: path.into(),
        source,
    })?;
    Ok(read_document(&outline))
}

#[cfg(test)]
mod tests {
    use std::fs;

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
        let (document, source) = open(&path)?;
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
        let (document, _) = open(&dir.path().join("new.taskpaper"))?;
        assert_eq!(document, read_document(&parse("")).0);
        assert_eq!(document.children(document.inbox()), []);
        Ok(())
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
