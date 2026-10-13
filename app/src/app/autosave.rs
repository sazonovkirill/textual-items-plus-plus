//! When the open file is saved: a moment after the last change, one save at a time, again after
//! a failure only once something changes, and before the session ends (F-FILE-05, FR-STORE-09).
//! The policy only decides; the frontend writes the file and reports how it went.
//!
//! The document is known by its revision, a number that changes with every change, and time by
//! the frontend's clock, both passed in.

use std::time::Duration;

/// How long after the last change a save starts.
pub const DELAY: Duration = Duration::from_secs(1);

/// How the session is asked to end.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ending {
    /// The user quits: save first, and ask before ending if that fails.
    Quit,
    /// The process was told to end, as by `SIGTERM` or `SIGHUP`: save first, and end even if that
    /// fails.
    Terminate,
}

/// What the frontend does next for saving.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    /// Nothing until the next change, report or [`Autosave::deadline`].
    Wait,
    /// Save the document as it is now, at this revision, and report how it went.
    Save(u64),
    /// End the session: the file holds every change, or the session ends without them.
    End,
}

/// What the status line says about saving.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SaveState<'a> {
    /// The file on disk holds every change.
    Saved,
    /// Changes are waiting for their save or being written.
    Unsaved,
    /// The last save failed, for this reason; the next change and ending try again.
    Failed {
        /// What the frontend reported.
        reason: &'a str,
    },
}

/// How the session ends, from the gentlest to the most final.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Close {
    Quit,
    Terminate,
    /// End without saving.
    Discard,
}

/// A save that failed.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Failure {
    revision: u64,
    reason: String,
}

/// The saving policy of one open file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Autosave {
    /// Whether there is a file to save to; a read-only file or a document without a file is
    /// never saved.
    saveable: bool,
    /// The revision the file holds, or `None` when it holds none of them.
    saved: Option<u64>,
    /// The latest revision seen, and when it was first seen.
    seen: u64,
    changed_at: Duration,
    /// The revision being written; at most one save runs at a time.
    writing: Option<u64>,
    failure: Option<Failure>,
    close: Option<Close>,
    /// Whether a quit's save failed, so quitting again ends without saving.
    confirm_quit: bool,
}

impl Autosave {
    /// The policy for a document at `revision` that the file holds. A file that is not
    /// `saveable` is never saved.
    #[must_use]
    pub fn new(revision: u64, saveable: bool) -> Self {
        Self {
            saved: Some(revision),
            ..Self::unsaved(revision, saveable)
        }
    }

    /// The policy for a document at `revision` that the file does not hold, as one recovered
    /// after a crash: it is due for a save [`DELAY`] after time zero.
    #[must_use]
    pub fn unsaved(revision: u64, saveable: bool) -> Self {
        Self {
            saveable,
            saved: None,
            seen: revision,
            changed_at: Duration::ZERO,
            writing: None,
            failure: None,
            close: None,
            confirm_quit: false,
        }
    }

    /// What to do with the document at `revision` at time `now`, called after every message. A
    /// new revision restarts the delay and withdraws the quit question. Ending the session saves
    /// at once and retries a failed revision once; otherwise a failed revision waits for the next
    /// change. A save that starts is under way until it is reported.
    pub fn step(&mut self, revision: u64, now: Duration) -> Step {
        if revision != self.seen {
            self.seen = revision;
            self.changed_at = now;
            self.confirm_quit = false;
        }
        let due = self.saveable
            && self.writing.is_none()
            && Some(revision) != self.saved
            && match self.close {
                Some(Close::Quit | Close::Terminate) => true,
                Some(Close::Discard) => false,
                None => !self.failed_at(revision) && now >= self.changed_at.saturating_add(DELAY),
            };
        if due {
            self.writing = Some(revision);
            return Step::Save(revision);
        }
        let finished = match self.close {
            None => false,
            Some(Close::Discard) => true,
            // A save under way is of a revision newer than the saved one, so this waits for it.
            Some(Close::Quit | Close::Terminate) => !self.saveable || Some(revision) == self.saved,
        };
        if finished { Step::End } else { Step::Wait }
    }

    fn failed_at(&self, revision: u64) -> bool {
        self.failure
            .as_ref()
            .is_some_and(|failure| failure.revision == revision)
    }

    /// When a save is next due without anything else happening, if one is, so the frontend can
    /// wake then.
    #[must_use]
    pub fn deadline(&self) -> Option<Duration> {
        (self.saveable
            && self.close.is_none()
            && self.writing.is_none()
            && Some(self.seen) != self.saved
            && !self.failed_at(self.seen))
        .then(|| self.changed_at.saturating_add(DELAY))
    }

