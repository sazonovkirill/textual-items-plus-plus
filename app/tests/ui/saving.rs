//! Saving every change to the opened file, taking in changes another program makes to it, the
//! conflict prompt when both changed it, and the recovery file after a crash.

use std::{
    fs,
    path::{Path, PathBuf},
    time::Duration,
};

use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use tempfile::TempDir;
use textual_items_plus_plus::ui::Input;

use crate::support::{Harness, TODAY};

const TEXT: &str = "Inbox:\n\t- Renew the passport\n\t- Book flights\n";

/// The app on a file in a temporary folder holding `text`, and the file's path.
fn on_file(text: &str) -> (TempDir, PathBuf, Harness) {
    let dir = tempfile::tempdir().expect("a temporary folder");
    let path = dir.path().join("todo.taskpaper");
    fs::write(&path, text).expect("the fixture is written");
    let harness = Harness::open(path.clone(), 70, 20);
    (dir, path, harness)
}

/// The status line, without trailing spaces.
fn status(harness: &Harness) -> String {
    harness
        .rows()
        .last()
        .map(|row| row.trim_end().to_owned())
        .unwrap_or_default()
}

fn read(path: &PathBuf) -> String {
    fs::read_to_string(path).expect("the file is there")
}

// FR-STORE-09, F-FILE-05
#[test]
fn a_change_is_saved_a_second_after_it() {
    let (_dir, path, mut harness) = on_file(TEXT);
    harness.type_text("x");
    assert!(
        status(&harness).ends_with("· unsaved"),
        "{}",
        status(&harness)
    );
    harness.wait(999);
    assert_eq!(read(&path), TEXT);
    harness.wait(1);
    assert_eq!(
        read(&path),
        "Inbox:\n\t- Renew the passport @done(2026-10-05)\n\t- Book flights\n"
    );
    assert!(
        status(&harness).ends_with("· saved"),
        "{}",
        status(&harness)
    );
    assert!(
        fs::read_dir(harness.backups())
            .expect("the backups folder")
            .next()
            .is_some(),
        "the day's backup came first"
    );
}

// F-FILE-05
#[test]
fn quitting_saves_first() {
    let (_dir, path, mut harness) = on_file(TEXT);
    harness.type_text("x");
    harness.ctrl('q');
    assert!(harness.tui.should_quit());
    assert!(read(&path).contains("@done"));
    let (_dir, path, mut harness) = on_file(TEXT);
    harness.type_text("x");
    harness.terminate();
    assert!(harness.tui.should_quit());
    assert!(read(&path).contains("@done"));
}

// FR-STORE-01, D-29
#[test]
fn the_demo_is_never_saved() {
    let mut harness = Harness::new(60, 12);
    harness.type_text("x");
    harness.wait(5000);
    assert!(
        status(&harness).ends_with("· not saved"),
        "{}",
        status(&harness)
    );
    harness.ctrl('q');
    assert!(harness.tui.should_quit());
    assert!(!harness.backups().exists());
}

// F-FILE-01
#[test]
fn a_missing_file_is_created_by_the_first_save() {
    let dir = tempfile::tempdir().expect("a temporary folder");
    let path = dir.path().join("new.taskpaper");
    let mut harness = Harness::open(path.clone(), 60, 12);
    harness.type_text("o");
    harness.type_text("First");
    harness.key(KeyCode::Esc);
    harness.wait(1000);
    assert_eq!(read(&path), "Inbox:\n\t- First");
}

// F-FILE-06
#[test]
fn a_change_made_elsewhere_is_reloaded() {
    let (_dir, path, mut harness) = on_file(TEXT);
    harness.type_text("j");
    fs::write(&path, "Inbox:\n\t- Pack\n\t- Book flights\n").expect("written");
    harness.check();
    let screen = harness.screen();
    assert!(screen.contains("▌ ☐ Book flights"), "{screen}");
    assert!(screen.contains("  ☐ Pack"), "{screen}");
    assert!(
        status(&harness).ends_with("· reloaded (changed on disk)"),
        "{}",
        status(&harness)
    );
    harness.type_text("x");
    harness.wait(1000);
    assert_eq!(
        read(&path),
        "Inbox:\n\t- Pack\n\t- Book flights @done(2026-10-05)\n"
    );
}

