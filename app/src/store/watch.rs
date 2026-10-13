//! Watching the open file for changes made by other programs (F-FILE-06): each burst of changes
//! in its folder asks for a check, and the check tells whether the file itself changed.

use std::{fs, path::Path, time::Duration};

use notify_debouncer_full::{
    DebounceEventResult, Debouncer, RecommendedCache, new_debouncer,
    notify::{
        self, EventKind, RecommendedWatcher, RecursiveMode,
        event::{AccessKind, AccessMode},
    },
};

/// How long the folder must stay quiet before a burst of changes in it is reported.
const QUIET: Duration = Duration::from_millis(200);

/// Watches a file until dropped.
#[derive(Debug)]
pub struct Watcher {
    _debouncer: Debouncer<RecommendedWatcher, RecommendedCache>,
}

/// Watches the folder of the file at `path`, or of a link's target, and calls `check` on a thread
/// of the watcher's after each burst of changes in it that may have changed the file. The folder
/// is watched rather than the file, because many editors save by renaming a new file over the old
/// one, and the file may not exist yet.
///
/// # Errors
///
/// The error of the system's watcher, or why `path` cannot be made absolute.
pub fn watch(path: &Path, mut check: impl FnMut() + Send + 'static) -> notify::Result<Watcher> {
    let file = fs::canonicalize(path).or_else(|_| std::path::absolute(path))?;
    let folder = file.parent().unwrap_or(&file);
    let on_burst = move |result: DebounceEventResult| {
        if may_change(&result) {
            check();
        }
    };
    let mut debouncer = new_debouncer(QUIET, None, on_burst)?;
    debouncer.watch(folder, RecursiveMode::NonRecursive)?;
    Ok(Watcher {
        _debouncer: debouncer,
    })
}

/// Whether a burst may have changed a file: an error of the watcher may hide a change, and
/// reading the file again is cheap, so neither which file changed nor the error needs telling
/// apart.
fn may_change(result: &DebounceEventResult) -> bool {
    result
        .as_ref()
        .map_or(true, |events| events.iter().any(|event| writes(event.kind)))
}

/// Whether an event of `kind` may write: anything but files opened, read or closed unwritten,
/// such as the check's own read of the file, which would otherwise ask for the next check and
/// keep reading the file for as long as the app runs.
fn writes(kind: EventKind) -> bool {
    match kind {
        EventKind::Access(access) => access == AccessKind::Close(AccessMode::Write),
        _ => true,
    }
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc;

    use super::*;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    /// Watches `path`, sending `()` to the returned receiver for each check asked for.
    fn checks(path: &Path) -> notify::Result<(Watcher, mpsc::Receiver<()>)> {
        let (sender, received) = mpsc::channel();
        let watcher = watch(path, move || {
            let _ = sender.send(());
        })?;
        Ok((watcher, received))
    }

    // F-FILE-06
    #[test]
    fn a_change_to_the_file_asks_for_a_check() -> TestResult {
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("todo.taskpaper");
        fs::write(&path, "- a\n")?;
        let (_watcher, received) = checks(&path)?;
        fs::write(&path, "- b\n")?;
        received.recv_timeout(Duration::from_secs(10))?;
        Ok(())
    }

    // F-FILE-06
    #[test]
    fn a_file_that_does_not_exist_yet_is_watched_for_its_creation() -> TestResult {
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("new.taskpaper");
        let (_watcher, received) = checks(&path)?;
        fs::write(&path, "- a\n")?;
        received.recv_timeout(Duration::from_secs(10))?;
        Ok(())
    }

    // F-FILE-06
    #[test]
    fn reading_the_file_asks_for_no_check() -> TestResult {
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("todo.taskpaper");
        fs::write(&path, "- a\n")?;
        let (_watcher, received) = checks(&path)?;
        fs::read_to_string(&path)?;
        assert!(
            received.recv_timeout(Duration::from_millis(600)).is_err(),
            "a read is no change"
        );
        fs::write(&path, "- b\n")?;
        received.recv_timeout(Duration::from_secs(10))?;
        Ok(())
    }

    #[test]
    fn a_dropped_watcher_asks_for_nothing() -> TestResult {
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("todo.taskpaper");
        let (watcher, received) = checks(&path)?;
        drop(watcher);
        fs::write(&path, "- a\n")?;
        assert!(received.recv_timeout(Duration::from_millis(600)).is_err());
        Ok(())
    }

    #[test]
    fn watching_a_file_in_a_missing_folder_fails() {
        assert!(checks(Path::new("/nonexistent-folder/todo.taskpaper")).is_err());
        assert!(checks(Path::new("")).is_err());
    }

    #[test]
    fn only_events_that_may_write_ask_for_a_check() {
        assert!(!writes(EventKind::Access(AccessKind::Close(
            AccessMode::Read
        ))));
        assert!(!writes(EventKind::Access(AccessKind::Open(
            AccessMode::Any
        ))));
        assert!(writes(EventKind::Access(AccessKind::Close(
            AccessMode::Write
        ))));
        assert!(writes(EventKind::Modify(notify::event::ModifyKind::Any)));
        assert!(writes(EventKind::Remove(notify::event::RemoveKind::File)));
        assert!(
            may_change(&Err(vec![notify::Error::generic("lost")])),
            "an error may hide a change"
        );
        assert!(!may_change(&Ok(Vec::new())));
    }
}
