//! The weather: leaves, snow or rain by season, falling over the whole screen once the app has
//! sat idle, until the next input, ported from taskpaper-terminal (FR-UI-08,
//! [ui.md §10](../../../specs/reference/taskpaper-terminal/specs/ui.md#10-weather-fr-ui-12)).
//! Hides the idle timer, the particles, the piles, their randomness and their glyphs; the core
//! knows nothing of it.

use std::time::Duration;

use crossterm::event::Event;
use jiff::civil::Date;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier},
};

use super::IconSet;

/// How long a frame of the weather lasts: 30 frames a second (ui.md §10.3).
const FRAME: Duration = Duration::from_micros(33_333);

/// The most frames one advance works out; after a longer pause the weather skips ahead.
const CATCH_UP: u32 = 30;

/// How long a raindrop's splash shows on the bottom row: three frames.
const SPLASH: Duration = FRAME.saturating_mul(3);

/// How fast the weather moves against the clock: half speed, to be calm to watch (D-147).
const PACE: f32 = 0.5;

/// How many columns rain moves right for each row it falls.
const SLANT: f32 = 0.25;

/// How much of a cell a settling snowflake adds to its drift, in eighths of a cell.
const FLAKE_DEPTH: u16 = 2;

/// The season, which picks the weather; summer has none (ui.md §10.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Season {
    /// March to May: rain.
    Spring,
    /// September to November: leaves.
    Autumn,
    /// December to February: snow.
    Winter,
}

impl Season {
    /// The season of `date`, by its month, or `None` in summer, June to August.
    pub(super) fn of(date: Date) -> Option<Self> {
        match date.month() {
            3..=5 => Some(Self::Spring),
            9..=11 => Some(Self::Autumn),
            12 | 1 | 2 => Some(Self::Winter),
            _ => None,
        }
    }
}

/// Whether `event` is input that ends the weather and counts as activity: a key, a mouse event,
/// a paste or a resize, not the terminal gaining or losing focus (ui.md §10.1).
pub(super) fn is_input(event: &Event) -> bool {
    matches!(
        event,
        Event::Key(_) | Event::Mouse(_) | Event::Paste(_) | Event::Resize(..)
    )
}

/// What arms the idle timer: the season whose weather it starts and how long it waits without
/// input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Idle {
    pub(super) season: Season,
    pub(super) after: Duration,
}

/// The idle timer, counting from time zero, and, while it plays, the weather.
#[derive(Debug, Default)]
pub(super) struct Weather {
    /// When the last input arrived, since the frontend started.
    last_input: Duration,
    /// The screen at the last draw, which the weather fills.
    area: Rect,
    sky: Option<Sky>,
}

impl Weather {
    /// Whether the weather plays.
    pub(super) fn playing(&self) -> bool {
        self.sky.is_some()
    }

    /// Takes in input at time `now`, which restarts the idle timer; returns whether it ended the
    /// weather, when it does nothing else.
    pub(super) fn input(&mut self, now: Duration) -> bool {
        self.last_input = now;
        self.sky.take().is_some()
    }

    /// Starts `season`'s weather at time `now`, with an empty sky and no piles.
    pub(super) fn start(&mut self, now: Duration, season: Season) {
        self.sky = Some(Sky::new(season, now, self.area));
    }

    /// When the idle timer `idle` arms runs out.
    fn due(&self, idle: Idle) -> Option<Duration> {
        self.last_input.checked_add(idle.after)
    }

    /// Lets time pass to `now`: starts the weather when `idle` arms the idle timer and it has run
    /// out, and moves the weather that plays.
    pub(super) fn tick(&mut self, now: Duration, idle: Option<Idle>) {
        match &mut self.sky {
            Some(sky) => sky.advance(now),
            None => {
                if let Some(idle) = idle
                    && self.due(idle).is_some_and(|due| now >= due)
                {
                    self.start(now, idle.season);
                }
            }
        }
    }

    /// When the screen next changes by itself: the next frame while the weather plays, else when
    /// the idle timer runs out if `idle` arms it; `None` otherwise (N-PERF-03).
    pub(super) fn deadline(&self, idle: Option<Idle>) -> Option<Duration> {
        match &self.sky {
            Some(sky) => sky.time.checked_add(FRAME),
            None => idle.and_then(|idle| self.due(idle)),
        }
    }

    /// Draws the weather over everything else in `area` of `buf`, in `icons`' glyphs, in grays
    /// when `grayscale` (FR-UI-11), and keeps the screen's size for the weather to fill.
    pub(super) fn draw(&mut self, buf: &mut Buffer, area: Rect, icons: IconSet, grayscale: bool) {
        self.area = area;
        if let Some(sky) = &mut self.sky {
            sky.fit(area);
            sky.paint(buf, icons, grayscale);
        }
    }
}

