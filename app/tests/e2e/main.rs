//! End-to-end smoke tests of the real binary in a pseudo-terminal, with termlens.

use std::{fs, path::Path, process::Command};

type TestResult = Result<(), Box<dyn std::error::Error>>;

// FR-STORE-01, FR-KEY-01, FR-NAV-01, FR-EDIT-07, N-PORT-03
#[test]
fn opens_the_demo_inbox_moves_through_it_and_quits_with_ctrl_q_from_quick_add_restoring_the_terminal()
-> TestResult {
    let mut terminal = termlens::bin!("tipp")?;
    terminal.wait_until(|screen| screen.text().contains("Call the bank about the card"))?;
    terminal.send(termlens::Key::Char('j'))?;
    terminal.wait_until(|screen| screen.text().contains("▌ ☐ Buy coffee beans"))?;
    terminal.send(termlens::Key::Char('q'))?;
    terminal.wait_until(|screen| screen.text().contains("Quick Add → Inbox"))?;
    terminal.send(termlens::Key::Ctrl('q'))?;
    assert!(terminal.wait_exit()?.success());
    assert!(!terminal.screen().alternate_screen());
    Ok(())
}

/// Runs the app on `file` with its output, and what it reports, passing through a reader the
/// test stops by writing to the pipe `stop`; the reader then writes to the pipe `stopped`.
const OUTPUT_THROUGH_A_READER: &str = r#"
"$0" "$1" 2>&1 | {
    exec 3<&0
    cat <&3 & reader=$!
    exec 0<&- 3<&-
    read _ < "$2"
    kill "$reader"
    wait "$reader"
    echo > "$3"
}
"#;

// N-REL-01, D-38, D-96
#[test]
fn a_change_outlives_the_terminal_that_goes_away_before_it_is_saved() -> TestResult {
    let dir = tempfile::tempdir()?;
    let file = dir.path().join("todo.taskpaper");
    fs::write(&file, "Inbox:\n\t- Renew the passport\n")?;
    let [stop, stopped] = ["stop", "stopped"].map(|name| dir.path().join(name));
    for pipe in [&stop, &stopped] {
        assert!(Command::new("mkfifo").arg(pipe).status()?.success());
    }
    let state = dir.path().join("state");
    let mut terminal = termlens::Terminal::builder()
        .env("HOME", dir.path())
        .env("XDG_STATE_HOME", &state)
        .env("XDG_DATA_HOME", dir.path().join("data"))
        .env("XDG_CONFIG_HOME", dir.path().join("config"))
        .args([
            "-c",
            OUTPUT_THROUGH_A_READER,
            env!("CARGO_BIN_EXE_tipp"),
            &file.to_string_lossy(),
            &stop.to_string_lossy(),
            &stopped.to_string_lossy(),
        ])
        .spawn("sh")?;
    terminal.wait_until(|screen| screen.text().contains("Renew the passport"))?;
    fs::write(&stop, "\n")?;
    fs::read(&stopped)?;
    // The next frame finds the terminal gone, with the change not yet saved.
    terminal.send(termlens::Key::Char('x'))?;
    terminal.wait_exit()?;
    let kept = holding_done(&file)
        || recovery_files(&state)?
            .iter()
            .any(|path| holding_done(path));
    assert!(kept, "the change is neither saved nor in a recovery file");
    Ok(())
}

/// Whether the file at `path` holds a completed task.
fn holding_done(path: &Path) -> bool {
    fs::read_to_string(path).is_ok_and(|text| text.contains("@done"))
}

/// The files under `folder`, at any depth.
fn recovery_files(folder: &Path) -> std::io::Result<Vec<std::path::PathBuf>> {
    let mut files = Vec::new();
    if !folder.exists() {
        return Ok(files);
    }
    for entry in fs::read_dir(folder)? {
        let path = entry?.path();
        if path.is_dir() {
            files.extend(recovery_files(&path)?);
        } else {
            files.push(path);
        }
    }
    Ok(files)
}
