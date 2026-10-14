//! Animation: the effects drawn over a rendered frame to show where focus landed, what a command
//! changed and that an action was refused, and to ease what opens, closes and changes in place,
//! as much as the `animations` setting lets them (FR-UI-07, D-72), ported from
//! taskpaper-terminal's ([ui.md §9](../../../specs/reference/taskpaper-terminal/specs/ui.md#9-animation-fr-ui-11)).
//! Hides tachyonfx, the triggers and every duration.

use std::{ffi::OsStr, mem, rc::Rc, time::Duration};

use ratatui::{
    buffer::{Buffer, Cell},
    layout::Rect,
    style::{Color, Modifier},
};
use tachyonfx::{ColorSpace, Effect, EffectManager, EffectTimer, Interpolation, fx};

use super::Theme;
use crate::{
    app::{Animations, BuiltIn, Command, Message, Row, State, view::Shown},
    domain::ItemId,
};

/// The setting a session runs with when the settings file does not set it: full, or reduced over
/// SSH, where every frame crosses the network, as `ssh_connection` and `ssh_tty`, the values of
/// `SSH_CONNECTION` and `SSH_TTY`, tell when either is set.
#[must_use]
pub fn animations_default(ssh_connection: Option<&OsStr>, ssh_tty: Option<&OsStr>) -> Animations {
    if ssh_connection.is_some() || ssh_tty.is_some() {
        Animations::Reduced
    } else {
        Animations::Full
    }
}

/// How long a frame lasts while an effect runs: 60 frames a second.
const FRAME: Duration = Duration::from_micros(16_667);

/// How long the landing flash and the change highlight last.
const FLASH: Duration = Duration::from_millis(250);

/// How long the refusal flash lasts.
const REFUSAL: Duration = Duration::from_millis(150);

/// How much of its color a flash mixes into a background in RGB at its start: a faint tint.
const TINT: f32 = 0.3;

/// How long what a change to the document altered on screen takes to pass from how it was drawn
/// to how it is now, as a completed or deleted task fades out.
const SETTLE: Duration = Duration::from_millis(200);

/// How long a window or panel takes to pass in or out, and a theme into another.
const SWAP: Duration = Duration::from_millis(150);

/// How long a pane body takes to fade in when it opens or shows another place.
const FADE_IN: Duration = Duration::from_millis(120);

/// How far toward the background dimmed text stands while it eases, as most terminals draw it.
const DIM_SHADE: f32 = 0.5;

/// What an effect shows; a new one replaces a running one of the same kind in the same pane.
/// `EffectManager` asks for keys with a default.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
enum Kind {
    /// Where focus landed.
    #[default]
    Landing,
    /// What a command changed.
    Change,
    /// That an action was refused.
    Refusal,
    /// What a change to the document altered, passing from how it was drawn.
    Settle,
    /// The body of a pane that opened or shows another place fading in.
    Root,
    /// The title line of such a pane.
    Title,
    /// The screen passing from how it was drawn, as a window opens or closes or the theme
    /// changes.
    Swap,
}

/// What an effect is keyed by: the position of its pane among the panes, `None` for the whole
/// screen, and its kind.
type Key = (Option<usize>, Kind);

/// What the app showed before an input, which tells after it whether focus jumped, which panes
/// show another place and whether a window opened or closed or the theme changed.
#[derive(Debug)]
pub(super) struct Before {
    /// The position of the pane with the keys among the panes.
    current: usize,
    focus: Option<ItemId>,
    /// What each pane shows.
    panes: Vec<Shown>,
    /// The rows of the pane with the keys, under its heading when the heading takes the focus.
    rows: (Option<ItemId>, Rc<[Row]>),
    windows: [bool; 11],
    theme: BuiltIn,
}

