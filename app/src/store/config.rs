//! The settings file `config.toml`: the settings it sets, each read on its own so that what the
//! app cannot use is reported and the rest still applies, and writing one setting back keeping
//! the file's comments, order and unknown keys (FR-SET-01, FR-SET-04). Hides the file's format;
//! what the settings are is the core's.

use std::{
    fs, io,
    path::{Path, PathBuf},
};

use serde::{Serialize, de::DeserializeOwned};
use thiserror::Error;
use toml_edit::{DocumentMut, Item};

use super::atomic;

/// Something in the settings file the app ignored (FR-SET-04).
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ConfigProblem {
    /// The file exists but cannot be read as text, so every setting keeps its default.
    #[error("cannot read it: {reason}")]
    Unreadable {
        /// What the system reported.
        reason: String,
    },
    /// The file is not TOML, so every setting keeps its default.
    #[error("line {line}: {message}")]
    Syntax {
        /// The line, counted from 1.
        line: usize,
        /// What the parser reported.
        message: String,
    },
    /// The file sets something the app has no setting for.
    #[error("unknown setting `{key}`")]
    UnknownSetting {
        /// The setting's key.
        key: String,
    },
    /// A setting's value is not one it can take, so it keeps its default.
    #[error("`{key}`: {message}")]
    InvalidValue {
        /// The setting's key.
        key: String,
        /// Why, as the reader says it.
        message: String,
    },
}

