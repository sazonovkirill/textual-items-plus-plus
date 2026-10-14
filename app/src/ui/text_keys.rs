//! The standard text keys every text field takes: caret motions, selection, deletion and typing,
//! as taskpaper-terminal's edit mode binds them (FR-EDIT-04, OQ-12,
//! [editing.md §2](../../../specs/reference/taskpaper-terminal/specs/editing.md#2-edit-mode)).

use crossterm::event::{KeyCode, KeyModifiers};

use super::keymap::Key;
use crate::app::{Motion, TextEdit};

const NONE: KeyModifiers = KeyModifiers::NONE;
const SHIFT: KeyModifiers = KeyModifiers::SHIFT;
const CTRL: KeyModifiers = KeyModifiers::CONTROL;
const ALT: KeyModifiers = KeyModifiers::ALT;
const CTRL_SHIFT: KeyModifiers = CTRL.union(SHIFT);
const ALT_SHIFT: KeyModifiers = ALT.union(SHIFT);

/// Moving the caret by `motion`, extending the selection with `select`.
const fn go(motion: Motion, select: bool) -> TextEdit {
    TextEdit::Move { motion, select }
}

/// The text keys besides typing, as the keymap matches them. `Shift+↑` and `Shift+↓` select to
/// the start and end of the text. The `Alt`, `Ctrl+A`, `Ctrl+E` and `Ctrl+U` keys are what macOS
/// terminals send for `Option` and `Cmd` with the arrows and `Delete`, as iTerm2's Natural Text
/// Editing preset maps them, and what macOS text fields take as well (D-93).
pub(super) const TEXT: &[(Key, TextEdit)] = &[
    ((NONE, KeyCode::Left), go(Motion::Left, false)),
    ((NONE, KeyCode::Right), go(Motion::Right, false)),
    ((CTRL, KeyCode::Left), go(Motion::WordLeft, false)),
    ((CTRL, KeyCode::Right), go(Motion::WordRight, false)),
    ((ALT, KeyCode::Left), go(Motion::WordLeft, false)),
    ((ALT, KeyCode::Right), go(Motion::WordRight, false)),
    ((ALT, KeyCode::Char('b')), go(Motion::WordLeft, false)),
    ((ALT, KeyCode::Char('f')), go(Motion::WordRight, false)),
    ((NONE, KeyCode::Home), go(Motion::Start, false)),
    ((NONE, KeyCode::End), go(Motion::End, false)),
    ((CTRL, KeyCode::Char('a')), go(Motion::LineStart, false)),
    ((CTRL, KeyCode::Char('e')), go(Motion::LineEnd, false)),
    ((SHIFT, KeyCode::Left), go(Motion::Left, true)),
    ((SHIFT, KeyCode::Right), go(Motion::Right, true)),
    ((CTRL_SHIFT, KeyCode::Left), go(Motion::WordLeft, true)),
    ((CTRL_SHIFT, KeyCode::Right), go(Motion::WordRight, true)),
    ((ALT_SHIFT, KeyCode::Left), go(Motion::WordLeft, true)),
    ((ALT_SHIFT, KeyCode::Right), go(Motion::WordRight, true)),
    ((SHIFT, KeyCode::Home), go(Motion::Start, true)),
    ((SHIFT, KeyCode::End), go(Motion::End, true)),
    ((SHIFT, KeyCode::Up), go(Motion::Start, true)),
    ((SHIFT, KeyCode::Down), go(Motion::End, true)),
    ((NONE, KeyCode::Backspace), TextEdit::DeleteBackward),
    ((NONE, KeyCode::Delete), TextEdit::DeleteForward),
    ((CTRL, KeyCode::Char('w')), TextEdit::DeleteWordBackward),
    ((ALT, KeyCode::Backspace), TextEdit::DeleteWordBackward),
    ((ALT, KeyCode::Char('d')), TextEdit::DeleteWordForward),
    ((ALT, KeyCode::Delete), TextEdit::DeleteWordForward),
    ((CTRL, KeyCode::Char('u')), TextEdit::DeleteLineBackward),
];

