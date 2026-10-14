//! The keys of edit mode: `Enter` and `Esc` end it, as in taskpaper-terminal, and the standard
//! text keys edit the title (FR-EDIT-01, FR-EDIT-04,
//! [editing.md §2](../../../specs/reference/taskpaper-terminal/specs/editing.md#2-edit-mode)).

use crossterm::event::{Event, KeyCode, KeyModifiers};

use super::{
    keymap::{self, Key},
    text_keys,
};
use crate::app::EditInput;

/// The keys of edit mode besides the text keys.
const EDIT: &[(Key, EditInput)] = &[
    ((KeyModifiers::NONE, KeyCode::Enter), EditInput::Next),
    ((KeyModifiers::NONE, KeyCode::Esc), EditInput::Done),
];

/// What `event` does in edit mode: its edit-mode key, else the change to the title its text key
/// asks for. Only key presses count.
pub(super) fn edit_input(event: &Event) -> Option<EditInput> {
    let key = keymap::key_press(event)?;
    EDIT.iter()
        .find(|(bound, _)| *bound == key)
        .map(|&(_, input)| input)
        .or_else(|| text_keys::text_edit(key).map(EditInput::Text))
}

#[cfg(test)]
mod tests {
    use crossterm::event::{KeyEvent, KeyEventKind};

    use super::*;
    use crate::app::TextEdit;

    fn press(code: KeyCode, modifiers: KeyModifiers) -> Event {
        Event::Key(KeyEvent::new(code, modifiers))
    }

    // FR-EDIT-01
    #[test]
    fn enter_goes_on_to_the_next_task_and_escape_ends_editing() {
        assert_eq!(
            edit_input(&press(KeyCode::Enter, KeyModifiers::NONE)),
            Some(EditInput::Next)
        );
        assert_eq!(
            edit_input(&press(KeyCode::Esc, KeyModifiers::NONE)),
            Some(EditInput::Done)
        );
    }

    // FR-EDIT-04
    #[test]
    fn other_keys_edit_the_title_as_in_any_text_field() {
        assert_eq!(
            edit_input(&press(KeyCode::Char('o'), KeyModifiers::NONE)),
            Some(EditInput::Text(TextEdit::Insert('o')))
        );
        assert_eq!(
            edit_input(&press(KeyCode::Backspace, KeyModifiers::NONE)),
            Some(EditInput::Text(TextEdit::DeleteBackward))
        );
        assert_eq!(
            edit_input(&press(KeyCode::Char('q'), KeyModifiers::CONTROL)),
            None
        );
    }

    #[test]
    fn only_presses_edit() {
        let release = Event::Key(KeyEvent::new_with_kind(
            KeyCode::Char('a'),
            KeyModifiers::NONE,
            KeyEventKind::Release,
        ));
        assert_eq!(edit_input(&release), None);
    }
}
