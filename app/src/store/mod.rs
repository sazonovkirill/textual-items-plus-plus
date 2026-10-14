//! The Store layer: persistence, the `.taskpaper` format and its mapping to the domain model, and
//! file I/O, which every frontend reuses beside the core.
//!
//! The text side of saving lives here: replacing a file atomically, the daily
//! [`Backups`], [`Recovery`] files, watching a file for outside changes ([`watch()`]) and the
//! [`Disk`] worker that does all of it off the event loop. It takes text and paths; what to save
//! and when is the core's decision.

mod atomic;
mod backup;
pub mod demo;
mod disk;
mod folders;
mod open;
mod recovery;
pub mod taskpaper;
mod version;
mod watch;

pub use backup::{BackupError, Backups, KEEP as BACKUPS_KEPT};
pub use disk::{Disk, Job, Outcome};
pub use folders::{backups as backups_folder, recovery as recovery_folder};
pub use open::{OpenError, OpenFile, decode, open, recover};
pub use recovery::{Recovery, RecoveryError};
pub use version::Version;
pub use watch::{Watcher, watch};
