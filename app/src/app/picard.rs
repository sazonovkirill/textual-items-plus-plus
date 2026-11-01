//! A hidden touch of the Star Trek TNG theme: now and then, adding or completing something has
//! Captain Picard say a word on the status line (D-162). Chance comes from the frontend's clock,
//! so the same clock always gives the same quote.

use std::time::Duration;

use super::{BuiltIn, Notice, Random, State, autosave::SaveState};

/// One in how many occasions gets a quote.
const ODDS: usize = 5;

/// The least time from one quote to the next.
const QUIET: Duration = Duration::from_mins(1);

/// How many items completed at once count as a lot.
const LIGHTS: usize = 4;

/// What Captain Picard says.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Quote {
    /// "Make it so."
    MakeItSo,
    /// "Engage."
    Engage,
    /// "Tea. Earl Grey. Hot."
    Tea,
    /// "There are four lights!"
    FourLights,
}

impl Quote {
    /// The words, as he says them.
    #[must_use]
    pub const fn words(self) -> &'static str {
        match self {
            Self::MakeItSo => "Make it so.",
            Self::Engage => "Engage.",
            Self::Tea => "Tea. Earl Grey. Hot.",
            Self::FourLights => "There are four lights!",
        }
    }
}

/// What a message did that a quote may follow.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Occasion {
    /// A task, project, section or area was added.
    Added,
    /// Quick Add added a task.
    QuickAdded,
    /// These many tasks and projects were completed.
    Completed(usize),
}

impl Occasion {
    /// The quotes that fit it.
    fn quotes(self) -> &'static [Quote] {
        match self {
            Self::QuickAdded => &[Quote::Tea],
            Self::Completed(count) if count >= LIGHTS => &[Quote::FourLights],
            Self::Added | Self::Completed(_) => &[Quote::MakeItSo, Quote::Engage],
        }
    }
}

/// The occasion the message being handled made, the quote the status line shows and the one
/// before it, and when it was said.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct Picard {
    occasion: Option<Occasion>,
    shown: Option<Quote>,
    last: Option<(Quote, Duration)>,
}

impl Picard {
    /// Notes that the message being handled made `occasion`.
    pub(super) fn witness(&mut self, occasion: Occasion) {
        self.occasion = Some(occasion);
    }

    /// Takes back the quote shown, as each command clears the status line.
    pub(super) fn hush(&mut self) {
        self.shown = None;
    }

    /// The quote the status line shows.
    pub(super) fn shown(&self) -> Option<Quote> {
        self.shown
    }

    /// Ends the message handled at `now`, by the frontend's clock: when it made an occasion and
    /// the status line is `free` for a quote, one in [`ODDS`] times a quote that fits it shows,
    /// never the one said last and never sooner than [`QUIET`] after it.
    pub(super) fn speak(&mut self, free: bool, now: Duration) {
        let Some(occasion) = self.occasion.take() else {
            return;
        };
        let rested = self
            .last
            .is_none_or(|(_, said)| now.saturating_sub(said) >= QUIET);
        if !free || !rested {
            return;
        }
        let mut random = Random::new(u64::try_from(now.as_nanos()).unwrap_or_default());
        if random.below(ODDS) != 0 {
            return;
        }
        let fresh: Vec<Quote> = occasion
            .quotes()
            .iter()
            .copied()
            .filter(|&quote| self.last.is_none_or(|(last, _)| last != quote))
            .collect();
        if let Some(&quote) = fresh.get(random.below(fresh.len())) {
            self.shown = Some(quote);
            self.last = Some((quote, now));
        }
    }
}

impl State {
    /// The quote of Captain Picard the status line shows with the Star Trek TNG theme while it
    /// says nothing else that matters, as Quick Add's confirmation does not (D-162).
    #[must_use]
    pub fn quote(&self) -> Option<Quote> {
        self.picard.shown().filter(|_| self.quiet())
    }

    /// Ends the message just handled, at the frontend's clock: a quote may follow what it did,
    /// with the Star Trek TNG theme while the status line is quiet (D-162).
    pub(super) fn salute(&mut self) {
        let free = self.settings.theme == BuiltIn::StarTrekTng && self.quiet();
        self.picard.speak(free, self.now());
    }

    /// Whether the status line says nothing that a quote would crowd: no notice but Quick Add's
    /// confirmation, no failed save, no file gone from disk and no quit to confirm.
    fn quiet(&self) -> bool {
        matches!(self.notice(), None | Some(Notice::QuickAdded(_)))
            && !matches!(self.save_state(), SaveState::Failed { .. })
            && !self.gone_from_disk()
            && !self.confirming_quit()
    }
}

#[cfg(test)]
mod tests {
    use jiff::civil::{Date, date};

