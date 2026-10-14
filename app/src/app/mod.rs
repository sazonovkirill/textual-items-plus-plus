//! The App layer: application state, messages, the update function, effects and the registries of
//! commands and views. Depends only on the domain layer, so the same core can run behind another
//! frontend.

mod adding;
pub mod autosave;
mod command;
mod completion;
mod container;
mod editing;
mod focus;
mod history;
mod navigator;
mod picker;
mod selection;
mod state;
mod text_field;
mod trash;
mod update;
pub mod view;
mod visits;

pub use adding::Side;
pub use command::{Command, Context, Entry, Go, REGISTRY};
pub use container::{Row, rows};
pub use editing::EditInput;
pub use focus::Focus;
pub use history::History;
pub use navigator::Place;
pub use picker::{Candidate, Entries, Picked, Picker, PickerInput, PickerView};
pub use selection::Selection;
pub use state::{Opened, State};
pub use text_field::{Motion, TextEdit, TextField};
pub use update::{Effect, Message, update};
