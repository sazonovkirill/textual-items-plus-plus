//! The App layer: application state, messages, the update function, effects and the registries of
//! commands and views. Depends only on the domain layer, so the same core can run behind another
//! frontend.

mod adding;
mod all;
mod attachments;
pub mod autosave;
mod clipboard;
mod command;
mod completion;
mod container;
mod converting;
mod date_prompt;
mod editing;
mod focus;
mod history;
mod logbook;
mod move_to;
mod moving;
mod navigator;
mod pane;
mod picker;
mod saving;
mod selection;
mod sidebar;
mod state;
mod tags;
mod text_field;
mod text_path;
mod today;
mod trash;
mod upcoming;
mod update;
pub mod view;
mod visits;

pub use adding::Side;
pub use attachments::{AttachmentsInput, AttachmentsView};
pub use command::{Command, Context, Entry, Go, REGISTRY, Whose};
pub use container::{Row, rows};
pub use date_prompt::{DateInput, DatePromptView};
pub use editing::{EditInput, Part};
pub use focus::Focus;
pub use history::History;
pub use move_to::Destination;
pub use moving::{Direction, Refusal};
pub use navigator::{Place, Stop};
pub use picker::{Candidate, Entries, Picked, Picker, PickerInput, PickerView};
pub use saving::{ConflictSide, ConflictView, Notice, OnDisk, Report};
pub use selection::Selection;
pub use sidebar::SidebarView;
pub use state::{Opened, State};
pub use tags::{TagEntry, TagMark, TagsFor, TagsView};
pub use text_field::{Motion, TextEdit, TextField};
pub use update::{Effect, Message, update};