/// Why a setting could not be written to the settings file (FR-SET-04).
#[derive(Debug, Error)]
pub enum ConfigError {
    /// Neither `$XDG_CONFIG_HOME` nor a home folder says where the file goes.
    #[error("no place for it: neither $XDG_CONFIG_HOME nor a home folder is set")]
    NoFile,
    /// The settings have no setting of this key.
    #[error("no setting `{key}`")]
    NoSuchSetting {
        /// The key asked for.
        key: String,
    },
    /// The file is not TOML, so writing it would lose what it holds.
    #[error("not valid TOML: {message}")]
    NotToml {
        /// What the parser reported.
        message: String,
    },
    /// The file has a table where the setting's value goes.
    #[error("`{key}` is a table in the file")]
    InTheWay {
        /// The setting's key.
        key: String,
    },
    /// The settings cannot be written as TOML.
    #[error(transparent)]
    Toml(#[from] toml_edit::ser::Error),
    /// Reading or writing the file failed.
    #[error(transparent)]
    Io(#[from] io::Error),
}

/// The settings file of the app, if there is a place for one.
#[derive(Debug, Clone)]
pub struct ConfigFile {
    file: Option<PathBuf>,
}

impl ConfigFile {
    /// The settings file at `file`, or none anywhere when it is `None`.
    #[must_use]
    pub fn new(file: Option<PathBuf>) -> Self {
        Self { file }
    }

    /// Where the file is, when there is a place for it.
    #[must_use]
    pub fn path(&self) -> Option<&Path> {
        self.file.as_deref()
    }

    /// The settings the file sets, the default for each it does not set or sets to a value the
    /// settings cannot take, with what it sets that the app ignored. A missing file, or none
    /// anywhere, gives the defaults without problems; a file that cannot be read or is not TOML
    /// gives the defaults with that problem (FR-SET-01, FR-SET-04). `T` takes its defaults for
    /// what a file leaves out, as `#[serde(default)]` has it, since each key is read on its own.
    #[must_use]
    pub fn load<T: Serialize + DeserializeOwned + Default>(&self) -> (T, Vec<ConfigProblem>) {
        let Some(file) = &self.file else {
            return (T::default(), Vec::new());
        };
        let text = match fs::read_to_string(file) {
            Ok(text) => text,
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                return (T::default(), Vec::new());
            }
            Err(error) => {
                let reason = error.to_string();
                return (T::default(), vec![ConfigProblem::Unreadable { reason }]);
            }
        };
        match text.parse::<DocumentMut>() {
            Ok(document) => read(&document),
            Err(error) => {
                let at = error.span().map_or(0, |span| span.start);
                let line = text.get(..at).unwrap_or(&text).matches('\n').count() + 1;
                let message = error.message().to_owned();
                (T::default(), vec![ConfigProblem::Syntax { line, message }])
            }
        }
    }

    /// Sets the setting `key` in the file to its value in `settings`, keeping the file's
    /// comments, order and other keys, and a comment on the value it replaces; creates the file
    /// and its folder when they do not exist. The file is replaced at once, never seen half
    /// written, and a link to it stays a link (FR-SET-01, FR-SET-04).
    ///
    /// # Errors
    ///
    /// [`ConfigError`] when there is no place for the file, `settings` has no setting `key`, or
    /// the file is not TOML, holds a table at `key` or cannot be read or written; it is then left
    /// as it was.
    pub fn write<T: Serialize>(&self, key: &str, settings: &T) -> Result<(), ConfigError> {
        let file = self.file.as_deref().ok_or(ConfigError::NoFile)?;
        let Some(Item::Value(mut value)) = toml_edit::ser::to_document(settings)?.remove(key)
        else {
            return Err(ConfigError::NoSuchSetting {
                key: key.to_owned(),
            });
        };
        let text = match fs::read_to_string(file) {
            Ok(text) => text,
            Err(error) if error.kind() == io::ErrorKind::NotFound => String::new(),
            Err(error) => return Err(error.into()),
        };
        let mut document: DocumentMut =
            text.parse()
                .map_err(|error: toml_edit::TomlError| ConfigError::NotToml {
                    message: error.message().to_owned(),
                })?;
        match document.get_mut(key) {
            Some(Item::Value(old)) => {
                *value.decor_mut() = old.decor().clone();
                *old = value;
            }
            Some(Item::None) | None => {
                document.insert(key, Item::Value(value));
            }
            Some(_) => {
                return Err(ConfigError::InTheWay {
                    key: key.to_owned(),
                });
            }
        }
        file.parent().map(fs::create_dir_all).transpose()?;
        atomic::prepare(file, document.to_string().as_bytes())?.commit()?;
        Ok(())
    }
}

/// The settings `document` sets, each of its keys taken on its own: one the settings do not have
/// is reported, and one whose value they cannot take is reported and keeps its default.
fn read<T: Serialize + DeserializeOwned + Default>(
    document: &DocumentMut,
) -> (T, Vec<ConfigProblem>) {
    let known = toml_edit::ser::to_document(&T::default()).unwrap_or_default();
    let mut taken = DocumentMut::new();
    let mut problems = Vec::new();
    for (key, item) in document.iter() {
        if !known.contains_key(key) {
            problems.push(ConfigProblem::UnknownSetting {
                key: key.to_owned(),
            });
            continue;
        }
        let mut tried = taken.clone();
        tried.insert(key, item.clone());
        match toml_edit::de::from_document::<T>(tried.clone()) {
            Ok(_) => taken = tried,
            Err(error) => problems.push(ConfigProblem::InvalidValue {
                key: key.to_owned(),
                message: error.message().to_owned(),
            }),
        }
    }
    let settings = toml_edit::de::from_document(taken).unwrap_or_default();
    (settings, problems)
}

#[cfg(test)]
mod tests {
    use std::os::unix::fs::symlink;

    use serde::Deserialize;

    use super::*;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    /// Settings as the core might have them.
    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(default)]
    struct Settings {
        theme: Theme,
        wrap: bool,
    }

    impl Default for Settings {
        fn default() -> Self {
            Self {
                theme: Theme::Dark,
                wrap: true,
            }
        }
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(rename_all = "kebab-case")]
    enum Theme {
        Dark,
        FlexokiLight,
    }

