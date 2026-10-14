//! The Store layer: persistence, the `.taskpaper` format and its mapping to the domain model, and
//! file I/O, which every frontend reuses beside the core.
//!
//! The text side of saving lives here: replacing a file atomically, the daily
//! [`Backups`], [`Recovery`] files, the pane [`Layouts`], the settings' [`ConfigFile`], watching a file for outside changes ([`watch()`]) and the
//! [`Disk`] worker that does all of it off the event loop. It takes text and paths; what to save
//! and when is the core's decision.

mod atomic;
mod backup;
mod config;
pub mod demo;
mod disk;
mod folders;
mod layouts;
mod open;
mod recovery;
pub mod taskpaper;
mod version;
mod watch;

pub use backup::{BackupError, Backups, KEEP as BACKUPS_KEPT};
pub use config::{ConfigError, ConfigFile, ConfigProblem};
pub use disk::{Disk, Job, Outcome, Written, save_as};
pub use folders::{
    backups as backups_folder, config as config_file, layouts as layouts_file,
    recovery as recovery_folder,
};
pub use layouts::{Layouts, LayoutsError};
pub use open::{OpenError, OpenFile, decode, encode, open, recover};
pub use recovery::{Recovery, RecoveryError};
pub use version::Version;
pub use watch::{Watcher, watch};
