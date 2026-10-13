//! textual-items++, a keyboard-driven terminal todo app.
//!
//! Four layers with dependencies pointing inward (docs/code-quality.md, Architecture): [`ui`]
//! depends on [`app`] and [`store`], which depend on [`domain`]. Domain and App form a core that
//! knows nothing about the terminal or files, so another frontend could reuse it.

pub mod app;
pub mod domain;
pub mod store;
pub mod ui;