    use super::*;
    use crate::{
        app::{
            Command, EditInput, Message, Opened, QuickAddInput, Report, Settings, Side, TextEdit,
            update,
        },
        domain::{Document, Item, ItemKind, Node, PlacementError},
    };

    /// A Wednesday.
    const TODAY: Date = date(2026, 10, 14);

    /// Times by the clock, a second apart, whose roll shows a quote when `hits`, else none.
    fn rolls(hits: bool) -> impl Iterator<Item = Duration> {
        (0..).map(Duration::from_secs).filter(move |&now| {
            let seed = u64::try_from(now.as_nanos()).unwrap_or_default();
            (Random::new(seed).below(ODDS) == 0) == hits
        })
    }

    fn hit_after(after: Duration) -> Duration {
        rolls(true).find(|&now| now >= after).expect("rolls go on")
    }

    fn said(picard: &mut Picard, occasion: Occasion, now: Duration) -> Option<Quote> {
        picard.hush();
        picard.witness(occasion);
        picard.speak(true, now);
        picard.shown()
    }

    // D-162
    #[test]
    fn about_one_occasion_in_five_gets_a_quote() {
        let mut picard = Picard::default();
        let mut quotes = 0;
        for minute in 0..1000 {
            let now = Duration::from_mins(minute) + Duration::from_millis(minute * 7);
            if said(&mut picard, Occasion::Added, now).is_some() {
                quotes += 1;
            }
        }
        assert!((150..250).contains(&quotes), "{quotes}");
    }

    // D-162
    #[test]
    fn the_quote_fits_the_occasion() {
        let now = hit_after(Duration::ZERO);
        let quote = |occasion| said(&mut Picard::default(), occasion, now);
        assert!(matches!(
            quote(Occasion::Added),
            Some(Quote::MakeItSo | Quote::Engage)
        ));
        assert_eq!(quote(Occasion::QuickAdded), Some(Quote::Tea));
        assert!(matches!(
            quote(Occasion::Completed(3)),
            Some(Quote::MakeItSo | Quote::Engage)
        ));
        assert_eq!(quote(Occasion::Completed(4)), Some(Quote::FourLights));
        assert_eq!(quote(Occasion::Completed(9)), Some(Quote::FourLights));
    }

    // D-162
    #[test]
    fn a_roll_that_misses_or_a_busy_status_line_shows_nothing() {
        let miss = rolls(false).next().expect("rolls go on");
        assert_eq!(said(&mut Picard::default(), Occasion::Added, miss), None);
        let mut picard = Picard::default();
        picard.witness(Occasion::Added);
        picard.speak(false, hit_after(Duration::ZERO));
        assert_eq!(picard.shown(), None);
        assert_eq!(picard.last, None, "nothing said, so no wait after it");
    }

    // D-162
    #[test]
    fn a_message_without_an_occasion_says_nothing_and_an_occasion_lasts_one_message() {
        let mut picard = Picard::default();
        let now = hit_after(Duration::ZERO);
        picard.speak(true, now);
        assert_eq!(picard.shown(), None);
        picard.witness(Occasion::Added);
        picard.speak(false, now);
        picard.speak(true, now);
        assert_eq!(picard.shown(), None, "the occasion went with its message");
    }

    // D-162
    #[test]
    fn a_minute_passes_between_quotes() {
        let mut picard = Picard::default();
        let first = hit_after(Duration::ZERO);
        assert!(said(&mut picard, Occasion::QuickAdded, first).is_some());
        let soon = hit_after(first + Duration::from_secs(1));
        assert!(soon < first + QUIET, "a roll that hits inside the minute");
        assert_eq!(said(&mut picard, Occasion::Added, soon), None);
        let later = hit_after(first + QUIET);
        assert!(said(&mut picard, Occasion::Added, later).is_some());
    }

    // D-162
    #[test]
    fn the_same_quote_never_comes_twice_in_a_row() {
        let mut picard = Picard::default();
        let mut now = Duration::ZERO;
        let mut quotes = Vec::new();
        for _ in 0..20 {
            now = hit_after(now + QUIET);
            quotes.extend(said(&mut picard, Occasion::Added, now));
        }
        assert_eq!(quotes.len(), 20);
        assert!(
            quotes.windows(2).all(|pair| pair[0] != pair[1]),
            "{quotes:?}"
        );
        now = hit_after(now + QUIET);
        let mut picard = Picard::default();
        assert_eq!(
            said(&mut picard, Occasion::QuickAdded, now),
            Some(Quote::Tea)
        );
        now = hit_after(now + QUIET);
        assert_eq!(
            said(&mut picard, Occasion::QuickAdded, now),
            None,
            "the only quote that fits was said last"
        );
    }

