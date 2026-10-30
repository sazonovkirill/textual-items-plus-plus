# Website

- Status: Draft
- Last updated: 2026-10-18

What the textual-items++ product website holds and how it is built. The site lives in `site/` at the repository root (owner, 2026-10-18). It describes the app; where it and the [functional requirements](functional-requirements.md) or the app's own behavior disagree, they win and the site is a bug.

textual-items++ is free and open source under the MIT license ([LICENSE](../LICENSE)), and the repository goes public before the site launches. The site shows one install command, `cargo install --git https://github.com/sazonovkirill/textual-items-plus-plus`, which installs the `tipp` binary (owner, 2026-10-18).

Priorities use the scale of the functional requirements. IDs are stable; reference them from tasks and commits.

## Goals

1. **Interest a newcomer** in a few minutes: the landing page shows what the app is and why it is worth trying.
2. **Teach the value** in 10 to 15 minutes of reading: the Tutorial.
3. **Answer every question** about a feature, key or setting: the Reference.

## W-SITE · Site

| ID | Requirement | Priority |
|---|---|---|
| W-SITE-01 | A static site in `site/`, built with [Zola](https://www.getzola.org), pinned to one version that `site.sh` downloads with a checksum into an ignored cache folder, into plain HTML, CSS and a little JavaScript that any static host serves with no server code. Pages work with JavaScript off, except for animations. | Must |
| W-SITE-02 | Pages: Home (landing), Tutorial, Reference, Pricing. Every page has the same header with links to all four and to the GitHub repository, and the same footer. | Must |
| W-SITE-03 | One script, run from the repository root: `bash site/site.sh build` renders the site into an ignored output folder, `serve` previews it with live reload and prints the address, `check` validates it (W-SITE-07, W-REF-04), `reference` regenerates the Reference data from the app (W-REF-03) and `shoot [scene,...]` regenerates the screenshots (W-SHOT-03). Without an argument it prints its usage. | Must |
| W-SITE-04 | Tutorial and Reference are written in Markdown, one file per chapter or topic. | Must |
| W-SITE-05 | The style matches the app: dark by default, the palette the screenshots render Terminal Dark in (Tomorrow Night) as accent colors, a monospace font for keys and code. The site follows the system's light or dark preference with a light palette of its own. Colors are defined once, as CSS variables. | Must |
| W-SITE-06 | Readable at 360 px wide, with no horizontal page scroll; screenshots scale down. | Must |
| W-SITE-07 | `check` builds the site and fails on a broken internal link, a missing image or an anchor that does not exist. | Must |
| W-SITE-08 | Accessible: semantic headings, alt text on every screenshot saying what it shows, visible keyboard focus, motion reduced when the system asks for it. | Must |
| W-SITE-09 | No fonts, scripts or analytics from third-party hosts; images have their width and height set and are lazily loaded below the fold. | Must |
| W-SITE-10 | Published to a public host with a real base URL: GitHub Pages at `https://sazonovkirill.github.io/textual-items-plus-plus/`, deployed by `.github/workflows/site.yml` on every push to `main` that passes `site.sh check` (owner, 2026-10-30). A custom domain is for later. | Must |

## W-SHOT · Screenshots

| ID | Requirement | Priority |
|---|---|---|
| W-SHOT-01 | Every screenshot is taken from the real `tipp` release build by `site/screenshots/shoot.py`: it starts the app on a demo `.taskpaper` file in a throwaway home with its own settings file, sends the scene's keys in tmux, captures the screen with `tmux capture-pane -e`, turns the cells into HTML in JetBrains Mono Nerd Font and renders a PNG with Playwright's Chromium. No screenshot is edited by hand. | Must |
| W-SHOT-02 | Scenes are lists of named steps (keys, typed text, waits) in `site/screenshots/scenes/*.json`; adding a screenshot means adding a scene. Dates in the demo files are written `{today+N}` and filled in relative to the day the script runs, so date chips always read "today", "tomorrow" and so on. | Must |
| W-SHOT-03 | One command, `bash site/site.sh shoot`, regenerates every screenshot after a UI change. The images are committed, so building the site needs neither the app nor the capture tools. | Must |
| W-SHOT-04 | Rendered at 2x in the default theme, Terminal Dark, with the terminal palette set to Tomorrow Night, inside a window frame unless a scene asks for none. The font is downloaded at a pinned version and checksum into the ignored cache, not committed. | Must |
| W-SHOT-05 | Short animated WebP clips of a step sequence, for features that are about motion, such as Quick Add and completing a task. | Should |
| W-SHOT-06 | A screenshot in every built-in theme for the Reference's appearance page. | Should |
| W-SHOT-07 | The demo file reads like a real person's work and fills the screen, never "foo", "test" or lorem ipsum. | Must |

## W-HOME · Landing page

| ID | Requirement | Priority |
|---|---|---|
| W-HOME-01 | Opens with the product name, a one-line pitch, the install command with a Copy button, links to the Tutorial and the repository, and a large hero screenshot of realistic use. | Must |
| W-HOME-02 | One long scrolling page in the manner of Apple's product pages: full-width sections, each one idea, one short headline, a sentence or two and one screenshot or clip, with its two or three key shortcuts as key caps where they help. | Must |
| W-HOME-03 | The sections, in order: Things 3's calm structure in the terminal (areas, projects, sections, the Inbox); one plain-text TaskPaper file you own; Today, Upcoming and the other views; keyboard first, with mnemonic keys, the palette and the shortcuts panel; Quick Add with dates in plain words; notes, checklists and comments in the attachments panel; finding anything (Navigator, Go to Symbol, search); panes side by side; themes, animations and the weather; works wherever a terminal does. | Must |
| W-HOME-04 | Sections reveal as they scroll into view (fade and slide, screenshots that grow). Without JavaScript, or with reduced motion, everything is shown statically. | Must |
| W-HOME-05 | Ends with a call to action: install, read the Tutorial, star on GitHub. | Must |

## W-PRICE · Pricing page

| ID | Requirement | Priority |
|---|---|---|
| W-PRICE-01 | A joke pricing page laid out like a real SaaS pricing table, with three plans side by side: Basic, Pro and Pro Max. | Must |
| W-PRICE-02 | Each plan appears to add features to the one before ("Everything in Basic, plus:"). Every feature listed is one the app really has, and all three plans have all of them. | Must |
| W-PRICE-03 | The page pushes the reader toward Pro, as real pricing pages do: raised, larger, highlighted, badged "Most popular", the only filled button and "★ Chosen by 100% of people who chose it". | Must |
| W-PRICE-04 | Every plan costs $0 / month ($0 annually), with a Monthly / Annually toggle and a "Save 0%" badge that change only the period label. | Must |
| W-PRICE-05 | Dry jokes in the voice of real SaaS copy, and an honest FAQ under the table: it is free, with no account, telemetry or subscription; the MIT license; the install command; and why there is a pricing page at all. | Must |

## W-TUT · Tutorial

| ID | Requirement | Priority |
|---|---|---|
| W-TUT-01 | Readable start to finish in 10 to 15 minutes: about 2,500 to 3,500 words, plus screenshots, in 8 to 12 short chapters. | Must |
| W-TUT-02 | Opens with why: why one plain-text TaskPaper file beats an app's database, why Things 3's structure of areas, projects and sections keeps a list calm, and why keys beat a mouse for a list you visit all day. | Must |
| W-TUT-03 | Goes from simple to advanced, each chapter building on the last: first launch, moving around, adding and editing, completing and undo, organizing, dates and Today, tags and flags, the attachments panel, finding things, panes and settings. | Must |
| W-TUT-04 | Teaches by doing: every chapter uses the downloadable sample file, `site/static/tutorial/tutorial.taskpaper`, says what to press and what the reader sees, with a screenshot of that state taken from the same file. Every step is checked in the real app. It does not cover every feature; each chapter links to the Reference for the rest. | Must |
| W-TUT-05 | Ends with where to go next: the Reference, the `?` shortcuts panel, the Keymap window and the command palette. | Must |

## W-REF · Reference

| ID | Requirement | Priority |
|---|---|---|
| W-REF-01 | Covers every user-visible feature, every key binding and every setting of the app. A feature, key or setting the app has and the Reference lacks is a bug. | Must |
| W-REF-02 | Organized by topic, following the groups of the functional requirements, one page per topic: files and saving, organization, views and the sidebar, navigation, editing, selection and rearranging, dates, tags, flags and pins, attachments, search and finders, panes, keyboard and mouse, appearance, settings. | Must |
| W-REF-03 | The key and settings tables are generated by `site.sh reference` from the app itself: a small Rust program in `site/reference/` reads the command registry, the default keymap, every window's key table, the settings and the themes through the app crate's public API, and `site/scripts/reference.py` places each on its topic page from a hand-written map, `site/reference/topics.toml`, failing loudly on anything it cannot place or a name the app does not have. | Must |
| W-REF-04 | `check` fails when the generated data differs from what the app gives now, or when the app has a command, window or setting the built Reference does not render. | Must |
| W-REF-05 | Every page has a table of contents and stable anchors, so a key or setting can be linked to directly. | Must |
| W-REF-06 | A search box over the Reference, with the index built at site build time and no server. | Should |
| W-REF-07 | A printable cheat sheet of every default key, rendered from the same data, that prints on one or two pages. | Should |
| W-REF-08 | Where the app and the functional requirements disagree, the Reference follows the app, and the mismatch is reported to the owner. | Must |

## Out of scope

A blog, newsletter, analytics, comments, accounts, versioned docs for older releases and translations.

## Milestones

Built straight through and reported once, when the whole site is built (owner, 2026-10-18). Site changes need no independent review; their validation is `bash site/site.sh check` ([agent workflow](../docs/agent-workflow.md#the-product-website)).

| # | Milestone | Covers |
|---|---|---|
| W1 | **Foundations.** The `site/` skeleton, `site.sh`, the shared layout, header, footer and style in light and dark, the screenshot script with its demo file and first scenes, and the Pricing page. | W-SITE-01–09, W-SHOT-01–04, W-SHOT-07, W-PRICE |
| W2 | **Landing page.** The scrolling story, hero shot, scroll animations and the clips its sections need. | W-HOME, W-SHOT-05 |
| W3 | **Tutorial.** The chapters, the sample file and their screenshots. | W-TUT |
| W4 | **Reference.** Every topic page, the generated tables with their coverage check, the cheat sheet, the theme screenshots and search. | W-REF, W-SHOT-06 |
| W5 | **Ready to publish.** Page titles and descriptions, a social preview image and a 404 page; the LICENSE file is already in place. No hosting or deploy until the owner asks. | W-SITE-10 is left for later |
