//! The keys of edit mode: `Enter` and `Esc` end it, as in taskpaper-terminal, and the standard
//! text keys edit the title (FR-EDIT-01, FR-EDIT-04,
//! [editing.md §2](../../../specs/reference/taskpaper-terminal/specs/editing.md#2-edit-mode)); a
//! description takes the keys of taskpaper-terminal's note editor instead
//! ([keybindings.md §4](../../../specs/reference/taskpaper-terminal/specs/keybindings.md#4-other-contexts)).

use crossterm::event::{Event, KeyCode, KeyModifiers};

use super::{
    keymap::{self, Key},
    text_keys,
};
use crate::app::{EditInput, Motion, TextEdit};

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

/// What `event` does while a description is edited, its lines wrapping in `width` columns:
/// `Enter` breaks the line, `Esc` ends editing, `↑` and `↓` move by lines on screen,
/// `Home` and `End` go to the start and end of the line, with `Ctrl` of the text, each selecting
/// with `Shift`, and the other text keys edit it. Only key presses count.
pub(super) fn description_input(event: &Event, width: usize) -> Option<EditInput> {
    let (modifiers, code) = keymap::key_press(event)?;
    let select = modifiers.contains(KeyModifiers::SHIFT);
    let moved = modifiers.difference(KeyModifiers::SHIFT);
    let motion = match (moved, code) {
        (KeyModifiers::NONE, KeyCode::Up) => Motion::Up { width },
        (KeyModifiers::NONE, KeyCode::Down) => Motion::Down { width },
        (KeyModifiers::NONE, KeyCode::Home) => Motion::LineStart,
        (KeyModifiers::NONE, KeyCode::End) => Motion::LineEnd,
        (KeyModifiers::CONTROL, KeyCode::Home) => Motion::Start,
        (KeyModifiers::CONTROL, KeyCode::End) => Motion::End,
        _ => {
            return match (modifiers, code) {
                (KeyModifiers::NONE, KeyCode::Enter) => {
                    Some(EditInput::Text(TextEdit::Insert('\n')))
                }
                (KeyModifiers::NONE, KeyCode::Esc) => Some(EditInput::Done),
                key => text_keys::text_edit(key).map(EditInput::Text),
            };
        }
    };
    Some(EditInput::Text(TextEdit::Move { motion, select }))
}

#[cfg(test)]
mod tests {
    use crossterm::event::{KeyEvent, KeyEventKind};

    use super::*;

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

    // FR-EDIT-03, FR-EDIT-04
    #[test]
    fn in_a_description_enter_breaks_the_line_and_escape_ends_editing() {
        let input = |code, modifiers| description_input(&press(code, modifiers), 30);
        assert_eq!(
            input(KeyCode::Enter, KeyModifiers::NONE),
            Some(EditInput::Text(TextEdit::Insert('\n')))
        );
        assert_eq!(
            input(KeyCode::Esc, KeyModifiers::NONE),
            Some(EditInput::Done)
        );
        assert_eq!(
            input(KeyCode::Char('n'), KeyModifiers::NONE),
            Some(EditInput::Text(TextEdit::Insert('n')))
        );
        assert_eq!(input(KeyCode::Char('q'), KeyModifiers::CONTROL), None);
    }

    // FR-EDIT-04, FR-UI-05
    #[test]
    fn in_a_description_the_arrows_move_by_lines_on_screen_and_home_and_end_by_line() {
        let shift = KeyModifiers::SHIFT;
        let ctrl = KeyModifiers::CONTROL;
        for (code, modifiers, motion, select) in [
            (
                KeyCode::Up,
                KeyModifiers::NONE,
                Motion::Up { width: 30 },
                false,
            ),
            (KeyCode::Down, shift, Motion::Down { width: 30 }, true),
            (KeyCode::Home, KeyModifiers::NONE, Motion::LineStart, false),
            (KeyCode::End, shift, Motion::LineEnd, true),
            (KeyCode::Home, ctrl, Motion::Start, false),
            (KeyCode::End, ctrl | shift, Motion::End, true),
            (KeyCode::Left, shift, Motion::Left, true),
        ] {
            assert_eq!(
                description_input(&press(code, modifiers), 30),
                Some(EditInput::Text(TextEdit::Move { motion, select })),
                "{code:?} {modifiers:?}"
            );
        }
    }
}
