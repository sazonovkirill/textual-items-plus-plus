//! Performance benchmarks for N-PERF-01 and N-PERF-02: opening a file of 50,000 items and the time
//! from a keystroke to its drawn frame on it, palettes and search included (plan step 6.3).
//!
//! `bash app/scripts/validate.sh bench` (or `cargo bench --bench perf`) builds the file, measures
//! each scenario and fails when one misses its budget; text after `--` picks the scenarios whose
//! name holds it. Run as a test, as `cargo test --all-targets` does, it drives the same scenarios
//! once on a small file without timing them, so they cannot rot.

use std::{
    fmt::Write as _,
    path::Path,
    process::ExitCode,
    sync::mpsc::{self, Receiver},
    time::{Duration, Instant},
};

use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use jiff::civil::{Date, DateTime, date};
use ratatui::{Terminal, backend::TestBackend};
use textual_items_plus_plus::{
    app::Animations,
    ui::{Edge, Input, Tui},
};

/// The local date the benchmarks run on, so the file's dates are near it.
const TODAY: Date = date(2026, 10, 5);

/// The local time the app reads.
const NOW: DateTime = TODAY.at(9, 30, 0, 0);

/// The file N-PERF-01 names: 50,000 items.
const ITEMS: usize = 50_000;

/// The terminal the benchmarks draw on.
const WIDTH: u16 = 160;
const HEIGHT: u16 = 48;

/// N-PERF-01: open and draw.
const OPEN_BUDGET: Duration = Duration::from_millis(300);

/// N-PERF-02: keystroke to drawn frame.
const KEY_BUDGET: Duration = Duration::from_millis(16);

/// How often each keystroke scenario is measured at most; the slowest run is held to the budget.
const RUNS: usize = 15;

/// A scenario stops repeating once its runs took this long, so a slow one does not take minutes.
const SCENARIO_TIME: Duration = Duration::from_secs(2);