// F-FILE-06, FR-VIEW-10
#[test]
fn a_reload_keeps_the_sidebar_on_its_entry_and_its_areas_collapsed() {
    let (_dir, path, mut harness) =
        on_file("Inbox:\n\t- t1\n\t- t2\nAlpha: @area\n\tA1:\nBeta: @area\n\tB1:\n");
    harness.type_text("g0Gkkk'");
    fs::write(&path, "Inbox:\nAlpha: @area\n\tA1:\nBeta: @area\n\tB1:\n").expect("written");
    harness.check();
    let rows = harness.rows();
    assert!(rows[8].starts_with("▌󰆧 ALPHA             ▸│"), "{rows:#?}");
    assert!(
        rows[11].starts_with(" # B1 "),
        "Beta stays expanded: {rows:#?}"
    );
}

// F-FILE-06
#[test]
fn the_watcher_sees_a_change_made_elsewhere() {
    let (_dir, path, mut harness) = on_file(TEXT);
    fs::write(&path, "Inbox:\n\t- Pack\n").expect("written");
    harness.watch_until(|harness| harness.screen().contains("☐ Pack"));
    assert!(status(&harness).ends_with("· reloaded (changed on disk)"));
}

/// The app on [`TEXT`] with a change of its own unsaved and another made on disk.
fn conflicted() -> (TempDir, PathBuf, Harness) {
    let (dir, path, mut harness) = on_file(TEXT);
    harness.type_text("x");
    fs::write(&path, "Inbox:\n\t- Renew the passport\n\t- Book trains\n").expect("written");
    harness.check();
    (dir, path, harness)
}

// F-FILE-06
#[test]
fn a_change_made_elsewhere_to_unsaved_changes_opens_the_prompt() {
    let (_dir, path, mut harness) = conflicted();
    let screen = harness.screen();
    assert!(screen.contains("Changed on disk"), "{screen}");
    assert!(
        screen.contains("The file changed on disk. Your changes are not saved yet."),
        "{screen}"
    );
    assert!(
        screen.contains("Ctrl+E Keep mine  Ctrl+R Take theirs  Ctrl+L Show diff  Ctrl+Q Quit"),
        "{screen}"
    );
    harness.wait(5000);
    harness.type_text("jx");
    assert_eq!(
        read(&path),
        "Inbox:\n\t- Renew the passport\n\t- Book trains\n"
    );
    harness.ctrl('l');
    let screen = harness.screen();
    for line in [
        "│  Inbox:",
        "│-     - Renew the passport",
        "│-     - Book trains",
        "│+     - Renew the passport @done(2026-10-05)",
        "│+     - Book flights",
    ] {
        assert!(screen.contains(line), "{line} in {screen}");
    }
    assert!(screen.contains("Ctrl+L Hide diff"), "{screen}");
    assert_eq!(
        harness.tui.cursor(harness.terminal.get_frame().area()),
        None
    );
}

/// The conflict copies in the backups folder of the file, by name.
fn conflict_copies(harness: &Harness) -> Vec<PathBuf> {
    let mut copies: Vec<_> = fs::read_dir(harness.backups())
        .expect("the backups folder")
        .flat_map(|folder| fs::read_dir(folder.expect("a folder").path()).expect("its copies"))
        .map(|copy| copy.expect("a copy").path())
        .filter(|copy| copy.to_string_lossy().ends_with("-conflict.taskpaper"))
        .collect();
    copies.sort();
    copies
}