/// A small seeded generator (xorshift), so a test's weather is the same on every run.
#[derive(Debug)]
struct Random(u64);

impl Random {
    /// A generator seeded with `seed`.
    fn new(seed: u64) -> Self {
        Self(seed ^ 0x9E37_79B9_7F4A_7C15 | 1)
    }

    fn next(&mut self) -> u32 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        u32::try_from(x >> 32).unwrap_or_default()
    }

    /// A number from `low` up to `high`.
    fn between(&mut self, low: f32, high: f32) -> f32 {
        let unit = f32::from(u16::try_from(self.next() >> 16).unwrap_or_default()) / 65_536.0;
        low + unit * (high - low)
    }

    /// A number below `count`, or 0 when it is 0.
    fn below(&mut self, count: usize) -> usize {
        usize::try_from(self.next()).unwrap_or_default() % count.max(1)
    }
}

/// The cell at `at` along a side `size` cells long, if it is on it.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "the value is checked to be a whole number from 0 to below a u16 first"
)]
fn cell(at: f32, size: u16) -> Option<u16> {
    let at = at.floor();
    (at >= 0.0 && at < f32::from(size)).then_some(at as u16)
}

/// One falling leaf, flake or drop.
#[derive(Debug)]
struct Particle {
    /// The column it drifts about, and the row, from the top of the screen.
    x: f32,
    y: f32,
    /// How fast it falls, in rows a second at full [`PACE`].
    speed: f32,
    /// How far it sways to each side of `x` (leaves, snow), or how fast it moves right in
    /// columns a second (rain).
    drift: f32,
    /// Where it is in its sway, in radians.
    phase: f32,
    /// Which of the season's glyphs it shows, and which of its colors.
    look: usize,
    tint: usize,
}

impl Particle {
    /// The column it shows in.
    fn column(&self, season: Season) -> f32 {
        match season {
            Season::Spring => self.x,
            Season::Autumn | Season::Winter => self.x + self.drift * self.phase.sin(),
        }
    }
}

/// The weather that plays: its particles, piles and splashes on a screen of a size.
#[derive(Debug)]
struct Sky {
    season: Season,
    random: Random,
    /// How far the weather has moved, since the frontend started.
    time: Duration,
    area: Rect,
    particles: Vec<Particle>,
    /// Each column's leaves, from the bottom up, as the look and tint of each (autumn).
    leaves: Vec<Vec<(usize, usize)>>,
    /// Each column's drift, in eighths of a cell (winter).
    snow: Vec<u16>,
    /// Each splash's column and when it ends (spring).
    splashes: Vec<(u16, Duration)>,
}

impl Sky {
    /// `season`'s weather at time `now` on a screen of `area`.
    fn new(season: Season, now: Duration, area: Rect) -> Self {
        let mut sky = Self {
            season,
            random: Random::new(u64::try_from(now.as_nanos()).unwrap_or_default()),
            time: now,
            area: Rect::ZERO,
            particles: Vec::new(),
            leaves: Vec::new(),
            snow: Vec::new(),
            splashes: Vec::new(),
        };
        sky.fit(area);
        sky
    }

    /// Starts over on a screen of `area` when it differs from the one it fills.
    fn fit(&mut self, area: Rect) {
        if area == self.area {
            return;
        }
        self.area = area;
        let cells = usize::from(area.width) * usize::from(area.height);
        let density = match self.season {
            Season::Autumn => 120,
            Season::Winter => 40,
            Season::Spring => 30,
        };
        let count = cells.div_ceil(density);
        self.particles = (0..count).map(|_| self.particle()).collect();
        self.leaves = vec![Vec::new(); usize::from(area.width)];
        self.snow = vec![0; usize::from(area.width)];
        self.splashes.clear();
    }

    /// A new particle somewhere above the screen, so the sky starts empty and fills, and a
    /// particle that lands comes back after a while.
    fn particle(&mut self) -> Particle {
        let width = f32::from(self.area.width);
        let height = f32::from(self.area.height);
        let random = &mut self.random;
        let (speed, drift) = match self.season {
            Season::Autumn => (random.between(2.5, 4.5), random.between(1.0, 3.0)),
            Season::Winter => (random.between(2.0, 5.0), random.between(0.5, 1.5)),
            Season::Spring => {
                let speed = random.between(20.0, 30.0);
                (speed, speed * SLANT)
            }
        };
        // Rain starts as far left as it drifts on its way down, so the left edge gets its share.
        let left = match self.season {
            Season::Spring => -height * SLANT,
            Season::Autumn | Season::Winter => 0.0,
        };
        Particle {
            x: random.between(left, width),
            y: -random.between(0.0, height.max(1.0)),
            speed,
            drift,
            phase: random.between(0.0, std::f32::consts::TAU),
            look: random.below(8),
            tint: random.below(8),
        }
    }

    /// The most cells a pile may hold: a third of the screen's height.
    fn most(&self) -> usize {
        usize::from(self.area.height / 3)
    }