impl Before {
    /// What `state` shows now.
    pub(super) fn of(state: &State) -> Self {
        Self {
            current: state.current_pane(),
            focus: state.selection().focused(),
            panes: state.panes().map(|pane| pane.shown()).collect(),
            rows: {
                let pane = state.pane_view();
                (pane.heading(), pane.shared_rows())
            },
            windows: [
                state.navigator().is_some(),
                state.palette().is_some(),
                state.go_to_symbol().is_some(),
                state.move_to().is_some(),
                state.tags().is_some(),
                state.settings_window().is_some(),
                state.date_prompt().is_some(),
                state.attachments().is_some(),
                state.conflict().is_some(),
                state.sidebar().is_some(),
                state.shows_shortcuts(),
            ],
            theme: state.settings().theme,
        }
    }

    /// What the pane with the keys shows.
    fn pane(&self) -> Option<Shown> {
        self.panes.get(self.current).copied()
    }

    /// Where `id` is among the rows of the pane with the keys, the heading first.
    fn at(&self, id: ItemId) -> Option<usize> {
        let (heading, rows) = &self.rows;
        if *heading == Some(id) {
            return Some(0);
        }
        let at = rows.iter().position(|row| row.id() == id)?;
        Some(at + usize::from(heading.is_some()))
    }

    /// Whether focus jumped from here to `after`: it moved, other than to a row next to the one
    /// it left, in the rows before or after the move, in the same pane showing the same place.
    fn jumped_to(&self, after: &Self) -> bool {
        let (Some(focus), Some(was), Some(now)) = (after.focus, self.pane(), after.pane()) else {
            return false;
        };
        if self.current != after.current || was != now {
            return true;
        }
        let Some(left) = self.focus.filter(|&left| left != focus) else {
            return false;
        };
        let next_to = |rows: &Self| {
            rows.at(left)
                .zip(rows.at(focus))
                .is_some_and(|(a, b)| a.abs_diff(b) == 1)
        };
        !(next_to(self) || next_to(after))
    }

    /// The panes of `after` that opened or show another place than here: the one with the keys
    /// when a pane opened, else each that shows another place.
    fn rooted_in(&self, after: &Self) -> Vec<usize> {
        if after.panes.len() > self.panes.len() {
            return vec![after.current];
        }
        if after.panes.len() < self.panes.len() {
            return Vec::new();
        }
        (0..after.panes.len())
            .filter(|&index| self.panes.get(index) != after.panes.get(index))
            .collect()
    }
}

/// What a message may do that the effects show, told before the core takes it.
#[derive(Debug, Clone, Copy)]
pub(super) struct Intent {
    /// It brings items in or moves them where the user may not be looking: pasting,
    /// duplicating, Move to, undo or redo.
    brings: bool,
}

impl Intent {
    /// What `message` may do.
    pub(super) fn of(message: &Message) -> Self {
        Self {
            brings: matches!(
                message,
                Message::Paste(..)
                    | Message::MoveTo(_)
                    | Message::Command(Command::Duplicate | Command::Undo | Command::Redo)
            ),
        }
    }
}

/// A pane as a frame put it on screen, for the effects over it.
#[derive(Debug)]
pub(super) struct DrawnPane {
    /// Its position among the panes.
    pub index: usize,
    /// Whether it has the keys.
    pub focused: bool,
    /// Its focused item.
    pub focus: Option<ItemId>,
    /// Where its rows are on screen.
    pub body: Rect,
    /// Where its title line is, when it is on screen.
    pub title: Option<Rect>,
    /// The screen lines of each row on screen, in order.
    pub rows: Vec<(ItemId, Rect)>,
}

/// What the inputs since the last frame asked to show once the next frame places it.
#[derive(Debug, Default)]
struct Pending {
    landing: bool,
    refused: bool,
    /// The items whose rows the change highlight tints.
    changed: Vec<ItemId>,
    /// What passes from how the frame before drew it.
    passing: Option<Passing>,
    /// The panes that opened or show another place.
    rooted: Vec<usize>,
}

/// What passes from how the frame before drew it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Passing {
    /// The bodies of the panes, where a change to the document altered them.
    Panes,
    /// The whole screen, as a window, a panel or a pane opened or closed or the theme changed.
    Screen,
}