// F-FILE-06
#[test]
fn keep_mine_keeps_theirs_in_a_conflict_copy_and_overwrites_the_file() {
    let (_dir, path, mut harness) = conflicted();
    harness.ctrl('e');
    assert!(!harness.screen().contains("Changed on disk"));
    let copies = conflict_copies(&harness);
    assert_eq!(copies.len(), 1);
    assert_eq!(
        read(&copies[0]),
        "Inbox:\n\t- Renew the passport\n\t- Book trains\n"
    );
    assert!(
        status(&harness).contains(" · kept mine; theirs is in /"),
        "{}",
        status(&harness)
    );
    harness.wait(1000);
    assert_eq!(
        read(&path),
        "Inbox:\n\t- Renew the passport @done(2026-10-05)\n\t- Book flights\n"
    );
}

// F-FILE-06
#[test]
fn take_theirs_keeps_mine_in_a_conflict_copy_and_reloads_the_file() {
    let (_dir, path, mut harness) = conflicted();
    harness.ctrl('r');
    let copies = conflict_copies(&harness);
    assert_eq!(copies.len(), 1);
    assert!(read(&copies[0]).contains("@done"));
    let screen = harness.screen();
    assert!(screen.contains("☐ Book trains"), "{screen}");
    assert!(
        status(&harness).contains("took theirs; mine is in"),
        "{}",
        status(&harness)
    );
    harness.type_text("z");
    harness.wait(5000);
    assert_eq!(
        read(&path),
        "Inbox:\n\t- Renew the passport\n\t- Book trains\n",
        "no undo"
    );
}

// F-FILE-05, F-FILE-06
#[test]
fn quitting_with_the_prompt_open_leaves_theirs_and_keeps_mine_in_a_copy() {
    let (_dir, path, mut harness) = conflicted();
    harness.ctrl('q');
    assert!(harness.tui.should_quit());
    assert_eq!(
        read(&path),
        "Inbox:\n\t- Renew the passport\n\t- Book trains\n"
    );
    let copies = conflict_copies(&harness);
    assert_eq!(copies.len(), 1);
    assert!(read(&copies[0]).contains("@done"));
    assert_eq!(
        harness.tui.farewell(),
        Some(format!(
            "tipp: left theirs on disk; mine is in {}",
            copies[0].display()
        ))
    );
}

// F-FILE-05, F-FILE-06
#[test]
fn ending_once_a_save_is_due_with_the_prompt_open_still_leaves_theirs() {
    for terminate in [false, true] {
        let (_dir, path, mut harness) = conflicted();
        harness.wait(5000);
        if terminate {
            harness.terminate();
        } else {
            harness.ctrl('q');
        }
        assert!(harness.tui.should_quit());
        harness.wait(5000);
        assert_eq!(
            read(&path),
            "Inbox:\n\t- Renew the passport\n\t- Book trains\n",
            "terminate: {terminate}"
        );
        assert!(read(&conflict_copies(&harness)[0]).contains("@done"));
    }
}

// F-FILE-05, F-FILE-06
#[test]
fn quitting_before_a_choice_is_reported_still_quits_keeping_mine_in_a_copy() {
    for choice in ['e', 'r'] {
        let (_dir, path, mut harness) = conflicted();
        let ctrl = |c| Event::Key(KeyEvent::new(KeyCode::Char(c), KeyModifiers::CONTROL));
        harness.tui.handle(&ctrl(choice), TODAY);
        harness.tui.flush();
        harness.send(&ctrl('q'));
        assert!(harness.tui.should_quit(), "after Ctrl+{choice}");
        assert_eq!(
            read(&path),
            "Inbox:\n\t- Renew the passport\n\t- Book trains\n"
        );
        let farewell = harness.tui.farewell().expect("a farewell");
        let mine = farewell
            .strip_prefix("tipp: left theirs on disk; mine is in ")
            .expect("mine is in a copy");
        assert!(read(&PathBuf::from(mine)).contains("@done"), "{farewell}");
    }
}