    /// Each frame up to `now`, at most [`CATCH_UP`] of them.
    fn advance(&mut self, now: Duration) {
        let mut frames = 0;
        while let Some(next) = self.time.checked_add(FRAME)
            && next <= now
        {
            if frames == CATCH_UP {
                self.time = now;
                break;
            }
            self.step();
            self.time = next;
            frames += 1;
        }
        let time = self.time;
        self.splashes.retain(|&(_, end)| end > time);
    }

    /// Moves every particle by a frame, settling or splashing those that reach the ground.
    fn step(&mut self) {
        let seconds = FRAME.as_secs_f32() * PACE;
        let bottom = self.area.height.saturating_sub(1);
        let heights: Vec<u16> = (0..self.area.width)
            .map(|column| self.height(column))
            .collect();
        let mut landed = Vec::new();
        for (at, particle) in self.particles.iter_mut().enumerate() {
            particle.y += particle.speed * seconds;
            match self.season {
                Season::Spring => particle.x += particle.drift * seconds,
                Season::Autumn | Season::Winter => particle.phase += seconds * 1.5,
            }
            let look = (particle.look, particle.tint);
            // Off to a side, it may sway or drift back, so it comes back once it falls past.
            let Some(column) = cell(particle.column(self.season), self.area.width) else {
                if particle.y >= f32::from(self.area.height) {
                    landed.push((at, None, look));
                }
                continue;
            };
            let height = heights
                .get(usize::from(column))
                .copied()
                .unwrap_or_default();
            let ground = f32::from(bottom.saturating_sub(height));
            if particle.y >= ground {
                landed.push((at, Some(column), look));
            }
        }
        for (at, column, look) in landed {
            if let Some(column) = column {
                match self.season {
                    Season::Autumn => self.settle_leaf(column, look),
                    Season::Winter => self.settle_flake(column),
                    Season::Spring => self.splashes.push((column, self.time + SPLASH)),
                }
            }
            let fresh = self.particle();
            if let Some(particle) = self.particles.get_mut(at) {
                *particle = fresh;
            }
        }
    }

    /// How many whole cells the pile at `column` covers.
    fn height(&self, column: u16) -> u16 {
        let column = usize::from(column);
        let cells = match self.season {
            Season::Autumn => self.leaves.get(column).map_or(0, Vec::len),
            Season::Winter => usize::from(self.snow.get(column).copied().unwrap_or_default() / 8),
            Season::Spring => 0,
        };
        u16::try_from(cells).unwrap_or(u16::MAX)
    }

    /// Where something landing at `column` among piles of `depths` comes to rest: there, or on
    /// the lower neighbor while its pile is lower by more than `step`, so piles have no spikes.
    fn rest(depths: &[usize], column: usize, step: usize) -> usize {
        let mut at = column;
        loop {
            let here = depths.get(at).copied().unwrap_or_default();
            let lower = [at.checked_sub(1), at.checked_add(1)]
                .into_iter()
                .flatten()
                .filter_map(|next| depths.get(next).map(|&depth| (next, depth)))
                .filter(|&(_, depth)| depth + step < here)
                .min_by_key(|&(_, depth)| depth);
            match lower {
                Some((next, _)) => at = next,
                None => return at,
            }
        }
    }

    /// Settles a leaf of `look` landing at `column` on the leaves there.
    fn settle_leaf(&mut self, column: u16, look: (usize, usize)) {
        let most = self.most();
        let depths: Vec<usize> = self.leaves.iter().map(Vec::len).collect();
        let at = Self::rest(&depths, usize::from(column), 1);
        if let Some(pile) = self.leaves.get_mut(at)
            && pile.len() < most
        {
            pile.push(look);
        }
    }

    /// Settles a snowflake landing at `column` into the drift there.
    fn settle_flake(&mut self, column: u16) {
        let most = u16::try_from(self.most() * 8).unwrap_or(u16::MAX);
        let depths: Vec<usize> = self.snow.iter().map(|&depth| usize::from(depth)).collect();
        let at = Self::rest(&depths, usize::from(column), 8);
        if let Some(depth) = self.snow.get_mut(at) {
            *depth = depth.saturating_add(FLAKE_DEPTH).min(most);
        }
    }

