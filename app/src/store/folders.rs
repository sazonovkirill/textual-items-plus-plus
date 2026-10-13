//! Where the app keeps the files it writes beside the user's own: backups under the XDG data
//! folder and recovery files under the XDG state folder, in a folder named like the settings
//! folder (FR-SET-01), also on macOS (D-32).

use std::{ffi::OsString, path::PathBuf};

/// The folder of the app's own files under each XDG base folder.
const APP: &str = "textual-items-plus-plus";

/// The backups folder: `$XDG_DATA_HOME/textual-items-plus-plus/backups`, or under
/// `~/.local/share` when that is unset or not absolute. Backups are the user's data, not
/// disposable state. `None` when neither folder is known.
#[must_use]
pub fn backups(xdg_data_home: Option<OsString>, home: Option<PathBuf>) -> Option<PathBuf> {
    folder(xdg_data_home, home, ".local/share", "backups")
}

/// The recovery folder: `$XDG_STATE_HOME/textual-items-plus-plus/recovery`, or under
/// `~/.local/state` when that is unset or not absolute. `None` when neither folder is known.
#[must_use]
pub fn recovery(xdg_state_home: Option<OsString>, home: Option<PathBuf>) -> Option<PathBuf> {
    folder(xdg_state_home, home, ".local/state", "recovery")
}

/// `<base>/textual-items-plus-plus/<name>`, where `base` is an XDG base folder's variable, or
/// `fallback` under the home folder when the variable is unset or not absolute, as the XDG base
/// directory specification says.
fn folder(
    variable: Option<OsString>,
    home: Option<PathBuf>,
    fallback: &str,
    name: &str,
) -> Option<PathBuf> {
    let base = variable
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .or_else(|| home.map(|home| home.join(fallback)))?;
    Some(base.join(APP).join(name))
}

#[cfg(test)]
mod tests {
    use super::*;

    // F-FILE-09
    #[test]
    fn backups_go_under_xdg_data_home_else_the_home_folder() {
        let home = Some(PathBuf::from("/home/me"));
        assert_eq!(
            backups(Some("/data".into()), home.clone()),
            Some(PathBuf::from("/data/textual-items-plus-plus/backups"))
        );
        let default = Some(PathBuf::from(
            "/home/me/.local/share/textual-items-plus-plus/backups",
        ));
        assert_eq!(backups(None, home.clone()), default);
        assert_eq!(backups(Some("".into()), home.clone()), default);
        assert_eq!(backups(Some("data".into()), home), default, "not absolute");
        assert_eq!(backups(None, None), None);
        assert_eq!(backups(Some("data".into()), None), None);
    }

    // N-REL-01
    #[test]
    fn recovery_files_go_under_xdg_state_home_else_the_home_folder() {
        let home = Some(PathBuf::from("/home/me"));
        assert_eq!(
            recovery(Some("/state".into()), home.clone()),
            Some(PathBuf::from("/state/textual-items-plus-plus/recovery"))
        );
        let default = Some(PathBuf::from(
            "/home/me/.local/state/textual-items-plus-plus/recovery",
        ));
        assert_eq!(recovery(None, home.clone()), default);
        assert_eq!(recovery(Some("state".into()), home), default);
        assert_eq!(recovery(None, None), None);
    }
}