// F-FILE-06, F-FILE-07
#[test]
fn a_file_made_unreadable_elsewhere_asks_and_cannot_be_taken() {
    let (_dir, path, mut harness) = on_file(TEXT);
    fs::write(&path, b"\xff\xfe").expect("written");
    harness.check();
    let screen = harness.screen();
    assert!(
        screen.contains("The file on disk is not UTF-8, so it cannot be reloaded."),
        "{screen}"
    );
    harness.ctrl('r');
    assert!(
        status(&harness).ends_with("cannot take theirs: the file on disk is not UTF-8"),
        "{}",
        status(&harness)
    );
    harness.ctrl('l');
    assert!(harness.screen().contains("+     - Book flights"));
    harness.ctrl('e');
    harness.wait(1000);
    assert_eq!(read(&path), TEXT);
    assert_eq!(
        fs::read(&conflict_copies(&harness)[0]).expect("the copy"),
        b"\xff\xfe"
    );
}

// F-FILE-06
#[test]
fn a_file_removed_elsewhere_is_written_again_by_the_next_save() {
    let (_dir, path, mut harness) = on_file(TEXT);
    fs::remove_file(&path).expect("removed");
    harness.check();
    assert!(
        status(&harness).ends_with("· not on disk"),
        "{}",
        status(&harness)
    );
    harness.type_text("x");
    harness.wait(1000);
    assert!(read(&path).contains("@done"));
    assert!(
        status(&harness).ends_with("· saved"),
        "{}",
        status(&harness)
    );
}

// N-REL-01
#[test]
fn after_a_crash_the_document_goes_to_a_recovery_file_that_opens_next_time() {
    let (_dir, path, mut harness) = on_file(TEXT);
    assert_eq!(harness.tui.recover().expect("nothing to write"), None);
    harness.type_text("x");
    let recovered = harness
        .tui
        .recover()
        .expect("written")
        .expect("there were changes");
    assert!(
        recovered.starts_with(harness.recovery()),
        "{}",
        recovered.display()
    );
    let mut harness = Harness::on(Some(path.clone()), Some(&recovered), 70, 20)
        .expect("the recovered document opens");
    assert!(
        status(&harness).ends_with("· unsaved"),
        "{}",
        status(&harness)
    );
    assert_eq!(conflict_copies(&harness).len(), 1, "the file as it was");
    harness.wait(1000);
    assert!(read(&path).contains("@done"));
    let gone = recovered.with_extension("gone");
    let error = Harness::on(Some(path), Some(&gone), 70, 20)
        .err()
        .expect("no recovery file");
    assert!(error.to_string().contains("cannot read"), "{error}");
}

// N-REL-01, D-96
#[test]
fn a_crash_while_the_last_save_is_under_way_writes_no_recovery_file_once_it_lands() {
    let (_dir, path, mut harness) = on_file(TEXT);
    harness.type_text("x");
    // As when the terminal is closed: the save is asked for, and the session ends before the
    // event loop hears that it landed.
    harness.tui.input(Input::Terminate, TODAY);
    assert_eq!(harness.tui.recover().expect("nothing to write"), None);
    assert!(read(&path).contains("@done"));
    assert!(!harness.recovery().exists(), "no recovery file");
}

// F-FILE-06
#[test]
fn a_save_refused_because_the_file_changed_opens_the_prompt() {
    let (_dir, path, mut harness) = on_file(TEXT);
    harness.check();
    assert!(status(&harness).ends_with("· saved"), "nothing changed");
    harness.type_text("x");
    assert_eq!(harness.tui.deadline(), Some(Duration::from_secs(1)));
    fs::write(&path, "Inbox:\n\t- Pack\n").expect("written");
    harness.wait(1000);
    assert!(harness.screen().contains("Changed on disk"));
    assert_eq!(harness.tui.deadline(), None, "no save is due");
    assert_eq!(read(&path), "Inbox:\n\t- Pack\n");
}