    /// Asks to end the session. A quit after a quit's failed save ends without saving, and a
    /// more final ending is never softened.
    pub fn end(&mut self, ending: Ending) {
        let close = match ending {
            Ending::Quit if self.confirm_quit => Close::Discard,
            Ending::Quit => Close::Quit,
            Ending::Terminate => Close::Terminate,
        };
        self.close = self.close.max(Some(close));
    }

    /// The save under way succeeded. A report without a save under way changes nothing.
    pub fn saved(&mut self) {
        if let Some(revision) = self.writing.take() {
            self.saved = Some(revision);
            self.failure = None;
            self.confirm_quit = false;
        }
    }

    /// The save under way failed for `reason`. When it was of the latest revision, a quit
    /// waiting for it is withdrawn and asks instead, and a termination ends anyway. A report without a save under way changes nothing.
    pub fn failed(&mut self, reason: String) {
        let Some(revision) = self.writing.take() else {
            return;
        };
        self.failure = Some(Failure { revision, reason });
        // A change made while the save ran has not been tried yet, so ending saves it next.
        if revision != self.seen {
            return;
        }
        match self.close {
            Some(Close::Quit) => {
                self.close = None;
                self.confirm_quit = true;
            }
            Some(Close::Terminate) => self.close = Some(Close::Discard),
            Some(Close::Discard) | None => {}
        }
    }

    /// The save under way was not done because another program changed the file; it is not a
    /// failure, and what follows is up to whoever handles the change.
    pub fn refused(&mut self) {
        self.writing = None;
    }