    /// Draws the piles, splashes and particles over `buf`, in grays when `grayscale`.
    fn paint(&self, buf: &mut Buffer, icons: IconSet, grayscale: bool) {
        let area = self.area;
        let bottom = area.bottom().saturating_sub(1);
        let looks = glyphs(self.season, icons);
        for (column, pile) in (area.left()..area.right()).zip(&self.leaves) {
            for (row, &(look, tint)) in (0..).zip(pile) {
                let glyph = pick(looks, look);
                put(
                    buf,
                    column,
                    bottom.saturating_sub(row),
                    glyph,
                    color(self.season, tint, grayscale),
                );
            }
        }
        let fill = DRIFT;
        for (column, &depth) in (area.left()..area.right()).zip(&self.snow) {
            let white = color(Season::Winter, 0, grayscale);
            for row in 0..depth / 8 {
                put(
                    buf,
                    column,
                    bottom.saturating_sub(row),
                    pick(fill, 7),
                    white,
                );
            }
            if depth % 8 > 0 {
                let glyph = fill.get(usize::from(depth % 8 - 1)).copied().unwrap_or(" ");
                put(buf, column, bottom.saturating_sub(depth / 8), glyph, white);
            }
        }
        for &(column, _) in &self.splashes {
            put(
                buf,
                area.left() + column,
                bottom,
                "˙",
                color(Season::Spring, 0, grayscale),
            );
        }
        for particle in &self.particles {
            let (Some(column), Some(row)) = (
                cell(particle.column(self.season), area.width),
                cell(particle.y, area.height),
            ) else {
                continue;
            };
            if row + self.height(column) > area.height.saturating_sub(1) {
                continue;
            }
            put(
                buf,
                area.left() + column,
                area.top() + row,
                pick(looks, particle.look),
                color(self.season, particle.tint, grayscale),
            );
        }
    }
}

/// The glyph `look` picks of `glyphs`.
fn pick(glyphs: &'static [&'static str], look: usize) -> &'static str {
    glyphs
        .get(look % glyphs.len().max(1))
        .copied()
        .unwrap_or(" ")
}

/// The glyphs of `season`'s particles in `icons`' set: no emoji, whose width differs between
/// terminals (ui.md §10.2).
fn glyphs(season: Season, icons: IconSet) -> &'static [&'static str] {
    match (season, icons) {
        (Season::Autumn, IconSet::Nerd) => &["\u{f06c}", "\u{f032a}", "❦", "❧"],
        (Season::Autumn, IconSet::Plain) => &["❦", "❧", "♣", "✿"],
        (Season::Winter, _) => &["·", "•", "*", "❄", "·", "•"],
        (Season::Spring, _) => &["╱", "╱", "'"],
    }
}

/// The glyphs of a drift's top cell, an eighth deeper each, the last a full cell.
const DRIFT: &[&str] = &["▁", "▂", "▃", "▄", "▅", "▆", "▇", "█"];

/// The color `tint` picks of `season`'s, in 256-color shades, which truecolor terminals show
/// too: the app tells neither truecolor nor `NO_COLOR` yet (D-30). In `grayscale`, each is the
/// grey of about its lightness, the leaves keeping their order from dark to light.
fn color(season: Season, tint: usize, grayscale: bool) -> Color {
    let leaves: [u8; 5] = if grayscale {
        [245, 249, 252, 242, 239]
    } else {
        [166, 208, 214, 130, 124]
    };
    Color::Indexed(match season {
        Season::Autumn => leaves.get(tint % leaves.len()).copied().unwrap_or_default(),
        Season::Winter => 255,
        Season::Spring if grayscale => 250,
        Season::Spring => 75,
    })
}

/// Puts `glyph` in `color` at column `x` of row `y` over what the cell showed, keeping its
/// background; a wide character it covers part of is blanked whole when the frame ends.
fn put(buf: &mut Buffer, x: u16, y: u16, glyph: &str, color: Color) {
    if let Some(cell) = buf.cell_mut((x, y)) {
        cell.set_symbol(glyph);
        cell.fg = color;
        cell.modifier = Modifier::empty();
    }
}

#[cfg(test)]
mod tests {
    use crossterm::event::{KeyCode, KeyEvent};
    use jiff::civil::date;
    use ratatui::text::Span;

    use super::*;

    const MINUTE: Duration = Duration::from_mins(1);

    fn seconds(seconds: u64) -> Duration {
        Duration::from_secs(seconds)
    }

    /// An idle timer armed for `season`'s weather after three minutes.
    fn idle(season: Season) -> Idle {
        Idle {
            season,
            after: 3 * MINUTE,
        }
    }

    /// A weather drawn once on a screen of `width` by `height`, so it knows the screen's size.
    fn weather(width: u16, height: u16) -> Weather {
        let mut weather = Weather::default();
        drawn(&mut weather, width, height);
        weather
    }

    /// What `weather` draws over a blank screen of `width` by `height`.
    fn drawn(weather: &mut Weather, width: u16, height: u16) -> Buffer {
        let area = Rect::new(0, 0, width, height);
        let mut buf = Buffer::empty(area);
        weather.draw(&mut buf, area, IconSet::Plain, false);
        buf
    }

    /// How many cells of `buf` show something.
    fn shown(buf: &Buffer) -> usize {
        buf.content
            .iter()
            .filter(|cell| cell.symbol() != " ")
            .count()
    }