    fn config(dir: &Path) -> ConfigFile {
        ConfigFile::new(Some(dir.join("settings/config.toml")))
    }

    fn load(dir: &Path, text: &str) -> Result<(Settings, Vec<ConfigProblem>), io::Error> {
        let config = config(dir);
        let path = config.path().map(Path::to_path_buf).unwrap_or_default();
        fs::create_dir_all(dir.join("settings"))?;
        fs::write(path, text)?;
        Ok(config.load())
    }

    // FR-SET-01
    #[test]
    fn a_missing_file_or_none_anywhere_gives_the_defaults_without_problems() -> TestResult {
        let dir = tempfile::tempdir()?;
        assert_eq!(config(dir.path()).load(), (Settings::default(), vec![]));
        assert_eq!(ConfigFile::new(None).load(), (Settings::default(), vec![]));
        assert_eq!(ConfigFile::new(None).path(), None);
        Ok(())
    }

    // FR-SET-01
    #[test]
    fn a_file_sets_the_settings_it_names() -> TestResult {
        let dir = tempfile::tempdir()?;
        let (settings, problems) = load(dir.path(), "theme = \"flexoki-light\"\nwrap = false\n")?;
        assert_eq!(
            settings,
            Settings {
                theme: Theme::FlexokiLight,
                wrap: false
            }
        );
        assert_eq!(problems, []);
        Ok(())
    }

    // FR-SET-04
    #[test]
    fn unknown_keys_and_invalid_values_are_reported_and_the_rest_applies() -> TestResult {
        let dir = tempfile::tempdir()?;
        let text = "font = \"mono\"\ntheme = \"nord\"\nwrap = false\n[keymap]\nx = 1\n";
        let (settings, problems) = load(dir.path(), text)?;
        assert_eq!(
            settings,
            Settings {
                theme: Theme::Dark,
                wrap: false
            }
        );
        let keys: Vec<_> = problems.iter().map(ToString::to_string).collect();
        assert_eq!(
            keys.iter()
                .map(|key| key.split(':').next())
                .collect::<Vec<_>>(),
            [
                Some("unknown setting `font`"),
                Some("`theme`"),
                Some("unknown setting `keymap`")
            ]
        );
        assert!(
            problems.get(1).is_some_and(|problem| problem
                .to_string()
                .starts_with("`theme`: unknown variant `nord`")),
            "{problems:?}"
        );
        let (_, problems) = load(dir.path(), "wrap = \"no\"\n")?;
        assert!(
            matches!(&problems[..], [ConfigProblem::InvalidValue { key, .. }] if key == "wrap"),
            "{problems:?}"
        );
        Ok(())
    }

    // FR-SET-04
    #[test]
    fn a_file_that_cannot_be_read_or_is_not_toml_gives_the_defaults_and_says_why() -> TestResult {
        let dir = tempfile::tempdir()?;
        let (settings, problems) = load(dir.path(), "wrap = false\ntheme = \n")?;
        assert_eq!(settings, Settings::default());
        assert!(
            matches!(&problems[..], [ConfigProblem::Syntax { line: 2, .. }]),
            "{problems:?}"
        );
        assert!(problems[0].to_string().starts_with("line 2: "));
        let (_, problems) = ConfigFile::new(Some(dir.path().to_path_buf())).load::<Settings>();
        assert!(
            matches!(&problems[..], [ConfigProblem::Unreadable { .. }]),
            "a folder: {problems:?}"
        );
        assert!(problems[0].to_string().starts_with("cannot read it: "));
        let (_, problems) = load(dir.path(), "")?;
        assert_eq!(problems, [], "an empty file sets nothing");
        Ok(())
    }

