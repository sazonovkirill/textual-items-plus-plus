//! The keys that drive typing the words to search for (FR-SRCH-01), which the pane shows after
//! the view's name, as Things 3 shows its search field above its results.

use crossterm::event::{Event, KeyCode, KeyModifiers};

use super::{
    keymap::{self, Key},
    text_keys,
};
use crate::app::SearchInput;

/// The keys of the words typed besides the text keys: `Enter` and `Esc` leave the keys to the
/// results.
pub(super) const KEYS: &[(Key, SearchInput)] = &[
    ((KeyModifiers::NONE, KeyCode::Enter), SearchInput::Done),
    ((KeyModifiers::NONE, KeyCode::Esc), SearchInput::Done),
];

/// What `event` does while the words are typed: its key in [`KEYS`], or a text key's edit. Only
/// key presses count.
pub(super) fn search_input(event: &Event) -> Option<SearchInput> {
    let key = keymap::key_press(event)?;
    KEYS.iter()
        .find(|(bound, _)| *bound == key)
        .map(|&(_, input)| input)
        .or_else(|| text_keys::text_edit(key).map(SearchInput::Edit))
}

#[cfg(test)]
mod tests {
    use crossterm::event::{KeyEvent, KeyEventKind};

    use super::*;
    use crate::app::TextEdit;

    // FR-SRCH-01
    #[test]
    fn enter_and_escape_leave_the_results_the_keys_and_text_keys_edit() {
        let none = KeyModifiers::NONE;
        let input = |code| search_input(&Event::Key(KeyEvent::new(code, none)));
        assert_eq!(input(KeyCode::Enter), Some(SearchInput::Done));
        assert_eq!(input(KeyCode::Esc), Some(SearchInput::Done));
        assert_eq!(
            input(KeyCode::Char('j')),
            Some(SearchInput::Edit(TextEdit::Insert('j')))
        );
        assert_eq!(input(KeyCode::F(2)), None);
        let mut release = KeyEvent::new(KeyCode::Enter, none);
        release.kind = KeyEventKind::Release;
        assert_eq!(search_input(&Event::Key(release)), None);
    }
}