    // ui.md §10.1
    #[test]
    fn the_season_goes_by_the_month_and_summer_has_none() {
        let seasons: Vec<_> = (1..=12)
            .map(|month| Season::of(date(2026, month, 15)))
            .collect();
        let (spring, autumn, winter) = (
            Some(Season::Spring),
            Some(Season::Autumn),
            Some(Season::Winter),
        );
        assert_eq!(
            seasons,
            [
                winter, winter, spring, spring, spring, None, None, None, autumn, autumn, autumn,
                winter
            ]
        );
    }

    // ui.md §10.1
    #[test]
    fn keys_clicks_pastes_and_resizes_are_input_and_focus_changes_are_not() {
        let key = Event::Key(KeyEvent::from(KeyCode::Char('a')));
        assert!(is_input(&key));
        assert!(is_input(&Event::Paste("a".into())));
        assert!(is_input(&Event::Resize(80, 24)));
        assert!(is_input(&Event::Mouse(crossterm::event::MouseEvent {
            kind: crossterm::event::MouseEventKind::Moved,
            column: 0,
            row: 0,
            modifiers: crossterm::event::KeyModifiers::NONE,
        })));
        assert!(!is_input(&Event::FocusGained));
        assert!(!is_input(&Event::FocusLost));
    }

    // FR-UI-08, ui.md §10.1
    #[test]
    fn the_weather_starts_once_the_app_has_sat_idle_for_the_idle_time() {
        let mut weather = weather(20, 9);
        let autumn = Some(idle(Season::Autumn));
        assert_eq!(weather.deadline(autumn), Some(3 * MINUTE));
        weather.input(seconds(10));
        assert_eq!(weather.deadline(autumn), Some(3 * MINUTE + seconds(10)));
        weather.tick(3 * MINUTE + seconds(9), autumn);
        assert!(!weather.playing(), "a second early");
        weather.tick(3 * MINUTE + seconds(10), autumn);
        assert!(weather.playing());
        assert_eq!(
            weather.deadline(autumn),
            Some(3 * MINUTE + seconds(10) + FRAME),
            "frames while it plays"
        );
    }

    // ui.md §10.1, N-PERF-03
    #[test]
    fn unarmed_nothing_waits_or_starts() {
        let mut weather = weather(20, 9);
        assert_eq!(weather.deadline(None), None);
        weather.tick(10 * MINUTE, None);
        assert!(!weather.playing());
    }

    // ui.md §10.1
    #[test]
    fn the_idle_time_is_what_arms_the_timer() {
        let mut weather = weather(20, 9);
        let spring = Some(Idle {
            season: Season::Spring,
            after: MINUTE,
        });
        assert_eq!(weather.deadline(spring), Some(MINUTE));
        weather.tick(MINUTE, spring);
        assert!(weather.playing());
    }

    // ui.md §10.1
    #[test]
    fn the_first_input_ends_the_weather_and_says_so_and_later_ones_do_not() {
        let mut weather = weather(20, 9);
        weather.start(MINUTE, Season::Winter);
        assert!(weather.input(2 * MINUTE), "it ended the weather");
        assert!(!weather.playing());
        assert!(!weather.input(2 * MINUTE), "nothing left to end");
        assert_eq!(
            weather.deadline(Some(idle(Season::Winter))),
            Some(5 * MINUTE),
            "the idle time counts again from the input"
        );
    }

    // ui.md §10.2
    #[test]
    fn the_sky_starts_empty_and_fills_as_the_weather_falls() {
        for season in [Season::Autumn, Season::Winter, Season::Spring] {
            let mut weather = weather(40, 12);
            weather.start(Duration::ZERO, season);
            assert_eq!(shown(&drawn(&mut weather, 40, 12)), 0, "{season:?}");
            weather.tick(seconds(4), Some(idle(season)));
            assert!(shown(&drawn(&mut weather, 40, 12)) > 0, "{season:?}");
        }
    }

    // ui.md §10.2
    #[test]
    fn leaves_and_snow_pile_up_to_a_third_of_the_height_and_rain_does_not() {
        for (season, piles) in [
            (Season::Autumn, true),
            (Season::Winter, true),
            (Season::Spring, false),
        ] {
            let mut weather = weather(12, 9);
            weather.start(Duration::ZERO, season);
            let mut now = Duration::ZERO;
            for _ in 0..1200 {
                now += seconds(1);
                weather.tick(now, Some(idle(season)));
            }
            let buf = drawn(&mut weather, 12, 9);
            let row = |y: u16| (0..12).filter(|&x| buf[(x, y)].symbol() != " ").count();
            if piles {
                assert_eq!(row(8), 12, "{season:?}: the bottom row is covered");
                assert_eq!(row(6), 12, "{season:?}: the piles are three rows deep");
            }
            let sky = weather.sky.as_ref().expect("it plays");
            for column in 0..12 {
                assert!(sky.height(column) <= 3, "{season:?}");
            }
            assert_eq!((0..12).any(|column| sky.height(column) > 0), piles);
        }
    }

