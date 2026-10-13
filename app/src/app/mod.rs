//! The App layer: application state, messages, the update function, effects and the registries of
//! commands and views. Depends only on the domain layer, so the same core can run behind another
//! frontend.

mod command;
mod selection;
mod state;
mod update;
pub mod view;

pub use command::{Command, Context, Entry, REGISTRY};
pub use selection::Selection;
pub use state::State;
pub use update::{Effect, Message, update};
