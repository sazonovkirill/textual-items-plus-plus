//! Prints the Reference's data as JSON: every command of the registry with its group and the
//! keys the default keymap binds it to, every window's own keys, every setting with its default
//! and the values the settings file takes, and every built-in theme.

use serde_json::{Value, json};
use textual_items_plus_plus::{
    app::{Animations, BuiltIn, Context, IconSet, KeyGroup, REGISTRY, Setting, Settings},
    ui::{Keymap, every_window_keys},
};

fn main() {
    let keymap = Keymap::default();
    let commands: Vec<Value> = REGISTRY
        .iter()
        .map(|entry| {
            json!({
                "title": entry.title,
                "group": KeyGroup::of(entry.command).name(),
                "context": match entry.context {
                    Context::Global => "global",
                    Context::Navigate => "navigate",
                    Context::Conflict => "conflict",
                },
                "keys": keys(&keymap.labels(entry.command)),
            })
        })
        .collect();
    let windows: Vec<Value> = every_window_keys()
        .into_iter()
        .map(|(window, bound)| {
            let keys: Vec<Value> = bound
                .iter()
                .map(|(name, labels)| json!({ "title": name, "keys": keys(labels) }))
                .collect();
            json!({ "window": window, "keys": keys })
        })
        .collect();
    let defaults = Settings::default();
    let settings: Vec<Value> = Setting::ALL
        .into_iter()
        .map(|setting| {
            json!({
                "key": setting.key(),
                "name": setting.name(),
                "group": setting.group(),
                "description": setting.description(),
                "default": defaults.value(setting),
                "file_default": file_value(&defaults, setting),
                "values": values(setting),
            })
        })
        .collect();
    let themes: Vec<Value> = BuiltIn::ALL
        .into_iter()
        .map(|theme| json!({ "id": theme.id(), "name": theme.name(), "light": theme.is_light() }))
        .collect();
    let data = json!({
        "commands": commands,
        "windows": windows,
        "settings": settings,
        "themes": themes,
    });
    println!(
        "{}",
        serde_json::to_string_pretty(&data).expect("the data is plain JSON")
    );
}

/// The key sequences of a label as `Keymap::labels` joins them, each on its own.
fn keys(labels: &str) -> Vec<&str> {
    if labels.is_empty() {
        Vec::new()
    } else {
        labels.split(", ").collect()
    }
}

/// How the settings file writes `setting`'s value in `settings`.
fn file_value(settings: &Settings, setting: Setting) -> Value {
    let value = serde_json::to_value(settings).expect("settings serialize");
    value[setting.key()].clone()
}

/// The values the settings file takes for `setting`, as it writes them. The matches are
/// exhaustive, so a new setting or value fails to compile here until it is listed.
fn values(setting: Setting) -> Value {
    match setting {
        Setting::Theme => BuiltIn::ALL.iter().map(|theme| theme.id()).collect(),
        Setting::Grayscale | Setting::WordWrap | Setting::TaskCounts | Setting::Weather => {
            json!([true, false])
        }
        Setting::Icons => [IconSet::Nerd, IconSet::Plain]
            .map(|icons| match icons {
                IconSet::Nerd | IconSet::Plain => serde_json::to_value(icons),
            })
            .into_iter()
            .collect::<Result<_, _>>()
            .expect("icon sets serialize"),
        Setting::Animations => [Animations::Full, Animations::Reduced, Animations::Off]
            .map(|animations| match animations {
                Animations::Full | Animations::Reduced | Animations::Off => {
                    serde_json::to_value(animations)
                }
            })
            .into_iter()
            .collect::<Result<_, _>>()
            .expect("animation levels serialize"),
        Setting::WeatherIdle => json!("a whole number of minutes, at least 1"),
    }
}