    // ui.md §10.2
    #[test]
    fn a_drift_has_no_spikes() {
        let mut weather = weather(12, 30);
        weather.start(Duration::ZERO, Season::Winter);
        let mut now = Duration::ZERO;
        for _ in 0..300 {
            now += seconds(1);
            weather.tick(now, Some(idle(Season::Winter)));
            let snow = &weather.sky.as_ref().expect("it plays").snow;
            for pair in snow.windows(2) {
                assert!(pair[0].abs_diff(pair[1]) <= 8 + FLAKE_DEPTH, "{snow:?}");
            }
        }
    }

    // ui.md §10.2
    #[test]
    fn a_drift_shows_its_depth_in_eighths_of_a_cell() {
        let mut weather = weather(3, 9);
        weather.start(Duration::ZERO, Season::Winter);
        let sky = weather.sky.as_mut().expect("it plays");
        sky.particles.clear();
        sky.snow = vec![2, 8, 10];
        let buf = drawn(&mut weather, 3, 9);
        assert_eq!(
            [
                buf[(0, 8)].symbol(),
                buf[(1, 8)].symbol(),
                buf[(2, 8)].symbol()
            ],
            ["▂", "█", "█"]
        );
        assert_eq!(buf[(2, 7)].symbol(), "▂");
        assert_eq!(buf[(1, 7)].symbol(), " ");
        assert_eq!(buf[(0, 8)].fg, Color::Indexed(255));
    }

    // ui.md §10.2
    #[test]
    fn a_particle_is_not_drawn_inside_a_pile() {
        let mut weather = weather(1, 9);
        weather.start(Duration::ZERO, Season::Winter);
        let sky = weather.sky.as_mut().expect("it plays");
        sky.snow = vec![16];
        sky.particles = vec![Particle {
            x: 0.0,
            y: 7.5,
            speed: 3.0,
            drift: 0.0,
            phase: 0.0,
            look: 3,
            tint: 0,
        }];
        assert_eq!(drawn(&mut weather, 1, 9)[(0, 7)].symbol(), "█");
        let sky = weather.sky.as_mut().expect("it plays");
        sky.particles[0].y = 6.5;
        assert_eq!(
            drawn(&mut weather, 1, 9)[(0, 6)].symbol(),
            "❄",
            "just above the drift"
        );
    }

    /// A particle at `x`, `y` that falls `speed` rows a second and sways `drift` columns, or
    /// drifts that many a second as rain, with its sway at `phase`.
    fn particle(x: f32, y: f32, speed: f32, drift: f32, phase: f32) -> Particle {
        Particle {
            x,
            y,
            speed,
            drift,
            phase,
            look: 0,
            tint: 0,
        }
    }

    /// The only particle of a sky of `season` on a screen of `width` by `height`, after `frames`
    /// frames from `start`.
    fn moved(
        season: Season,
        start: Particle,
        (width, height): (u16, u16),
        frames: u32,
    ) -> Particle {
        let mut weather = weather(width, height);
        weather.start(Duration::ZERO, season);
        let sky = weather.sky.as_mut().expect("it plays");
        sky.particles = vec![start];
        weather.tick(FRAME * frames, Some(idle(season)));
        weather.sky.expect("it plays").particles.remove(0)
    }

    fn close(left: f32, right: f32) -> bool {
        (left - right).abs() < 1e-4
    }

    // ui.md §10.3
    #[test]
    fn the_generator_gives_the_same_numbers_from_the_same_seed() {
        let mut random = Random::new(0);
        let numbers: Vec<u32> = (0..4).map(|_| random.next()).collect();
        assert_eq!(
            numbers,
            [3_692_787_630, 1_693_511_353, 2_064_109_201, 811_533_580]
        );
    }

    // ui.md §10.3
    #[test]
    fn the_generator_spreads_its_numbers_evenly() {
        let mut random = Random::new(7);
        let mut counts = [0; 8];
        for _ in 0..8000 {
            counts[random.below(8)] += 1;
        }
        assert!(
            counts.iter().all(|&count| (850..1150).contains(&count)),
            "{counts:?}"
        );
        let numbers: Vec<f32> = (0..1000).map(|_| random.between(2.0, 4.0)).collect();
        assert!(numbers.iter().all(|&number| (2.0..4.0).contains(&number)));
        assert!(numbers.iter().any(|&number| number < 2.1));
        assert!(numbers.iter().any(|&number| number > 3.9));
        assert_eq!(random.below(0), 0);
    }

    // ui.md §10.2
    #[test]
    fn a_cell_is_on_a_side_only_from_zero_up_to_below_its_length() {
        assert_eq!(cell(-0.1, 3), None);
        assert_eq!(cell(0.0, 3), Some(0));
        assert_eq!(cell(2.9, 3), Some(2));
        assert_eq!(cell(3.0, 3), None);
    }