    /// Whether the file holds every change of the document at `revision`, and whether the last
    /// save failed.
    #[must_use]
    pub fn state(&self, revision: u64) -> SaveState<'_> {
        match &self.failure {
            Some(failure) => SaveState::Failed {
                reason: &failure.reason,
            },
            None if Some(revision) != self.saved => SaveState::Unsaved,
            None => SaveState::Saved,
        }
    }

    /// Whether quitting was refused because its save failed, so quitting again ends without
    /// saving, until the next change or a successful save.
    #[must_use]
    pub fn confirming_quit(&self) -> bool {
        self.confirm_quit
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ms(millis: u64) -> Duration {
        Duration::from_millis(millis)
    }

    /// The policy of a file opened at revision 0 that can be saved.
    fn opened() -> Autosave {
        Autosave::new(0, true)
    }

    // F-FILE-05
    #[test]
    fn saves_a_second_after_the_last_change() {
        let mut autosave = opened();
        assert_eq!(autosave.state(0), SaveState::Saved);
        assert_eq!(autosave.deadline(), None);
        assert_eq!(autosave.step(0, ms(0)), Step::Wait);
        assert_eq!(autosave.step(1, ms(100)), Step::Wait);
        assert_eq!(autosave.state(1), SaveState::Unsaved);
        assert_eq!(autosave.deadline(), Some(ms(1100)));
        assert_eq!(
            autosave.step(2, ms(600)),
            Step::Wait,
            "a change restarts the delay"
        );
        assert_eq!(autosave.deadline(), Some(ms(1600)));
        assert_eq!(autosave.step(2, ms(1599)), Step::Wait);
        assert_eq!(autosave.step(2, ms(1600)), Step::Save(2));
        assert_eq!(autosave.state(2), SaveState::Unsaved);
        assert_eq!(
            autosave.deadline(),
            None,
            "nothing is due while the save runs"
        );
        autosave.saved();
        assert_eq!(autosave.state(2), SaveState::Saved);
        assert_eq!(autosave.step(2, ms(5000)), Step::Wait);
    }

    // F-FILE-05
    #[test]
    fn runs_one_save_at_a_time_and_saves_what_changed_meanwhile() {
        let mut autosave = opened();
        autosave.step(1, ms(0));
        assert_eq!(autosave.step(1, ms(1000)), Step::Save(1));
        autosave.step(2, ms(1200));
        assert_eq!(
            autosave.step(2, ms(2200)),
            Step::Wait,
            "the first save is still running"
        );
        autosave.saved();
        assert_eq!(autosave.state(2), SaveState::Unsaved);
        assert_eq!(autosave.step(2, ms(2200)), Step::Save(2));
        autosave.saved();
        assert_eq!(autosave.state(2), SaveState::Saved);
    }

    #[test]
    fn a_change_that_brings_back_the_saved_revision_saves_nothing() {
        let mut autosave = opened();
        autosave.step(1, ms(0));
        autosave.step(0, ms(100));
        assert_eq!(autosave.state(0), SaveState::Saved);
        assert_eq!(autosave.deadline(), None);
        assert_eq!(autosave.step(0, ms(5000)), Step::Wait);
    }

    // F-FILE-05
    #[test]
    fn a_failed_save_waits_for_the_next_change() {
        let mut autosave = opened();
        autosave.step(1, ms(0));
        autosave.step(1, ms(1000));
        autosave.failed("disk full".to_owned());
        assert_eq!(
            autosave.state(1),
            SaveState::Failed {
                reason: "disk full"
            }
        );
        assert_eq!(autosave.deadline(), None);
        assert_eq!(autosave.step(1, ms(9000)), Step::Wait);
        autosave.step(2, ms(10_000));
        assert_eq!(autosave.deadline(), Some(ms(11_000)));
        assert_eq!(autosave.step(2, ms(11_000)), Step::Save(2));
        assert!(matches!(autosave.state(2), SaveState::Failed { .. }));
        autosave.saved();
        assert_eq!(autosave.state(2), SaveState::Saved);
    }

    #[test]
    fn reports_without_a_save_under_way_change_nothing() {
        let mut autosave = opened();
        autosave.step(1, ms(0));
        let before = autosave.clone();
        autosave.saved();
        autosave.failed("stray".to_owned());
        autosave.refused();
        assert_eq!(autosave, before);
        assert_eq!(autosave.step(1, ms(1000)), Step::Save(1));
    }

    #[test]
    fn a_failed_save_of_an_older_revision_lets_ending_save_the_latest() {
        for ending in [Ending::Quit, Ending::Terminate] {
            let mut autosave = opened();
            autosave.step(1, ms(0));
            assert_eq!(autosave.step(1, ms(1000)), Step::Save(1));
            autosave.step(2, ms(1100));
            autosave.end(ending);
            assert_eq!(autosave.step(2, ms(1100)), Step::Wait);
            autosave.failed("disk full".to_owned());
            assert!(!autosave.confirming_quit(), "{ending:?}");
            assert_eq!(autosave.step(2, ms(1100)), Step::Save(2), "{ending:?}");
            autosave.saved();
            assert_eq!(autosave.step(2, ms(1100)), Step::End, "{ending:?}");
        }
    }

    #[test]
    fn a_refused_save_is_no_failure_and_is_due_again_at_once() {
        let mut autosave = opened();
        autosave.step(1, ms(0));
        autosave.step(1, ms(1000));
        autosave.refused();
        assert_eq!(autosave.state(1), SaveState::Unsaved);
        assert_eq!(autosave.deadline(), Some(ms(1000)));
        assert_eq!(autosave.step(1, ms(1000)), Step::Save(1));
    }

    // F-FILE-05
    #[test]
    fn quitting_saves_first_and_then_ends() {
        let mut autosave = opened();
        autosave.end(Ending::Quit);
        assert_eq!(autosave.step(0, ms(0)), Step::End);
        let mut autosave = opened();
        autosave.step(1, ms(0));
        autosave.end(Ending::Quit);
        assert_eq!(autosave.step(1, ms(0)), Step::Save(1), "without the delay");
        assert_eq!(autosave.deadline(), None);
        assert_eq!(autosave.step(1, ms(0)), Step::Wait, "the save is running");
        autosave.saved();
        assert_eq!(autosave.step(1, ms(0)), Step::End);
    }

    #[test]
    fn quitting_while_an_autosave_runs_waits_for_it() {
        let mut autosave = opened();
        autosave.step(1, ms(0));
        autosave.step(1, ms(1000));
        autosave.end(Ending::Quit);
        assert_eq!(autosave.step(1, ms(1000)), Step::Wait);
        autosave.saved();
        assert_eq!(autosave.step(1, ms(1000)), Step::End);
    }

    // F-FILE-05
    #[test]
    fn a_failed_save_on_quit_asks_before_quitting_without_saving() {
        let mut autosave = opened();
        autosave.step(1, ms(0));
        autosave.step(1, ms(1000));
        autosave.failed("disk full".to_owned());
        autosave.end(Ending::Quit);
        assert_eq!(
            autosave.step(1, ms(1000)),
            Step::Save(1),
            "quitting retries the failed save"
        );
        autosave.failed("disk full".to_owned());
        assert!(autosave.confirming_quit());
        assert_eq!(autosave.step(1, ms(5000)), Step::Wait);
        autosave.end(Ending::Quit);
        assert_eq!(autosave.step(1, ms(5000)), Step::End);
    }

    #[test]
    fn a_change_or_a_save_withdraws_the_quit_question() {
        let mut autosave = opened();
        autosave.step(1, ms(0));
        autosave.end(Ending::Quit);
        autosave.step(1, ms(0));
        autosave.failed("disk full".to_owned());
        autosave.step(2, ms(100));
        assert!(!autosave.confirming_quit());
        autosave.end(Ending::Quit);
        assert_eq!(autosave.step(2, ms(100)), Step::Save(2));
        autosave.failed("disk full".to_owned());
        assert!(autosave.confirming_quit());
        autosave.step(3, ms(200));
        assert_eq!(autosave.step(3, ms(1200)), Step::Save(3));
        autosave.saved();
        assert!(!autosave.confirming_quit());
        autosave.end(Ending::Quit);
        assert_eq!(autosave.step(3, ms(1200)), Step::End);
    }

    #[test]
    fn a_save_withdraws_the_quit_question_too() {
        let mut autosave = opened();
        autosave.step(1, ms(0));
        autosave.end(Ending::Quit);
        autosave.step(1, ms(0));
        autosave.failed("disk full".to_owned());
        assert!(autosave.confirming_quit());
        autosave.step(2, ms(100));
        autosave.step(1, ms(200));
        assert_eq!(autosave.step(1, ms(1200)), Step::Wait, "revision 1 failed");
        autosave.step(2, ms(1300));
        assert_eq!(autosave.step(2, ms(2300)), Step::Save(2));
        autosave.saved();
        assert!(!autosave.confirming_quit());
    }

    // F-FILE-05
    #[test]
    fn terminating_saves_and_ends_even_when_the_save_fails() {
        let mut autosave = opened();
        autosave.end(Ending::Terminate);
        assert_eq!(autosave.step(0, ms(0)), Step::End);
        let mut autosave = opened();
        autosave.step(1, ms(0));
        autosave.end(Ending::Terminate);
        assert_eq!(autosave.step(1, ms(0)), Step::Save(1));
        autosave.saved();
        assert_eq!(autosave.step(1, ms(0)), Step::End);
        let mut autosave = opened();
        autosave.step(1, ms(0));
        autosave.end(Ending::Terminate);
        autosave.step(1, ms(0));
        autosave.end(Ending::Quit);
        assert_eq!(
            autosave.step(1, ms(0)),
            Step::Wait,
            "quitting does not soften the end"
        );
        autosave.failed("disk full".to_owned());
        assert_eq!(autosave.step(1, ms(0)), Step::End);
        autosave.failed("stray".to_owned());
        assert_eq!(autosave.step(1, ms(0)), Step::End);
    }

    #[test]
    fn terminating_after_the_quit_question_saves_once_more() {
        let mut autosave = opened();
        autosave.step(1, ms(0));
        autosave.end(Ending::Quit);
        autosave.step(1, ms(0));
        autosave.failed("disk full".to_owned());
        autosave.end(Ending::Terminate);
        assert_eq!(autosave.step(1, ms(0)), Step::Save(1));
        autosave.failed("disk full".to_owned());
        assert_eq!(autosave.step(1, ms(0)), Step::End);
    }

    #[test]
    fn quitting_again_after_the_quit_question_ends_even_while_terminating() {
        let mut autosave = opened();
        autosave.step(1, ms(0));
        autosave.end(Ending::Quit);
        autosave.step(1, ms(0));
        autosave.failed("disk full".to_owned());
        autosave.end(Ending::Quit);
        autosave.end(Ending::Terminate);
        assert_eq!(
            autosave.step(1, ms(0)),
            Step::End,
            "ending without saving stays"
        );
    }

    #[test]
    fn a_file_that_cannot_be_saved_is_never_saved() {
        let mut autosave = Autosave::new(0, false);
        autosave.step(1, ms(0));
        assert_eq!(autosave.state(1), SaveState::Unsaved);
        assert_eq!(autosave.deadline(), None);
        assert_eq!(autosave.step(1, ms(5000)), Step::Wait);
        autosave.end(Ending::Quit);
        assert_eq!(autosave.step(1, ms(5000)), Step::End);
    }

    // F-FILE-05, N-REL-01
    #[test]
    fn a_document_the_file_does_not_hold_is_saved_like_a_change() {
        let mut autosave = Autosave::unsaved(7, true);
        assert_eq!(autosave.state(7), SaveState::Unsaved);
        assert_eq!(autosave.deadline(), Some(DELAY));
        assert_eq!(autosave.step(7, ms(999)), Step::Wait);
        assert_eq!(autosave.step(7, ms(1000)), Step::Save(7));
        autosave.saved();
        assert_eq!(autosave.state(7), SaveState::Saved);
        let mut autosave = Autosave::unsaved(7, true);
        autosave.end(Ending::Quit);
        assert_eq!(autosave.step(7, ms(0)), Step::Save(7));
        autosave.saved();
        assert_eq!(autosave.step(7, ms(0)), Step::End);
    }
}
