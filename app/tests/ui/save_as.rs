//! Save As: writing the document to another file, which the app saves to from then on
//! (FR-STORE-11).

use std::{
    fs,
    path::{Path, PathBuf},
};

use crossterm::event::{Event, KeyCode};
use tempfile::TempDir;

use crate::support::Harness;

const TEXT: &str = "Inbox:\n\t- Renew the passport\n\t- Book flights\n";

/// The app on a file in a temporary folder holding [`TEXT`], and the file's path.
fn on_file() -> (TempDir, PathBuf, Harness) {
    let dir = tempfile::tempdir().expect("a temporary folder");
    let path = dir.path().join("todo.taskpaper");
    fs::write(&path, TEXT).expect("the fixture is written");
    let harness = Harness::open(path.clone(), 120, 20);
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

fn read(path: &Path) -> String {
    fs::read_to_string(path).expect("the file is there")
}

/// Opens Save As with `Ctrl+S`, types `path` over the one shown and presses `Enter`.
fn save_as(harness: &mut Harness, path: &str) {
    harness.ctrl('s');
    harness.type_text(path);
    harness.key(KeyCode::Enter);
}

// FR-STORE-11
#[test]
fn ctrl_s_saves_to_another_file_which_is_saved_and_watched_from_then_on() {
    let (dir, path, mut harness) = on_file();
    harness.ctrl('s');
    let screen = harness.screen();
    assert!(screen.contains("Save As"), "{screen}");
    assert!(screen.contains(&path.display().to_string()), "{screen}");
    assert!(screen.contains("Enter saves here, Esc cancels"), "{screen}");
    harness.key(KeyCode::Esc);
    assert!(!harness.screen().contains("Save As"));
    let new = dir.path().join("new.taskpaper");
    save_as(&mut harness, &new.display().to_string());
    assert!(
        !harness.screen().contains("Enter saves here"),
        "the prompt closed"
    );
    assert_eq!(read(&new), TEXT, "written as the file was read");
    assert!(
        status(&harness).contains(&format!("saved as {}", new.display())),
        "{}",
        status(&harness)
    );
    harness.type_text("x");
    harness.wait(1000);
    assert!(
        read(&new).contains("Renew the passport @done"),
        "{}",
        read(&new)
    );
    assert_eq!(read(&path), TEXT, "the file before is left as it was");
    fs::write(&new, "Inbox:\n\t- Changed elsewhere\n").expect("the change is written");
    harness.check();
    assert!(
        harness.screen().contains("Changed elsewhere"),
        "{}",
        harness.screen()
    );
}

// FR-STORE-11: the file a Save As replaces is kept in a conflict copy.
#[test]
fn a_file_there_is_replaced_once_asked_and_kept_in_a_conflict_copy() {
    let (dir, _path, mut harness) = on_file();
    let other = dir.path().join("other.taskpaper");
    fs::write(&other, "Inbox:\n\t- Other\n").expect("the other file is written");
    save_as(&mut harness, &other.display().to_string());
    let screen = harness.screen();
    assert!(
        screen.contains("The file exists: Enter replaces it, Esc goes back"),
        "{screen}"
    );
    harness.key(KeyCode::Esc);
    assert!(harness.screen().contains("Enter saves here, Esc cancels"));
    harness.key(KeyCode::Enter);
    harness.key(KeyCode::Enter);
    assert_eq!(read(&other), TEXT);
    let kept: Vec<String> = fs::read_dir(harness.backups())
        .expect("the backups folder")
        .flat_map(|folder| fs::read_dir(folder.expect("a folder").path()).expect("its files"))
        .map(|file| file.expect("a file").path())
        .filter(|file| file.to_string_lossy().ends_with("-conflict.taskpaper"))
        .map(|file| read(&file))
        .collect();
    assert_eq!(kept, ["Inbox:\n\t- Other\n"]);
}

// FR-STORE-11
#[test]
fn a_file_that_cannot_be_written_says_why_and_takes_the_path_again() {
    let (dir, _path, mut harness) = on_file();
    let missing = dir.path().join("missing");
    save_as(
        &mut harness,
        &missing.join("x.taskpaper").display().to_string(),
    );
    let screen = harness.screen();
    assert!(
        screen.contains(&format!(
            "Cannot save: the folder {} does not exist",
            missing.display()
        )),
        "{screen}"
    );
    harness.ctrl('k');
    assert!(
        !harness.screen().contains("Save As"),
        "the Navigator closes it"
    );
}

// FR-STORE-11, FR-STORE-01: the demo document is kept once it has a file.
#[test]
fn the_demo_saved_as_a_file_in_the_home_folder_opens_again_as_it_was() {
    let mut harness = Harness::new(120, 30);
    fs::create_dir(harness.home()).expect("the home folder is made");
    assert!(
        status(&harness).ends_with("not saved"),
        "{}",
        status(&harness)
    );
    save_as(&mut harness, "~/demo.taskpaper");
    let path = harness.home().join("demo.taskpaper");
    assert!(read(&path).starts_with("Inbox:"), "{}", read(&path));
    let before = harness.rows();
    let rows = before.len() - 1;
    harness.key(KeyCode::Char('x'));
    harness.wait(1000);
    assert!(status(&harness).ends_with("saved"), "{}", status(&harness));
    harness.key(KeyCode::Char('z'));
    harness.wait(1000);
    let harness = harness.relaunch(path);
    assert_eq!(harness.rows()[..rows], before[..rows]);
}

// FR-STORE-11
#[test]
fn the_demo_replaces_a_file_once_asked_and_says_why_it_cannot_write_one() {
    let mut harness = Harness::new(120, 30);
    fs::create_dir(harness.home()).expect("the home folder is made");
    let path = harness.home().join("todo.taskpaper");
    fs::write(&path, TEXT).expect("the file is written");
    save_as(&mut harness, "~/missing/todo.taskpaper");
    let screen = harness.screen();
    assert!(screen.contains("Cannot save: the folder"), "{screen}");
    harness.key(KeyCode::Esc);
    save_as(&mut harness, "~/todo.taskpaper");
    let screen = harness.screen();
    assert!(screen.contains("The file exists"), "{screen}");
    assert_eq!(read(&path), TEXT);
    harness.key(KeyCode::Enter);
    assert!(
        read(&path).contains("Read the article Sam sent"),
        "{}",
        read(&path)
    );
}

// FR-STORE-11, FR-NAV-06, FR-OPS-05: the prompt takes a paste on its one line.
#[test]
fn the_palette_opens_save_as_which_takes_a_paste() {
    let (_dir, _path, mut harness) = on_file();
    harness.ctrl('p');
    harness.type_text("save as");
    harness.key(KeyCode::Enter);
    assert!(
        harness.screen().contains("Enter saves here"),
        "{}",
        harness.screen()
    );
    harness.send(&Event::Paste("/notes\nnew.taskpaper\n".to_owned()));
    let screen = harness.screen();
    assert!(
        screen.contains("│ /notes new.taskpaper "),
        "in place of the path selected: {screen}"
    );
}