    // ui.md §10.2
    #[test]
    fn leaves_and_snow_sway_about_their_column_and_rain_does_not() {
        let swaying = particle(5.0, 0.0, 3.0, 2.0, std::f32::consts::FRAC_PI_2);
        assert!(close(swaying.column(Season::Autumn), 7.0));
        assert!(close(swaying.column(Season::Winter), 7.0));
        assert!(close(swaying.column(Season::Spring), 5.0));
    }

    // ui.md §10.2
    #[test]
    fn a_frame_moves_rain_down_and_right_and_leaves_down_along_their_sway_at_half_speed() {
        let seconds = FRAME.as_secs_f32() / 2.0;
        let drop = moved(
            Season::Spring,
            particle(1.0, 2.0, 24.0, 6.0, 0.0),
            (20, 20),
            1,
        );
        assert!(close(drop.y, 2.0 + 24.0 * seconds), "{drop:?}");
        assert!(close(drop.x, 1.0 + 6.0 * seconds), "{drop:?}");
        let leaf = moved(
            Season::Autumn,
            particle(1.0, 2.0, 3.0, 2.0, 0.5),
            (20, 20),
            1,
        );
        assert!(close(leaf.y, 2.0 + 3.0 * seconds), "{leaf:?}");
        assert!(close(leaf.phase, 0.5 + 1.5 * seconds), "{leaf:?}");
        assert!(close(leaf.x, 1.0), "{leaf:?}");
    }

    // ui.md §10.2
    #[test]
    fn rain_slants_and_starts_as_far_left_as_it_drifts() {
        let mut weather = weather(40, 20);
        weather.start(Duration::ZERO, Season::Spring);
        let drops = &weather.sky.as_ref().expect("it plays").particles;
        assert!(
            drops
                .iter()
                .all(|drop| close(drop.drift, drop.speed * SLANT))
        );
        assert!(drops.iter().all(|drop| (-5.0..40.0).contains(&drop.x)));
        assert!(drops.iter().any(|drop| drop.x < 0.0));
        assert!(drops.iter().all(|drop| (-20.0..=0.0).contains(&drop.y)));
    }

    // ui.md §10.2
    #[test]
    fn a_particle_off_a_side_comes_back_only_once_it_falls_past_the_screen() {
        let falling = moved(
            Season::Spring,
            particle(-3.0, 5.0, 24.0, 6.0, 0.0),
            (10, 10),
            1,
        );
        assert!(falling.y > 5.0, "it keeps falling: {falling:?}");
        let past = moved(
            Season::Spring,
            particle(12.0, 9.9, 24.0, 6.0, 0.0),
            (10, 10),
            1,
        );
        assert!(past.y <= 0.0, "it starts again above the screen: {past:?}");
    }

    // ui.md §10.2
    #[test]
    fn a_landed_particle_starts_again_above_the_screen() {
        let leaf = moved(
            Season::Autumn,
            particle(3.0, 8.98, 3.0, 0.0, 0.0),
            (10, 10),
            1,
        );
        assert!(leaf.y <= 0.0, "{leaf:?}");
    }

    // ui.md §10.2
    #[test]
    fn something_landing_rests_on_a_lower_neighbor_while_it_is_lower_by_more_than_a_step() {
        assert_eq!(Sky::rest(&[0, 1, 2], 2, 1), 2, "one lower is no spike");
        assert_eq!(Sky::rest(&[0, 0, 2], 2, 1), 1);
        assert_eq!(Sky::rest(&[0, 2, 4], 2, 1), 0, "it rolls on down");
        assert_eq!(Sky::rest(&[3, 0, 3], 1, 1), 1, "the lowest stays");
        assert_eq!(Sky::rest(&[0, 5, 2], 1, 1), 0, "the lower side");
    }

    // ui.md §10.2
    #[test]
    fn the_weather_draws_where_its_screen_is() {
        let area = Rect::new(3, 2, 4, 6);
        let mut weather = Weather::default();
        weather.draw(&mut Buffer::empty(area), area, IconSet::Plain, false);
        weather.start(Duration::ZERO, Season::Spring);
        let sky = weather.sky.as_mut().expect("it plays");
        sky.particles = vec![particle(1.5, 0.5, 24.0, 0.0, 0.0)];
        sky.splashes = vec![(2, SPLASH)];
        let mut buf = Buffer::empty(area);
        weather.draw(&mut buf, area, IconSet::Plain, false);
        assert_eq!(buf[(4, 2)].symbol(), "╱");
        assert_eq!(buf[(5, 7)].symbol(), "˙");
    }