impl Pending {
    fn is_empty(&self) -> bool {
        !self.landing
            && !self.refused
            && self.changed.is_empty()
            && self.passing.is_none()
            && self.rooted.is_empty()
    }
}

/// What the messages of an input reported, gathered until it ends.
#[derive(Debug, Default)]
struct Fresh {
    refused: bool,
    /// The items to highlight.
    changed: Vec<ItemId>,
    /// Whether the document changed.
    settles: bool,
}

/// The effects over the screen, as the `animations` setting allows them.
pub(super) struct Animator {
    setting: Animations,
    manager: EffectManager<Key>,
    /// What the messages of the input being handled asked to show.
    fresh: Fresh,
    /// What the latest input asked to show, which the next frame starts.
    pending: Pending,
    /// The panes the frame being drawn put on screen.
    frame: Vec<DrawnPane>,
    /// The latest frame as its effects drew it, which what changes passes from; kept only
    /// while every animation runs.
    last: Buffer,
    /// The time, since the frontend started.
    now: Duration,
    /// When the latest frame was drawn.
    drawn_at: Duration,
}

impl std::fmt::Debug for Animator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Animator")
            .field("setting", &self.setting)
            .field("pending", &self.pending)
            .finish_non_exhaustive()
    }
}

impl Animator {
    /// Effects as `setting` allows them, at time zero.
    pub(super) fn new(setting: Animations) -> Self {
        Self {
            setting,
            manager: EffectManager::default(),
            fresh: Fresh::default(),
            pending: Pending::default(),
            frame: Vec::new(),
            last: Buffer::empty(Rect::ZERO),
            now: Duration::ZERO,
            drawn_at: Duration::ZERO,
        }
    }

    /// Follows `setting` from now on; effects it does not allow end at once.
    pub(super) fn set(&mut self, setting: Animations) {
        if setting == self.setting {
            return;
        }
        self.setting = setting;
        self.cut();
        self.pending = Pending::default();
        self.last = Buffer::empty(Rect::ZERO);
    }

    /// Lets time pass to `now`, since the frontend started.
    pub(super) fn at(&mut self, now: Duration) {
        self.now = self.now.max(now);
    }

    /// Ends every running effect, as an input does before it is taken, so that what it changes
    /// shows at once (ui.md §9.1, Cut short).
    pub(super) fn cut(&mut self) {
        self.manager = EffectManager::default();
    }

    /// Takes in what `state` reports after it handled a message that may do what `intent` says:
    /// the items it changed, which are highlighted when it changed items other than just the
    /// focused one or brought items in, and whether it was refused.
    pub(super) fn observe(&mut self, intent: Intent, state: &State) {
        let changed = state.changed();
        let focus = state.selection().focused();
        let elsewhere = changed.iter().any(|&id| Some(id) != focus);
        self.fresh.refused |= state.refused();
        self.fresh.settles |= !changed.is_empty();
        if elsewhere || intent.brings {
            self.fresh.changed.extend_from_slice(changed);
        }
    }

    /// Ends the input that started at `before`, whose effects the next frame starts: a landing
    /// flash when focus jumped, flashes for what changed and what was refused, and with every
    /// animation what the document change altered settling, panes that opened or show another
    /// place fading in, and the screen passing from how it was drawn when a window, a panel or
    /// a pane opened or closed or the theme changed.
    pub(super) fn settle(&mut self, before: &Before, state: &State) {
        let Fresh {
            refused,
            changed,
            settles,
        } = mem::take(&mut self.fresh);
        if self.setting == Animations::Off {
            return;
        }
        let after = Before::of(state);
        let mut pending = Pending {
            landing: before.jumped_to(&after),
            refused,
            changed,
            ..Pending::default()
        };
        if self.setting == Animations::Full {
            pending.rooted = before.rooted_in(&after);
            let swaps = before.windows != after.windows
                || before.theme != after.theme
                || before.panes.len() != after.panes.len();
            pending.passing = if !pending.rooted.is_empty() {
                None
            } else if swaps {
                Some(Passing::Screen)
            } else {
                settles.then_some(Passing::Panes)
            };
        }
        self.pending = pending;
    }