    // FR-SET-01, FR-SET-04
    #[test]
    fn writing_a_setting_creates_the_file_and_its_folder() -> TestResult {
        let dir = tempfile::tempdir()?;
        let config = config(dir.path());
        let settings = Settings {
            theme: Theme::FlexokiLight,
            wrap: false,
        };
        config.write("theme", &settings)?;
        let path = dir.path().join("settings/config.toml");
        assert_eq!(fs::read_to_string(&path)?, "theme = \"flexoki-light\"\n");
        config.write("wrap", &settings)?;
        assert_eq!(
            fs::read_to_string(&path)?,
            "theme = \"flexoki-light\"\nwrap = false\n"
        );
        assert_eq!(config.load(), (settings, vec![]), "it reads back");
        Ok(())
    }

    // FR-SET-04
    #[test]
    fn writing_a_setting_keeps_comments_order_and_unknown_keys() -> TestResult {
        let dir = tempfile::tempdir()?;
        let text = "# Mine\n\
                    wrap = true # long lines\n\
                    font = \"mono\"\n\
                    theme = \"dark\"  # at night\n\
                    \n\
                    [keymap]\n\
                    x = \"none\"\n";
        load(dir.path(), text)?;
        let settings = Settings {
            theme: Theme::FlexokiLight,
            wrap: false,
        };
        let config = config(dir.path());
        config.write("theme", &settings)?;
        config.write("wrap", &settings)?;
        assert_eq!(
            fs::read_to_string(dir.path().join("settings/config.toml"))?,
            "# Mine\n\
             wrap = false # long lines\n\
             font = \"mono\"\n\
             theme = \"flexoki-light\"  # at night\n\
             \n\
             [keymap]\n\
             x = \"none\"\n"
        );
        Ok(())
    }

    // FR-SET-04
    #[test]
    fn writing_through_a_link_replaces_the_file_it_points_to() -> TestResult {
        let dir = tempfile::tempdir()?;
        let target = dir.path().join("dotfiles.toml");
        fs::write(&target, "wrap = true\n")?;
        fs::create_dir_all(dir.path().join("settings"))?;
        let path = dir.path().join("settings/config.toml");
        symlink(&target, &path)?;
        let settings = Settings {
            theme: Theme::Dark,
            wrap: false,
        };
        config(dir.path()).write("wrap", &settings)?;
        assert!(fs::symlink_metadata(&path)?.file_type().is_symlink());
        assert_eq!(fs::read_to_string(&target)?, "wrap = false\n");
        Ok(())
    }

    // FR-SET-04
    #[test]
    fn a_file_that_cannot_take_the_setting_is_left_as_it_was() -> TestResult {
        let dir = tempfile::tempdir()?;
        let config = config(dir.path());
        let settings = Settings::default();
        let path = dir.path().join("settings/config.toml");
        for (text, error) in [
            ("theme = \n", "not valid TOML: "),
            ("[theme]\nx = 1\n", "`theme` is a table in the file"),
        ] {
            load(dir.path(), text)?;
            let written = config.write("theme", &settings);
            let message = written.map_err(|error| error.to_string()).err();
            assert!(
                message.as_deref().is_some_and(|m| m.starts_with(error)),
                "{text:?}: {message:?}"
            );
            assert_eq!(fs::read_to_string(&path)?, text, "unchanged");
        }
        fs::write(&path, b"theme = \"\xff\"\n")?;
        assert!(
            matches!(config.write("theme", &settings), Err(ConfigError::Io(_))),
            "a file that is not UTF-8 cannot be read"
        );
        assert!(matches!(
            config.write("font", &settings),
            Err(ConfigError::NoSuchSetting { .. })
        ));
        assert!(matches!(
            ConfigFile::new(None).write("theme", &settings),
            Err(ConfigError::NoFile)
        ));
        assert!(matches!(
            ConfigFile::new(Some(dir.path().to_path_buf())).write("theme", &settings),
            Err(ConfigError::Io(_))
        ));
        assert_eq!(
            ConfigError::NoFile.to_string(),
            "no place for it: neither $XDG_CONFIG_HOME nor a home folder is set"
        );
        Ok(())
    }
}
