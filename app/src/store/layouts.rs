//! The layouts file: the pane layout of each file the app opened, keyed by the file's canonical
//! path, so that the next launch of the file shows it as it was left (FR-PANE-03,
//! [file-io.md §5](../../../specs/reference/taskpaper-terminal/specs/file-io.md#5-view-state-persistence)).
//! Hides the file's format; what a layout holds is the core's.

use std::{
    fs::{self, Permissions},
    io,
    os::unix::fs::PermissionsExt as _,
    path::{Path, PathBuf},
};

use serde::{Serialize, de::DeserializeOwned};
use serde_json::{Map, Value};
use thiserror::Error;

use super::atomic;

/// Only the user may read the file, since it holds titles from every file opened.
const MODE: u32 = 0o600;

/// Why a layout could not be kept.
#[derive(Debug, Error)]
pub enum LayoutsError {
    /// Neither `$XDG_STATE_HOME` nor a home folder says where the layouts file goes.
    #[error("no place for the layouts file: neither $XDG_STATE_HOME nor a home folder is set")]
    NoFile,
    /// The layout cannot be written as JSON.
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    /// Reading or writing the file failed.
    #[error(transparent)]
    Io(#[from] io::Error),
}

/// The layouts kept in one file.
#[derive(Debug, Clone)]
pub struct Layouts {
    file: Option<PathBuf>,
}

impl Layouts {
    /// The layouts kept in `file`, or none anywhere when it is `None`.
    #[must_use]
    pub fn new(file: Option<PathBuf>) -> Self {
        Self { file }
    }

    /// The layout kept for the file at `path`, if one can be read. A layout never keeps a file
    /// from opening, so a missing or unreadable layouts file, or a layout in it that cannot be
    /// read, is none.
    #[must_use]
    pub fn load<T: DeserializeOwned>(&self, path: &Path) -> Option<T> {
        let bytes = fs::read(self.file.as_ref()?).ok()?;
        let mut layouts: Map<String, Value> = serde_json::from_slice(&bytes).ok()?;
        serde_json::from_value(layouts.remove(&key(path))?).ok()
    }

    /// Keeps `layout` as the layout of the file at `path`, replacing the layouts file atomically
    /// and keeping the layouts of the other files in it. A layouts file that is not a set of
    /// layouts is replaced, as nothing in it could be read back.
    ///
    /// # Errors
    ///
    /// [`LayoutsError`] when there is no place for the file, `layout` cannot be written as JSON,
    /// or the file could not be read or written.
    pub fn keep<T: Serialize>(&self, path: &Path, layout: &T) -> Result<(), LayoutsError> {
        let file = self.file.as_deref().ok_or(LayoutsError::NoFile)?;
        let mut layouts: Map<String, Value> = match fs::read(file) {
            Ok(bytes) => serde_json::from_slice(&bytes).unwrap_or_default(),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Map::new(),
            Err(error) => return Err(error.into()),
        };
        layouts.insert(key(path), serde_json::to_value(layout)?);
        file.parent().map(fs::create_dir_all).transpose()?;
        let text = serde_json::to_vec_pretty(&layouts)?;
        atomic::replace(file, &text, Some(Permissions::from_mode(MODE)))?;
        Ok(())
    }
}

/// The key of the file at `path`: its canonical path, or its absolute path while it does not
/// exist.
fn key(path: &Path) -> String {
    fs::canonicalize(path)
        .or_else(|_| std::path::absolute(path))
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned()
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, os::unix::fs::symlink};

    use super::*;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    fn layouts(dir: &Path) -> Layouts {
        Layouts::new(Some(dir.join("state/layouts.json")))
    }

    // FR-PANE-03
    #[test]
    fn a_kept_layout_is_loaded_for_its_file_and_only_the_user_may_read_it() -> TestResult {
        let dir = tempfile::tempdir()?;
        let todo = dir.path().join("todo.taskpaper");
        fs::write(&todo, "")?;
        let layouts = layouts(dir.path());
        assert_eq!(layouts.load::<Vec<u8>>(&todo), None, "nothing kept yet");
        layouts.keep(&todo, &vec![1_u8, 2])?;
        assert_eq!(layouts.load::<Vec<u8>>(&todo), Some(vec![1, 2]));
        assert_eq!(
            layouts.load::<String>(&todo),
            None,
            "not a layout of this kind"
        );
        let mode = fs::metadata(dir.path().join("state/layouts.json"))?
            .permissions()
            .mode();
        assert_eq!(mode & 0o777, MODE);
        Ok(())
    }

    // FR-PANE-03
    #[test]
    fn layouts_are_kept_by_the_canonical_path_beside_those_of_other_files() -> TestResult {
        let dir = tempfile::tempdir()?;
        let todo = dir.path().join("todo.taskpaper");
        fs::write(&todo, "")?;
        let link = dir.path().join("link.taskpaper");
        symlink(&todo, &link)?;
        let new = dir.path().join("new.taskpaper");
        let layouts = layouts(dir.path());
        layouts.keep(&link, &1)?;
        layouts.keep(&new, &2)?;
        assert_eq!(
            layouts.load::<u8>(&todo),
            Some(1),
            "the file through a link"
        );
        assert_eq!(layouts.load::<u8>(&new), Some(2), "a file not there yet");
        let kept: Value =
            serde_json::from_slice(&fs::read(dir.path().join("state/layouts.json"))?)?;
        let canonical = fs::canonicalize(&todo)?;
        assert_eq!(
            kept,
            serde_json::json!({
                canonical.to_string_lossy(): 1,
                new.to_string_lossy(): 2,
            })
        );
        Ok(())
    }

    // FR-PANE-03
    #[test]
    fn an_unreadable_layouts_file_loads_nothing_and_is_replaced() -> TestResult {
        let dir = tempfile::tempdir()?;
        let todo = dir.path().join("todo.taskpaper");
        let file = dir.path().join("state/layouts.json");
        fs::create_dir_all(dir.path().join("state"))?;
        fs::write(&file, "not json")?;
        let layouts = layouts(dir.path());
        assert_eq!(layouts.load::<u8>(&todo), None);
        layouts.keep(&todo, &3)?;
        assert_eq!(layouts.load::<u8>(&todo), Some(3));
        Ok(())
    }

    // FR-PANE-03
    #[test]
    fn keeping_fails_without_a_place_or_a_layout_json_can_hold() -> TestResult {
        let dir = tempfile::tempdir()?;
        let todo = dir.path().join("todo.taskpaper");
        let nowhere = Layouts::new(None);
        assert_eq!(nowhere.load::<u8>(&todo), None);
        assert!(matches!(nowhere.keep(&todo, &1), Err(LayoutsError::NoFile)));
        let not_json = HashMap::from([((1, 2), 3)]);
        assert!(matches!(
            layouts(dir.path()).keep(&todo, &not_json),
            Err(LayoutsError::Json(_))
        ));
        let in_a_file = Layouts::new(Some(todo.join("layouts.json")));
        fs::write(&todo, "")?;
        assert!(matches!(
            in_a_file.keep(&todo, &1),
            Err(LayoutsError::Io(_))
        ));
        Ok(())
    }
}