    // ui.md §10.2
    #[test]
    fn rain_splashes_on_the_bottom_row_for_a_moment() {
        let mut weather = weather(1, 3);
        weather.start(Duration::ZERO, Season::Spring);
        let sky = weather.sky.as_mut().expect("it plays");
        sky.particles = vec![Particle {
            x: 0.0,
            y: 1.6,
            speed: 30.0,
            drift: 0.0,
            phase: 0.0,
            look: 0,
            tint: 0,
        }];
        weather.tick(FRAME, Some(idle(Season::Spring)));
        assert_eq!(drawn(&mut weather, 1, 3)[(0, 2)].symbol(), "˙");
        weather.tick(FRAME * 2, Some(idle(Season::Spring)));
        assert_eq!(
            drawn(&mut weather, 1, 3)[(0, 2)].symbol(),
            "˙",
            "the frame before its end"
        );
        weather.tick(SPLASH, Some(idle(Season::Spring)));
        assert_eq!(drawn(&mut weather, 1, 3)[(0, 2)].symbol(), " ");
    }

    // ui.md §10.2
    #[test]
    fn a_particle_covers_the_cell_it_is_in_and_keeps_its_background() {
        let mut weather = weather(3, 3);
        weather.start(Duration::ZERO, Season::Autumn);
        let sky = weather.sky.as_mut().expect("it plays");
        sky.particles = vec![Particle {
            x: 1.2,
            y: 0.5,
            speed: 3.0,
            drift: 0.0,
            phase: 0.0,
            look: 1,
            tint: 2,
        }];
        let mut buf = Buffer::with_lines(["abc", "def", "ghi"]);
        buf.set_style(buf.area, ratatui::style::Style::new().bg(Color::Blue).dim());
        weather.draw(&mut buf, Rect::new(0, 0, 3, 3), IconSet::Plain, false);
        let cell = &buf[(1, 0)];
        assert_eq!(
            (cell.symbol(), cell.fg, cell.bg, cell.modifier),
            ("❧", Color::Indexed(214), Color::Blue, Modifier::empty())
        );
        assert_eq!(buf[(0, 0)].symbol(), "a");
    }

    // ui.md §10.2
    #[test]
    fn leaves_take_several_warm_colors_snow_white_and_rain_blue() {
        let leaves: Vec<_> = (0..5)
            .map(|tint| color(Season::Autumn, tint, false))
            .collect();
        assert_eq!(
            leaves,
            [166, 208, 214, 130, 124].map(Color::Indexed),
            "red, orange, yellow, brown, dark red"
        );
        assert_eq!(
            color(Season::Autumn, 6, false),
            Color::Indexed(208),
            "going round"
        );
        assert_eq!(color(Season::Winter, 3, false), Color::Indexed(255));
        assert_eq!(color(Season::Spring, 0, false), Color::Indexed(75));
    }

    // FR-UI-11
    #[test]
    fn in_grayscale_leaves_snow_and_rain_are_greys() {
        let grey = |color| matches!(color, Color::Indexed(232..=255));
        for season in [Season::Autumn, Season::Winter, Season::Spring] {
            for tint in 0..5 {
                assert!(grey(color(season, tint, true)), "{season:?} {tint}");
            }
        }
        let leaves: Vec<_> = (0..5)
            .map(|tint| color(Season::Autumn, tint, true))
            .collect();
        assert_eq!(leaves, [245, 249, 252, 242, 239].map(Color::Indexed));
    }

    // ui.md §10.2
    #[test]
    fn every_glyph_is_one_cell_wide_and_ascii_ones_are_ascii() {
        for season in [Season::Autumn, Season::Winter, Season::Spring] {
            for icons in [IconSet::Nerd, IconSet::Plain] {
                for glyph in glyphs(season, icons).iter().chain(DRIFT) {
                    assert_eq!(Span::raw(*glyph).width(), 1, "{glyph}");
                }
            }
        }
    }

    // ui.md §10.3
    #[test]
    fn a_long_pause_skips_ahead_rather_than_working_out_every_frame() {
        let mut weather = weather(10, 6);
        weather.start(Duration::ZERO, Season::Spring);
        weather.tick(MINUTE, Some(idle(Season::Spring)));
        let sky = weather.sky.as_ref().expect("it plays");
        assert_eq!(sky.time, MINUTE);
        weather.tick(MINUTE + FRAME / 2, Some(idle(Season::Spring)));
        assert_eq!(weather.sky.as_ref().expect("it plays").time, MINUTE);
    }

    // ui.md §10.2
    #[test]
    fn a_resized_screen_starts_the_sky_over() {
        let mut weather = weather(10, 6);
        weather.start(Duration::ZERO, Season::Winter);
        let count = weather.sky.as_ref().expect("it plays").particles.len();
        assert_eq!(count, 2);
        drawn(&mut weather, 40, 12);
        let sky = weather.sky.as_ref().expect("it plays");
        assert_eq!((sky.particles.len(), sky.snow.len()), (12, 40));
    }
}