    /// Notes where the frame being drawn put `pane`.
    pub(super) fn drawn(&mut self, pane: DrawnPane) {
        self.frame.push(pane);
    }

    /// Whether an effect runs or waits to start, which asks for frames.
    fn animating(&self) -> bool {
        self.manager.is_running() || !self.pending.is_empty()
    }

    /// When the screen next changes by itself: the next frame while an effect runs or waits to
    /// start, `None` when nothing moves (N-PERF-03).
    pub(super) fn deadline(&self) -> Option<Duration> {
        self.animating()
            .then(|| self.drawn_at.saturating_add(FRAME))
    }

    /// Draws the effects over the frame being drawn on `buf` in `theme`'s colors: time moves on
    /// to now, and what the inputs since the last frame asked for starts.
    pub(super) fn process(&mut self, buf: &mut Buffer, theme: &Theme) {
        let panes = mem::take(&mut self.frame);
        let pending = mem::take(&mut self.pending);
        let elapsed = self.now.saturating_sub(self.drawn_at);
        self.drawn_at = self.now;
        // Time moves on off screen, so that effects done by now are never drawn.
        self.manager
            .process_effects(elapsed, &mut Buffer::empty(Rect::ZERO), Rect::ZERO);
        if self.setting != Animations::Off {
            let last = mem::replace(&mut self.last, Buffer::empty(Rect::ZERO));
            let last = (last.area == buf.area).then_some(last);
            for pane in &panes {
                self.start(pane, &pending, last.as_ref(), theme);
            }
            if let Some(last) = last.filter(|_| pending.passing == Some(Passing::Screen)) {
                let effect = pass(last, buf.area, SWAP, theme);
                self.manager.add_unique_effect((None, Kind::Swap), effect);
            }
        }
        // Drops effects that new ones replaced before they are drawn.
        self.manager
            .process_effects(Duration::ZERO, &mut Buffer::empty(Rect::ZERO), Rect::ZERO);
        let area = buf.area;
        self.manager.process_effects(Duration::ZERO, buf, area);
        if self.setting == Animations::Full {
            self.last = buf.clone();
        }
    }

    /// Starts over `pane` the effects `pending` asks for there, what changes passing from `last`,
    /// the frame before, when there is one of the same size.
    fn start(&mut self, pane: &DrawnPane, pending: &Pending, last: Option<&Buffer>, theme: &Theme) {
        let lines = |wanted: &dyn Fn(ItemId) -> bool| -> Vec<Rect> {
            pane.rows
                .iter()
                .filter(|&&(id, _)| wanted(id))
                .map(|&(_, lines)| lines)
                .collect()
        };
        let focus = pane.focus.filter(|_| pane.focused);
        let at_focus = |id| Some(id) == focus;
        let fades = self.setting == Animations::Full;
        let mut started = Vec::new();
        let flashes = [
            (Kind::Landing, pending.landing, theme.accent, FLASH),
            (Kind::Refusal, pending.refused, theme.error, REFUSAL),
        ];
        for (kind, asked, color, duration) in flashes {
            let lines = lines(&at_focus);
            if asked && !lines.is_empty() {
                started.push((kind, flash(lines, color, theme, duration, fades)));
            }
        }
        // The landing flash tints its row as the highlight does, so the highlight leaves it
        // alone rather than tinting it twice.
        let changed =
            lines(&|id| pending.changed.contains(&id) && !(pending.landing && at_focus(id)));
        if !changed.is_empty() {
            started.push((
                Kind::Change,
                flash(changed, theme.accent, theme, FLASH, fades),
            ));
        }
        if let Some(last) = last.filter(|_| pending.passing == Some(Passing::Panes)) {
            started.push((Kind::Settle, pass(last.clone(), pane.body, SETTLE, theme)));
        }
        if fades && pending.rooted.contains(&pane.index) {
            started.push((Kind::Root, fade_in(pane.body, theme)));
            if let Some(title) = pane.title {
                started.push((
                    Kind::Title,
                    flash(vec![title], theme.accent, theme, FLASH, true),
                ));
            }
        }
        for (kind, effect) in started {
            self.manager
                .add_unique_effect((Some(pane.index), kind), effect);
        }
    }
}