/// The change to a text field `key` asks for: its text key, else typing the character it is.
pub(super) fn text_edit(key: Key) -> Option<TextEdit> {
    TEXT.iter()
        .find(|(bound, _)| *bound == key)
        .map(|&(_, edit)| edit)
        .or_else(|| typed_char(key).map(TextEdit::Insert))
}

/// The character `key` types: a character without modifiers, or with `Ctrl+Alt`, which some
/// terminals report for `AltGr`.
fn typed_char((modifiers, code): Key) -> Option<char> {
    let typed = modifiers.is_empty() || modifiers == CTRL.union(ALT);
    match code {
        KeyCode::Char(c) if typed => Some(c),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    #[test]
    fn each_text_key_is_bound_once() {
        let keys: HashSet<Key> = TEXT.iter().map(|&(key, _)| key).collect();
        assert_eq!(keys.len(), TEXT.len());
    }

    // FR-EDIT-04
    #[test]
    fn motions_move_the_caret_and_with_shift_select() {
        for (key, expected) in [
            ((NONE, KeyCode::Left), go(Motion::Left, false)),
            ((CTRL, KeyCode::Right), go(Motion::WordRight, false)),
            ((ALT, KeyCode::Char('b')), go(Motion::WordLeft, false)),
            ((NONE, KeyCode::End), go(Motion::End, false)),
            ((SHIFT, KeyCode::Right), go(Motion::Right, true)),
            ((CTRL_SHIFT, KeyCode::Left), go(Motion::WordLeft, true)),
            ((SHIFT, KeyCode::Up), go(Motion::Start, true)),
            ((SHIFT, KeyCode::Down), go(Motion::End, true)),
            ((ALT, KeyCode::Left), go(Motion::WordLeft, false)),
            ((ALT_SHIFT, KeyCode::Right), go(Motion::WordRight, true)),
            ((CTRL, KeyCode::Char('a')), go(Motion::LineStart, false)),
            ((CTRL, KeyCode::Char('e')), go(Motion::LineEnd, false)),
        ] {
            assert_eq!(text_edit(key), Some(expected), "{key:?}");
        }
    }

    // FR-EDIT-04
    #[test]
    fn deleting() {
        for (key, expected) in [
            ((NONE, KeyCode::Backspace), TextEdit::DeleteBackward),
            ((NONE, KeyCode::Delete), TextEdit::DeleteForward),
            ((CTRL, KeyCode::Char('w')), TextEdit::DeleteWordBackward),
            ((ALT, KeyCode::Backspace), TextEdit::DeleteWordBackward),
            ((ALT, KeyCode::Char('d')), TextEdit::DeleteWordForward),
            ((ALT, KeyCode::Delete), TextEdit::DeleteWordForward),
            ((CTRL, KeyCode::Char('u')), TextEdit::DeleteLineBackward),
        ] {
            assert_eq!(text_edit(key), Some(expected), "{key:?}");
        }
    }

    #[test]
    fn characters_are_typed_alone_or_with_altgr() {
        for c in ['a', 'A', ' ', '@', 'é', '日'] {
            assert_eq!(
                text_edit((NONE, KeyCode::Char(c))),
                Some(TextEdit::Insert(c))
            );
        }
        assert_eq!(
            text_edit((CTRL.union(ALT), KeyCode::Char('€'))),
            Some(TextEdit::Insert('€'))
        );
    }

    #[test]
    fn other_keys_change_nothing() {
        for key in [
            (CTRL, KeyCode::Char('x')),
            (ALT, KeyCode::Char('x')),
            (NONE, KeyCode::Enter),
            (NONE, KeyCode::Esc),
            (NONE, KeyCode::Tab),
            (NONE, KeyCode::F(1)),
        ] {
            assert_eq!(text_edit(key), None, "{key:?}");
        }
    }
}