fn main() -> ExitCode {
    // `cargo bench` passes `--bench`; a test run passes nothing of the sort.
    let timed = std::env::args().any(|arg| arg == "--bench");
    // `cargo bench --bench perf -- <text>` runs only the scenarios whose name holds the text.
    let only: Vec<String> = std::env::args()
        .skip(1)
        .filter(|arg| !arg.starts_with('-'))
        .collect();
    let items = if timed { ITEMS } else { 2_000 };
    let runs = if timed { RUNS } else { 1 };
    let dir = tempfile::tempdir().expect("a temporary folder");
    let path = dir.path().join("large.taskpaper");
    let (text, written) = file(items);
    assert!(written >= items, "{written} items");
    std::fs::write(&path, &text).expect("the temporary folder is writable");
    let mut results = Vec::new();
    let (mut app, open) = App::open(&path, dir.path());
    results.push(Report::new("open and first frame", open, OPEN_BUDGET));
    let wanted = |scenario: &&Scenario| {
        only.iter()
            .all(|wanted| scenario.name.contains(wanted.as_str()))
    };
    for scenario in SCENARIOS.iter().filter(wanted) {
        results.push(scenario.measure(&mut app, runs));
    }
    if !timed {
        return ExitCode::SUCCESS;
    }
    println!(
        "{written} items, {} lines, {:.1} MB, {WIDTH}×{HEIGHT}, slowest of up to {runs} runs:",
        text.lines().count(),
        megabytes(text.len())
    );
    let mut failed = false;
    for result in &results {
        let mark = if result.time <= result.budget {
            "ok  "
        } else {
            failed = true;
            "SLOW"
        };
        println!(
            "  {mark} {:<34} {:>9.2} ms  (budget {} ms)",
            result.name,
            result.time.as_secs_f64() * 1000.0,
            result.budget.as_millis()
        );
    }
    if failed {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

#[expect(clippy::cast_precision_loss, reason = "a size shown with one decimal")]
fn megabytes(bytes: usize) -> f64 {
    bytes as f64 / 1_000_000.0
}

/// One measured line of the report.
struct Report {
    name: &'static str,
    time: Duration,
    budget: Duration,
}

impl Report {
    fn new(name: &'static str, time: Duration, budget: Duration) -> Self {
        Self { name, time, budget }
    }
}

/// A keystroke scenario: it measures one keystroke (or a short sequence a user types as one,
/// such as `gg`) and leaves the app ready to run again.
struct Scenario {
    name: &'static str,
    run: fn(&mut App) -> Duration,
}

impl Scenario {
    /// Runs the scenario on `app` up to `runs` times and gives the slowest run.
    fn measure(&self, app: &mut App, runs: usize) -> Report {
        let mut took = Vec::new();
        while took.len() < runs && took.iter().sum::<Duration>() < SCENARIO_TIME {
            took.push((self.run)(app));
        }
        let slowest = took.iter().max().copied().unwrap_or_default();
        Report::new(self.name, slowest, KEY_BUDGET)
    }
}

const NONE: KeyModifiers = KeyModifiers::NONE;
const CTRL: KeyModifiers = KeyModifiers::CONTROL;
const ALT: KeyModifiers = KeyModifiers::ALT;

const SCENARIOS: &[Scenario] = &[
    Scenario {
        name: "j (next item)",
        run: |app| app.keys("j"),
    },
    Scenario {
        name: "k (previous item)",
        run: |app| app.keys("k"),
    },
    Scenario {
        name: "G then gg (last, first)",
        run: |app| app.keys("G").max(app.keys("gg")),
    },
    Scenario {
        name: "gj then gk (sections)",
        run: |app| app.keys("gj").max(app.keys("gk")),
    },
    Scenario {
        name: ". then > (into, up)",
        run: |app| {
            app.keys("j");
            app.keys(".").max(app.keys(">"))
        },
    },
    Scenario {
        name: "x (complete)",
        run: |app| app.keys("x").max(app.keys("x")),
    },
    Scenario {
        name: "Ctrl+O (show completed)",
        run: |app| {
            app.key(KeyCode::Char('o'), CTRL)
                .max(app.key(KeyCode::Char('o'), CTRL))
        },
    },
    Scenario {
        name: "Enter, type, Esc (edit)",
        run: |app| {
            let enter = app.key(KeyCode::Enter, NONE);
            let typed = app.keys("a");
            let erased = app.key(KeyCode::Backspace, NONE);
            let left = app.key(KeyCode::Esc, NONE);
            enter.max(typed).max(erased).max(left)
        },
    },
    Scenario {
        name: "o, type, Esc (new task)",
        run: |app| {
            let new = app.keys("o");
            let typed = app.keys("b");
            let left = app.key(KeyCode::Esc, NONE);
            new.max(typed).max(left)
        },
    },
    Scenario {
        name: "Alt+j, Alt+k (move)",
        run: |app| {
            app.key(KeyCode::Char('j'), ALT)
                .max(app.key(KeyCode::Char('k'), ALT))
        },
    },
    Scenario {
        name: "] then [ (indent)",
        run: |app| {
            app.keys("j");
            app.keys("]").max(app.keys("["))
        },
    },
    Scenario {
        name: "z (undo)",
        run: |app| {
            app.keys("x");
            app.keys("z")
        },
    },
    Scenario {
        name: "gt then H (Today)",
        run: |app| app.view("gt"),
    },
    Scenario {
        name: "gU then H (Upcoming)",
        run: |app| app.view("gU"),
    },
    Scenario {
        name: "ga then H (All)",
        run: |app| app.view("ga"),
    },
    Scenario {
        name: "gL then H (Logbook)",
        run: |app| app.view("gL"),
    },
    Scenario {
        name: "gd then H (Trash)",
        run: |app| app.view("gd"),
    },
    Scenario {
        name: "gi then H (Inbox)",
        run: |app| app.view("gi"),
    },
    Scenario {
        name: "x, z in Today",
        run: |app| {
            app.keys("gt");
            let done = app.keys("x").max(app.keys("z"));
            app.keys("H");
            done
        },
    },
    Scenario {
        name: "Ctrl+B, x, z, Ctrl+B (sidebar)",
        run: |app| {
            let open = app.key(KeyCode::Char('b'), CTRL);
            app.key(KeyCode::Char('l'), CTRL);
            let done = app.keys("x").max(app.keys("z"));
            let closed = app.key(KeyCode::Char('b'), CTRL);
            open.max(done).max(closed)
        },
    },
    Scenario {
        name: "Ctrl+G, type, Esc (Navigator)",
        run: |app| app.finder(|app| app.key(KeyCode::Char('g'), CTRL)),
    },
    Scenario {
        name: "Ctrl+P, type, Esc (palette)",
        run: |app| app.finder(|app| app.key(KeyCode::Char('p'), CTRL)),
    },
    Scenario {
        name: "/, type, Esc (Go to Symbol)",
        run: |app| app.finder(|app| app.keys("/")),
    },
    Scenario {
        name: "gm, type, Esc (Move to)",
        run: |app| app.finder(|app| app.keys("gm")),
    },
    Scenario {
        name: "mg, type, Esc (tags)",
        run: |app| app.finder(|app| app.keys("mg")),
    },
    Scenario {
        name: "Ctrl+F, type a word (search)",
        run: |app| {
            app.keys("x");
            let open = app.key(KeyCode::Char('f'), CTRL);
            let typed = app.typed("wor");
            let erased = app.key(KeyCode::Backspace, NONE);
            app.key(KeyCode::Esc, NONE);
            app.keys("H");
            open.max(typed).max(erased)
        },
    },
    Scenario {
        name: "Ctrl+F, two words, j, k (search)",
        run: |app| {
            app.keys("x");
            app.key(KeyCode::Char('f'), CTRL);
            let typed = app.typed("number 4");
            let done = app.key(KeyCode::Enter, NONE);
            let moved = app.keys("j").max(app.keys("k"));
            app.keys("H");
            typed.max(done).max(moved)
        },
    },
    Scenario {
        name: "n, Esc (notes panel)",
        run: |app| {
            let open = app.keys("n");
            let closed = app.key(KeyCode::Esc, NONE);
            open.max(closed)
        },
    },
    Scenario {
        name: "x, z beside three view panes",
        run: |app| {
            app.view_panes(true);
            let done = app.keys("x").max(app.keys("z"));
            app.view_panes(false);
            done
        },
    },
    Scenario {
        name: "edit beside three view panes",
        run: |app| {
            app.view_panes(true);
            let enter = app.key(KeyCode::Enter, NONE);
            let typed = app.keys("a");
            let erased = app.key(KeyCode::Backspace, NONE);
            let left = app.key(KeyCode::Esc, NONE);
            app.view_panes(false);
            enter.max(typed).max(erased).max(left)
        },
    },
    Scenario {
        name: "autosave",
        run: |app| {
            app.keys("x");
            app.later(Duration::from_secs(2))
        },
    },
];

/// The app on a file, drawn on a test terminal; disk work runs on the disk worker as in `tipp`.
struct App {
    tui: Tui,
    terminal: Terminal<TestBackend>,
    inputs: Receiver<Input>,
    now: Duration,
}

impl App {
    /// Opens `path` as `tipp` does, with its own files kept in `dir`, and draws the first frame;
    /// returns how long that took.
    #[expect(clippy::disallowed_methods, reason = "a benchmark measures wall time")]
    fn open(path: &Path, dir: &Path) -> (Self, Duration) {
        let (sender, inputs) = mpsc::channel();
        let edge = Edge {
            inputs: sender,
            backups: Some(dir.join("backups")),
            recovery: Some(dir.join("recovery")),
            layouts: Some(dir.join("layouts.json")),
            config: Some(dir.join("config.toml")),
            home: None,
            clock: || NOW,
            animations: Animations::Full,
        };
        let started = Instant::now();
        let tui = Tui::open(Some(path.to_owned()), None, TODAY, &edge).expect("the file opens");
        let mut app = Self {
            tui,
            terminal: Terminal::new(TestBackend::new(WIDTH, HEIGHT)).expect("a test terminal"),
            inputs,
            now: Duration::ZERO,
        };
        app.draw();
        let open = started.elapsed();
        (app, open)
    }

    /// Types `keys` and draws the frame after the last one; returns how long that took.
    fn keys(&mut self, keys: &str) -> Duration {
        self.timed(|tui| {
            for c in keys.chars() {
                tui.handle(&Event::Key(KeyEvent::new(KeyCode::Char(c), NONE)), TODAY);
            }
        })
    }

    /// Types `text` one key at a time, drawing the frame after each; returns the slowest.
    fn typed(&mut self, text: &str) -> Duration {
        text.chars()
            .map(|c| self.keys(&c.to_string()))
            .max()
            .unwrap_or_default()
    }

    /// Presses `code` with `modifiers` and draws the frame; returns how long that took.
    fn key(&mut self, code: KeyCode, modifiers: KeyModifiers) -> Duration {
        self.timed(|tui| tui.handle(&Event::Key(KeyEvent::new(code, modifiers)), TODAY))
    }

    /// Opens a view with `keys` after a change, as it usually follows one, and goes back; returns
    /// the slower of the two.
    fn view(&mut self, keys: &str) -> Duration {
        self.keys("x");
        let shown = self.keys(keys);
        self.keys("x");
        shown.max(self.keys("H"))
    }

    /// Opens a picker with `open` after a change, types into it, erases that and closes it;
    /// returns the slowest of these.
    fn finder(&mut self, open: fn(&mut Self) -> Duration) -> Duration {
        self.keys("x");
        let opened = open(self);
        let typed = self.keys("t");
        let erased = self.key(KeyCode::Backspace, NONE);
        let closed = self.key(KeyCode::Esc, NONE);
        opened.max(typed).max(erased).max(closed)
    }

    /// Opens Today, Upcoming and All in three more panes and goes back to the first, or with
    /// `false` closes them, untimed.
    fn view_panes(&mut self, open: bool) {
        for (pane, view) in [("g4", "ga"), ("g3", "gU"), ("g2", "gt")] {
            if open {
                self.key(KeyCode::Char('w'), CTRL);
                self.keys("v");
                self.keys(view);
            } else {
                self.keys(pane);
                self.key(KeyCode::Char('w'), CTRL);
                self.keys("c");
            }
        }
        self.keys("g1");
    }

    /// Lets `wait` pass, as the event loop does when a deadline such as an autosave passes, and
    /// draws the frame; returns how long that took.
    fn later(&mut self, wait: Duration) -> Duration {
        self.now += wait;
        let now = self.now;
        self.timed(|tui| tui.tick(now, TODAY))
    }

    /// Runs `input` and draws, timing both, as the event loop does for one input. Reports of disk
    /// work that arrived meanwhile are handed over first, untimed.
    #[expect(clippy::disallowed_methods, reason = "a benchmark measures wall time")]
    fn timed(&mut self, input: impl FnOnce(&mut Tui)) -> Duration {
        self.settle();
        self.now += Duration::from_millis(1);
        let now = self.now;
        self.tui.tick(now, TODAY);
        let started = Instant::now();
        input(&mut self.tui);
        self.draw();
        started.elapsed()
    }

    /// Hands the reports of finished disk work to the app.
    fn settle(&mut self) {
        self.tui.flush();
        for input in self.inputs.try_iter().collect::<Vec<_>>() {
            self.tui.input(input, TODAY);
        }
    }

    /// Draws the app and places the cursor, as `tipp` does.
    fn draw(&mut self) {
        let tui = &mut self.tui;
        self.terminal
            .draw(|frame| {
                let area = frame.area();
                frame.render_widget(&mut *tui, area);
                if let Some(cursor) = tui.cursor(area) {
                    frame.set_cursor_position(cursor);
                }
            })
            .expect("drawing to a test backend");
    }
}

/// A made-up file of at least `items` items and the number it holds, the shape of a large
/// personal one: an Inbox, top-level projects of nested projects and sections, tasks with
/// descriptions, checklists, notes, comments, dates, tags, completed and deleted tasks. The same
/// for every run.
fn file(items: usize) -> (String, usize) {
    let mut random = Random(0x5eed);
    let mut text = String::with_capacity(items * 80);
    let mut written = 0;
    text.push_str("Inbox:\n");
    written += 1;
    for _ in 0..40 {
        written += task(&mut text, &mut random, written, 1);
    }
    let mut project = 0;
    while written < items {
        project += 1;
        let _ = writeln!(
            text,
            "Project {project} of the large file with a longer name:"
        );
        let _ = writeln!(
            text,
            "\tWhat project {project} is for, in a sentence of its own."
        );
        written += 1;
        for sub in 0..random.below(4) {
            let _ = writeln!(text, "\tSubproject {project}.{sub} with a name:");
            written += 1;
            written += tasks(&mut text, &mut random, written, 2);
            if random.below(2) == 0 {
                let _ = writeln!(text, "\t\tStage {sub}: @section");
                written += 1;
                written += tasks(&mut text, &mut random, written, 3);
            }
        }
        written += tasks(&mut text, &mut random, written, 1);
        for section in 0..random.below(4) {
            let _ = writeln!(text, "\tPart {section} of project {project}: @section");
            written += 1;
            written += tasks(&mut text, &mut random, written, 2);
        }
    }
    (text, written)
}

/// Writes up to 25 tasks at `depth` and gives how many items they hold.
fn tasks(text: &mut String, random: &mut Random, n: usize, depth: usize) -> usize {
    let mut written = 0;
    for _ in 0..random.below(25) {
        written += task(text, random, n + written, depth);
    }
    written
}

/// Writes one task at `depth`, numbered `n`, with some of what a task has, and gives how many
/// items it holds with its checklist.
fn task(text: &mut String, random: &mut Random, n: usize, depth: usize) -> usize {
    let indent = "\t".repeat(depth);
    let day = random.below(28) + 1;
    let tags = match random.below(12) {
        0 => format!(" @done(2026-09-{day:02})"),
        1 => format!(" @due(2026-10-{day:02})"),
        2 => format!(" @start(2026-10-{day:02}) @due(2026-11-{day:02})"),
        3 => format!(" @start(2026-10-{:02})", day.min(5)),
        4 => format!(" @deleted(2026-09-{day:02})"),
        5 => " @errands".to_owned(),
        6 => " @work @phone".to_owned(),
        _ => String::new(),
    };
    let _ = writeln!(
        text,
        "{indent}- Do the thing number {n} that needs doing this week, before the review{tags}"
    );
    let mut written = 1;
    match random.below(10) {
        0 => {
            let _ = writeln!(
                text,
                "{indent}\tA description of task {n} that says what it is for and why it matters."
            );
        }
        1 => {
            for step in 0..=random.below(6) {
                let done = if step % 2 == 0 {
                    " @done(2026-10-01)"
                } else {
                    ""
                };
                let _ = writeln!(text, "{indent}\t- Step {step} of task {n}{done}");
                written += 1;
            }
        }
        2 => {
            let _ = writeln!(
                text,
                "{indent}\tNOTE @hidden\n{indent}\t\t# Notes on task {n}\n{indent}\t\tA longer note with **bold** words and a link https://example.com/{n}"
            );
        }
        3 => {
            let _ = writeln!(
                text,
                "{indent}\tCOMMENTS @hidden\n{indent}\t\t[2026-10-01 16:05] A comment on task {n} from the review."
            );
        }
        _ => {}
    }
    written
}

/// A small linear congruential generator, so the file is the same on every run without a
/// dependency.
struct Random(u64);

impl Random {
    fn below(&mut self, n: usize) -> usize {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        usize::try_from(self.0 >> 33).expect("31 bits fit a usize") % n
    }
}