/// Whether `color` is an RGB color, which colors can pass through on their way to another.
fn rgb(color: Color) -> bool {
    matches!(color, Color::Rgb(..))
}

/// `color`, or `default` for the terminal's own color, which the theme names.
fn or_default(color: Color, default: Color) -> Color {
    if color == Color::Reset {
        default
    } else {
        color
    }
}

/// A flash over `lines` in `color` for `duration`: a faint tint of each cell's background, in
/// `theme`'s background where the cell has the terminal's own, that fades out with an ease-out
/// curve when `fades`, else `color` itself, or the tint when both are RGB, for the whole
/// duration, reverting in one step.
fn flash(lines: Vec<Rect>, color: Color, theme: &Theme, duration: Duration, fades: bool) -> Effect {
    let background = theme.background;
    let timer = EffectTimer::new(duration, Interpolation::QuadOut);
    fx::effect_fn(lines, timer, move |lines, context, cells| {
        let strength = if fades {
            TINT * (1.0 - context.alpha())
        } else {
            TINT
        };
        for (position, cell) in cells {
            if !lines.iter().any(|line| line.contains(position)) {
                continue;
            }
            let base = or_default(cell.bg, background);
            let tinted = if rgb(base) && rgb(color) {
                ColorSpace::Rgb.lerp(&base, &color, strength)
            } else {
                color
            };
            cell.set_bg(tinted);
        }
    })
}

/// `area` passing for `duration` from how `before` drew it to how it is drawn now, in `theme`'s
/// colors, every glyph already the one it ends with: where a cell keeps its glyph its colors
/// ease from the old to the new, dimmed text standing [`DIM_SHADE`] of the way toward its
/// background until the terminal's own dim takes over at the end; where its glyph changes, the
/// new one fades in from its background. Between colors that are not RGB, as with the
/// terminal's own, colors change at once and a new glyph is drawn dimmed for the first half.
fn pass(before: Buffer, area: Rect, duration: Duration, theme: &Theme) -> Effect {
    let (text, background) = (theme.text, theme.background);
    let timer = EffectTimer::new(duration, Interpolation::QuadOut);
    fx::effect_fn(before, timer, move |before, context, cells| {
        let alpha = context.alpha();
        for (position, cell) in cells {
            let Some(was) = before.cell(position).filter(|_| area.contains(position)) else {
                continue;
            };
            if *was == *cell {
                continue;
            }
            if was.symbol() == cell.symbol() {
                ease(was, cell, alpha, text, background);
            } else {
                let ground = or_default(cell.bg, background);
                fade(cell, alpha, text, ground);
            }
        }
    })
}

/// Eases `cell` `alpha` of the way from the colors and dimming `was` drew it in to its own, its
/// background and text in `text` and `background` where they are the terminal's own.
fn ease(was: &Cell, cell: &mut Cell, alpha: f32, text: Color, background: Color) {
    let (from, to) = (
        Look::of(was, text, background),
        Look::of(cell, text, background),
    );
    if ![from.fg, from.bg, to.fg, to.bg].into_iter().all(rgb) {
        return;
    }
    cell.modifier.remove(Modifier::DIM);
    cell.fg = ColorSpace::Rgb.lerp(&from.text(), &to.text(), alpha);
    cell.bg = ColorSpace::Rgb.lerp(&from.bg, &to.bg, alpha);
}

/// How a cell looks: its colors, the terminal's own taken as the theme's, and whether it is
/// dimmed.
struct Look {
    fg: Color,
    bg: Color,
    dim: bool,
}