    // D-162
    #[test]
    fn hushing_takes_the_quote_off_the_status_line() {
        let mut picard = Picard::default();
        picard.witness(Occasion::Added);
        picard.speak(true, hit_after(Duration::ZERO));
        assert!(picard.shown().is_some());
        picard.hush();
        assert_eq!(picard.shown(), None);
    }

    // D-162
    #[test]
    fn every_quote_has_its_words() {
        assert_eq!(
            [
                Quote::MakeItSo,
                Quote::Engage,
                Quote::Tea,
                Quote::FourLights
            ]
            .map(Quote::words),
            [
                "Make it so.",
                "Engage.",
                "Tea. Earl Grey. Hot.",
                "There are four lights!"
            ]
        );
    }

    /// The app in `theme` on a document whose Inbox holds `tasks` tasks, at a time by the clock
    /// whose roll shows a quote.
    fn bridge(theme: BuiltIn, tasks: usize) -> Result<State, PlacementError> {
        let mut document = Document::new();
        let inbox = Some(document.inbox());
        for task in 0..tasks {
            let task = Node::Item(Item::new(ItemKind::Task, format!("Task {task}")));
            document.push(inbox, task)?;
        }
        let settings = Settings {
            theme,
            ..Settings::default()
        };
        let mut state =
            State::new(document, Opened::Demo, TODAY).with_settings(Settings::default(), settings);
        message(&mut state, Message::Tick(hit_after(Duration::ZERO)));
        Ok(state)
    }

    fn message(state: &mut State, message: Message) {
        assert_eq!(update(state, message, TODAY), []);
    }

    fn command(state: &mut State, command: Command) {
        message(state, Message::Command(command));
    }

    /// Adds a task titled `title` below the focus.
    fn add(state: &mut State, title: &str) {
        command(state, Command::New(Side::Below));
        for c in title.chars() {
            message(state, Message::Edit(EditInput::Text(TextEdit::Insert(c))));
        }
        message(state, Message::Edit(EditInput::Done));
    }

    // D-162
    #[test]
    fn adding_with_the_star_trek_theme_brings_a_quote_until_the_next_command()
    -> Result<(), PlacementError> {
        let mut state = bridge(BuiltIn::StarTrekTng, 1)?;
        add(&mut state, "");
        assert_eq!(state.quote(), None, "a new task left empty adds nothing");
        add(&mut state, "Chart a course");
        assert!(matches!(
            state.quote(),
            Some(Quote::MakeItSo | Quote::Engage)
        ));
        command(&mut state, Command::Shortcuts);
        assert_eq!(state.quote(), None);
        let mut state = bridge(BuiltIn::TerminalDark, 1)?;
        add(&mut state, "Chart a course");
        assert_eq!(state.quote(), None, "only with the Star Trek TNG theme");
        Ok(())
    }

    // D-162
    #[test]
    fn completing_brings_a_quote_and_four_at_once_the_four_lights() -> Result<(), PlacementError> {
        let mut state = bridge(BuiltIn::StarTrekTng, 1)?;
        command(&mut state, Command::ToggleCompleted);
        assert!(matches!(
            state.quote(),
            Some(Quote::MakeItSo | Quote::Engage)
        ));
        let mut state = bridge(BuiltIn::StarTrekTng, 4)?;
        command(&mut state, Command::SelectAll);
        command(&mut state, Command::ToggleCompleted);
        assert_eq!(state.quote(), Some(Quote::FourLights));
        message(&mut state, Message::Tick(hit_after(QUIET * 2)));
        command(&mut state, Command::ToggleCompleted);
        assert_eq!(state.quote(), None, "uncompleting is no occasion");
        Ok(())
    }

    // D-162
    #[test]
    fn quick_add_brings_tea_beside_where_the_task_went() -> Result<(), PlacementError> {
        let mut state = bridge(BuiltIn::StarTrekTng, 0)?;
        command(&mut state, Command::QuickAdd);
        for c in "Replicator".chars() {
            message(
                &mut state,
                Message::QuickAdd(QuickAddInput::Edit(TextEdit::Insert(c))),
            );
        }
        message(&mut state, Message::QuickAdd(QuickAddInput::Add));
        assert_eq!(
            state.notice(),
            Some(&Notice::QuickAdded("Inbox".to_owned()))
        );
        assert_eq!(state.quote(), Some(Quote::Tea));
        Ok(())
    }

    // D-162
    #[test]
    fn a_status_line_saying_something_that_matters_shows_no_quote() -> Result<(), PlacementError> {
        let mut state = bridge(BuiltIn::StarTrekTng, 1)?;
        add(&mut state, "Chart a course");
        assert!(state.quote().is_some());
        message(
            &mut state,
            Message::Disk(Report::BackupFailed("full".into())),
        );
        assert_eq!(state.quote(), None);
        Ok(())
    }
}