// F-FILE-06, F-FILE-07
#[test]
fn unsaved_changes_and_a_file_made_unreadable_elsewhere_ask_too() {
    let (_dir, path, mut harness) = on_file(TEXT);
    harness.type_text("x");
    fs::write(&path, b"\xff").expect("written");
    harness.check();
    let screen = harness.screen();
    assert!(
        screen.contains("The file changed on disk and is not UTF-8."),
        "{screen}"
    );
}

/// Puts a file where `folder` was, so nothing can be written in it.
fn block(folder: &Path) {
    if folder.exists() {
        fs::remove_dir_all(folder).expect("removed");
    }
    fs::write(folder, "").expect("written");
}

// F-FILE-09
#[test]
fn a_failed_backup_is_shown_and_saving_goes_on() {
    let dir = tempfile::tempdir().expect("a temporary folder");
    let path = dir.path().join("todo.taskpaper");
    fs::write(&path, TEXT).expect("the fixture is written");
    let folders = tempfile::tempdir().expect("a temporary folder");
    block(&folders.path().join("backups"));
    let mut harness =
        Harness::launched_in(folders, Some(path.clone()), 70, 20).expect("the file opens");
    assert!(
        status(&harness).contains("backup failed: "),
        "{}",
        status(&harness)
    );
    harness.type_text("x");
    harness.wait(1000);
    assert!(read(&path).contains("@done"));
}

// FR-STORE-09, F-FILE-05
#[test]
fn a_failed_save_is_shown_and_quitting_then_asks_again() {
    let (dir, _path, mut harness) = on_file(TEXT);
    fs::remove_dir_all(dir.path()).expect("removed");
    harness.type_text("x");
    harness.wait(1000);
    assert!(
        status(&harness).contains("save failed: the folder "),
        "{}",
        status(&harness)
    );
    harness.ctrl('q');
    assert!(
        status(&harness).contains("quit again to quit without saving; save failed: "),
        "{}",
        status(&harness)
    );
    assert!(!harness.tui.should_quit());
    harness.ctrl('q');
    assert!(harness.tui.should_quit());
}

// F-FILE-06
#[test]
fn a_conflict_copy_that_fails_leaves_the_prompt_open() {
    let (_dir, _path, mut harness) = conflicted();
    block(&harness.backups());
    harness.ctrl('e');
    assert!(harness.screen().contains("Changed on disk"));
    assert!(
        status(&harness).contains("conflict copy failed: "),
        "{}",
        status(&harness)
    );
}

// F-FILE-05, N-REL-01
#[test]
fn ending_with_the_prompt_open_falls_back_to_a_recovery_file_then_asks() {
    let (_dir, _path, mut harness) = conflicted();
    block(&harness.backups());
    harness.terminate();
    assert!(harness.tui.should_quit());
    let farewell = harness.tui.farewell().expect("where mine is");
    let recovery = harness.recovery();
    assert!(
        farewell.contains(&recovery.display().to_string()),
        "{farewell}"
    );
    let (_dir, _path, mut harness) = conflicted();
    block(&harness.backups());
    block(&harness.recovery());
    harness.ctrl('q');
    assert!(!harness.tui.should_quit());
    let status = status(&harness);
    assert!(
        status.contains("quit again to quit without saving; conflict copy failed:"),
        "{status}"
    );
    harness.ctrl('q');
    assert!(harness.tui.should_quit());
    assert_eq!(harness.tui.farewell(), None);
}

#[test]
fn terminal_events_reach_the_app_as_input() {
    let mut harness = Harness::new(40, 10);
    harness.tui.input(
        Input::Terminal(Event::Key(KeyEvent::new(
            KeyCode::Char('q'),
            KeyModifiers::CONTROL,
        ))),
        TODAY,
    );
    assert!(harness.tui.should_quit());
}
