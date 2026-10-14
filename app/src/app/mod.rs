//! The App layer: application state, messages, the update function, effects and the registries of
//! commands and views. Depends only on the domain layer, so the same core can run behind another
//! frontend.

pub mod autosave;
mod command;
mod container;
mod focus;
mod history;
mod picker;
mod selection;
mod state;
mod text_field;
mod update;
pub mod view;

pub use command::{Command, Context, Entry, REGISTRY};
pub use container::{Row, rows};
pub use focus::Focus;
pub use history::History;
pub use picker::{Candidate, Entries, Picked, Picker, PickerInput, PickerView};
pub use selection::Selection;
pub use state::{Opened, State};
pub use text_field::{Motion, TextEdit, TextField};
pub use update::{Effect, Message, update};