impl Look {
    /// How `cell` looks with `text` and `background` for the terminal's own colors.
    fn of(cell: &Cell, text: Color, background: Color) -> Self {
        Self {
            fg: or_default(cell.fg, text),
            bg: or_default(cell.bg, background),
            dim: cell.modifier.contains(Modifier::DIM),
        }
    }

    /// The color its text shows in: dimmed text [`DIM_SHADE`] of the way to its background.
    fn text(&self) -> Color {
        if self.dim {
            ColorSpace::Rgb.lerp(&self.fg, &self.bg, DIM_SHADE)
        } else {
            self.fg
        }
    }
}

/// Shows `cell`'s glyph `shown` of the way in from `ground`: its text that far from it in RGB,
/// `text` standing for the terminal's own, or dimmed until it is half way in.
fn fade(cell: &mut Cell, shown: f32, text: Color, ground: Color) {
    let fg = or_default(cell.fg, text);
    if rgb(fg) && rgb(ground) {
        cell.fg = ColorSpace::Rgb.lerp(&ground, &fg, shown);
    } else if shown < 0.5 {
        cell.modifier.insert(Modifier::DIM);
    }
}

/// `area` fading in from the background for [`FADE_IN`], from the top line down: each line
/// starts as the one above it is part way in, the last one half way through.
fn fade_in(area: Rect, theme: &Theme) -> Effect {
    let (text, background) = (theme.text, theme.background);
    let last = f32::from(area.height.saturating_sub(1).max(1));
    let timer = EffectTimer::new(FADE_IN, Interpolation::QuadOut);
    fx::effect_fn((), timer, move |(), context, cells| {
        let alpha = context.alpha();
        for (position, cell) in cells {
            if !area.contains(position) {
                continue;
            }
            let row = f32::from(position.y - area.y) / last;
            let shown = (alpha * 2.0 - row).clamp(0.0, 1.0);
            // A blank shows no text, so it is left as it is, not sent each frame.
            if cell.symbol() != " " {
                fade(cell, shown, text, background);
            }
            let bg = or_default(cell.bg, background);
            if rgb(bg) && rgb(background) {
                cell.bg = ColorSpace::Rgb.lerp(&background, &bg, shown);
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use ratatui::style::Style;

    use super::*;
    use crate::app::view::View;

    #[test]
    fn sessions_over_ssh_default_to_reduced() {
        let set = Some(OsStr::new("10.0.0.1 22 10.0.0.2 22"));
        assert_eq!(animations_default(None, None), Animations::Full);
        assert_eq!(animations_default(set, None), Animations::Reduced);
        assert_eq!(
            animations_default(None, Some(OsStr::new("/dev/pts/1"))),
            Animations::Reduced
        );
    }

    const BLACK: Color = Color::Rgb(0, 0, 0);

    fn grey(level: u8) -> Color {
        Color::Rgb(level, level, level)
    }

    /// A theme with `text` on `background`, accented in grey 100.
    fn theme(text: Color, background: Color) -> Theme {
        Theme {
            text,
            background,
            accent: grey(100),
            ..BuiltIn::CatppuccinMocha.colors()
        }
    }

    /// A buffer one line tall holding `cells`, each a symbol in its colors with its modifier.
    fn line_of(cells: &[(&str, Color, Color, Modifier)]) -> Buffer {
        let width = u16::try_from(cells.len()).unwrap_or_default();
        let mut buf = Buffer::empty(Rect::new(0, 0, width, 1));
        for (x, &(symbol, fg, bg, modifier)) in (0..).zip(cells) {
            buf[(x, 0)]
                .set_symbol(symbol)
                .set_style(Style::new().fg(fg).bg(bg).add_modifier(modifier));
        }
        buf
    }

    /// Each cell of `buf`'s first line after `effect` has run over it for `millis`.
    fn after(
        mut effect: Effect,
        mut buf: Buffer,
        millis: u64,
    ) -> Vec<(String, Color, Color, Modifier)> {
        let area = buf.area;
        effect.process(Duration::from_millis(millis), &mut buf, area);
        (0..area.width)
            .map(|x| {
                let cell = &buf[(x, 0)];
                (cell.symbol().to_owned(), cell.fg, cell.bg, cell.modifier)
            })
            .collect()
    }

    fn cell(
        symbol: &str,
        fg: Color,
        bg: Color,
        modifier: Modifier,
    ) -> (String, Color, Color, Modifier) {
        (symbol.to_owned(), fg, bg, modifier)
    }

    // FR-UI-07
    #[test]
    fn a_flash_tints_its_lines_toward_its_color_and_fades_out_with_an_ease_out() {
        let flashed = |color: Color, background: Color, fades: bool, millis: u64| {
            let area = Rect::new(0, 0, 2, 2);
            let mut buf = Buffer::empty(area);
            buf[(1, 0)].set_bg(Color::Rgb(0, 0, 200));
            let effect = flash(
                vec![Rect::new(0, 0, 2, 1)],
                color,
                &theme(Color::Reset, background),
                FLASH,
                fades,
            );
            let mut effect = effect;
            effect.process(Duration::from_millis(millis), &mut buf, area);
            [buf[(0, 0)].bg, buf[(1, 0)].bg, buf[(0, 1)].bg]
        };
        assert_eq!(
            flashed(grey(100), BLACK, true, 0),
            [grey(29), Color::Rgb(29, 29, 170), Color::Reset],
            "30% of the way, only over its lines"
        );
        assert_eq!(
            flashed(grey(100), BLACK, true, 125),
            [grey(7), Color::Rgb(7, 7, 192), Color::Reset],
            "a quarter of that half way through"
        );
        assert_eq!(
            flashed(grey(100), BLACK, false, 125),
            [grey(29), Color::Rgb(29, 29, 170), Color::Reset],
            "in one step"
        );
        assert_eq!(
            flashed(Color::Yellow, BLACK, true, 0),
            [Color::Yellow; 2]
                .into_iter()
                .chain([Color::Reset])
                .collect::<Vec<_>>()[..],
            "an ANSI color is shown as it is"
        );
        assert_eq!(
            flashed(grey(100), Color::Reset, true, 0)[0],
            grey(100),
            "and so is any color over the terminal's own background"
        );
    }

    // FR-UI-07
    #[test]
    fn what_changed_eases_from_how_it_was_drawn_new_glyphs_fading_in() {
        let none = Modifier::empty();
        let was = line_of(&[
            ("a", grey(200), BLACK, none),
            ("b", grey(200), BLACK, none),
            ("c", grey(200), BLACK, none),
            ("d", grey(200), BLACK, Modifier::DIM),
        ]);
        let now = line_of(&[
            ("a", grey(200), BLACK, none),
            ("x", grey(200), BLACK, none),
            ("c", grey(100), BLACK, none),
            ("d", grey(200), BLACK, none),
        ]);
        let rgb_theme = theme(grey(200), BLACK);
        let passed = |millis| {
            after(
                pass(was.clone(), was.area, SETTLE, &rgb_theme),
                now.clone(),
                millis,
            )
        };
        assert_eq!(
            passed(0),
            [
                cell("a", grey(200), BLACK, none),
                cell("x", BLACK, BLACK, none),
                cell("c", grey(200), BLACK, none),
                cell("d", grey(100), BLACK, none),
            ],
            "at first in the colors it had, dimmed text half way to its background, a new glyph \
             not yet in"
        );
        assert_eq!(
            passed(100),
            [
                cell("a", grey(200), BLACK, none),
                cell("x", grey(150), BLACK, none),
                cell("c", grey(125), BLACK, none),
                cell("d", grey(175), BLACK, none),
            ],
            "three quarters of the way half way through"
        );
        assert_eq!(
            passed(200),
            after(
                pass(was.clone(), Rect::ZERO, SETTLE, &rgb_theme),
                now.clone(),
                0
            ),
            "and then as it is"
        );
        let ansi = theme(Color::Reset, Color::Reset);
        let stepped = |millis| {
            let was = line_of(&[("b", Color::Reset, Color::Reset, none)]);
            let now = line_of(&[("x", Color::Reset, Color::Reset, none)]);
            after(pass(was.clone(), was.area, SETTLE, &ansi), now, millis)
        };
        assert_eq!(
            stepped(10),
            [cell("x", Color::Reset, Color::Reset, Modifier::DIM)]
        );
        assert_eq!(stepped(150), [cell("x", Color::Reset, Color::Reset, none)]);
    }

    // FR-UI-07
    #[test]
    fn a_pane_fades_in_from_its_background_from_the_top_down() {
        let column = |theme: &Theme, millis| {
            let area = Rect::new(0, 0, 2, 3);
            let mut buf = Buffer::empty(area);
            for y in 0..3 {
                buf[(0, y)]
                    .set_symbol("x")
                    .set_fg(grey(200))
                    .set_bg(grey(100));
                buf[(1, y)].set_fg(grey(200));
            }
            let mut effect = fade_in(area, theme);
            effect.process(Duration::from_millis(millis), &mut buf, area);
            (0..3)
                .map(|y| {
                    (
                        buf[(0, y)].fg,
                        buf[(0, y)].bg,
                        buf[(0, y)].modifier,
                        buf[(1, y)].fg,
                    )
                })
                .collect::<Vec<_>>()
        };
        let none = Modifier::empty();
        assert_eq!(
            column(&theme(grey(200), BLACK), 30),
            [
                (grey(175), grey(87), none, grey(200)),
                (grey(75), grey(37), none, grey(200)),
                (BLACK, BLACK, none, grey(200)),
            ],
            "the top line furthest in, a blank's text color untouched"
        );
        assert_eq!(
            column(&theme(Color::Reset, Color::Reset), 30)
                .into_iter()
                .map(|(.., modifier, _)| modifier)
                .collect::<Vec<_>>(),
            [none, Modifier::DIM, Modifier::DIM],
            "without RGB, lines not yet half way in are dimmed"
        );
    }

    // FR-UI-07
    #[test]
    fn focus_lands_nowhere_when_nothing_has_it_and_jumps_when_it_moves_further_than_a_row() {
        let ids: Vec<ItemId> = (1..=3).map(ItemId::new).collect();
        let rows: Rc<[Row]> = (ids.iter())
            .map(|&id| Row::Task { id, section: None })
            .collect();
        let at = |focus: Option<usize>| Before {
            current: 0,
            focus: focus.and_then(|index| ids.get(index).copied()),
            panes: vec![Shown::View(View::Today)],
            rows: (None, Rc::clone(&rows)),
            windows: [false; 11],
            theme: BuiltIn::default(),
        };
        assert!(!at(Some(0)).jumped_to(&at(None)), "nothing to land on");
        assert!(!at(Some(0)).jumped_to(&at(Some(1))));
        assert!(at(Some(0)).jumped_to(&at(Some(2))));
        assert!(!at(None).jumped_to(&at(Some(2))), "nothing left");
        let elsewhere = Before {
            panes: vec![Shown::View(View::Upcoming)],
            ..at(Some(1))
        };
        assert!(at(Some(0)).jumped_to(&elsewhere), "to another place");
        let heading = ItemId::new(9);
        let headed = |focus| Before {
            focus: Some(focus),
            rows: (Some(heading), Rc::clone(&rows)),
            ..at(None)
        };
        assert!(
            !headed(heading).jumped_to(&headed(ItemId::new(1))),
            "the heading is the row above the first"
        );
        assert!(headed(heading).jumped_to(&headed(ItemId::new(2))));
    }

    #[test]
    fn the_debug_form_shows_the_setting_and_what_waits_to_start() {
        let debug = format!("{:?}", Animator::new(Animations::Off));
        assert!(
            debug.starts_with("Animator { setting: Off, pending: Pending {"),
            "{debug}"
        );
    }
}
