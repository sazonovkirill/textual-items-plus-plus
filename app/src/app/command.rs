//! The command registry: every command a user can give, with its title and the context it is
//! given in. A frontend binds its own keys to these commands and lists them in its palette and
//! shortcuts panel.

/// A command: one user intent, whatever key, menu or palette gives it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Command {
    /// Leave the app.
    Quit,
}

/// Where a command can be given. Later steps add the modes and windows that take keys of their
/// own.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Context {
    /// Everywhere in the app.
    Global,
}

/// A command with what the palette and the shortcuts panel show of it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Entry {
    /// The command.
    pub command: Command,
    /// Its name as a user reads it.
    pub title: &'static str,
    /// Where it can be given.
    pub context: Context,
}

/// Every command, one entry each; a feature registers its commands here.
pub const REGISTRY: &[Entry] = &[Entry {
    command: Command::Quit,
    title: "Quit",
    context: Context::Global,
}];

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    #[test]
    fn every_command_is_registered_once_with_a_distinct_title() {
        let commands: HashSet<_> = REGISTRY.iter().map(|entry| entry.command).collect();
        let titles: HashSet<_> = REGISTRY.iter().map(|entry| entry.title).collect();
        assert_eq!(commands.len(), REGISTRY.len());
        assert_eq!(titles.len(), REGISTRY.len());
        assert!(REGISTRY.iter().all(|entry| !entry.title.is_empty()));
    }
}
