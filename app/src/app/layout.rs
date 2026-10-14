//! The pane layout as it is kept between sessions, for a frontend to keep with the file and give
//! back on the next launch (FR-PANE-03): the panes in order, each with what it shows, its focus,
//! its history, whether it shows completed items and its width, and which pane has the keys.
//! Items are kept by their text paths, which outlive the ids of one session, and restoring drops
//! what no longer matches
//! ([file-io.md §5](../../../specs/reference/taskpaper-terminal/specs/file-io.md#5-view-state-persistence)).
//! Hides how a pane is kept and found again.

use jiff::civil::Date;
use serde::{Deserialize, Serialize};

use super::{
    State, container,
    pane::Pane,
    text_path::TextPath,
    view::{Shown, View},
    visits::{Visit, Visits},
};
use crate::domain::{Document, ItemId};

/// The pane layout of the open document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Layout {
    /// The panes from left to right.
    panes: Vec<KeptPane>,
    /// The position of the pane with the keys.
    current: usize,
}

/// One pane of a [`Layout`]. Its tag filter and what it picks are not kept.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct KeptPane {
    /// Where it is.
    at: KeptVisit,
    /// The visits to go back to, latest last.
    back: Vec<KeptVisit>,
    /// The visits to go forward to, next last.
    forward: Vec<KeptVisit>,
    show_completed: bool,
    width: Option<u16>,
}

/// A [`Visit`] as kept.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct KeptVisit {
    shown: Place,
    focused: Option<TextPath>,
}

/// What a pane shows, as kept.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Place {
    Container(TextPath),
    View(View),
}

impl State {
    /// The pane layout, to keep with the document's file for the next launch (FR-PANE-03).
    #[must_use]
    pub fn layout(&self) -> Layout {
        Layout {
            panes: self
                .left
                .iter()
                .chain([&self.pane])
                .chain(&self.right)
                .map(|pane| KeptPane::of(&self.document, pane))
                .collect(),
            current: self.current_pane(),
        }
    }

    /// This state with the panes `layout` keeps, as far as they match the document: a pane whose
    /// container is not found shows the nearest one above it that opens, else the Inbox; a
    /// focus not found gives way to the first row, as in a new pane; a visit whose container is
    /// not found leaves the history. Without any pane, the panes stay as they are.
    #[must_use]
    pub fn with_layout(mut self, layout: &Layout) -> Self {
        let mut panes: Vec<_> = layout
            .panes
            .iter()
            .map(|kept| kept.restore(&self.document, self.today))
            .collect();
        let Some(last) = panes.len().checked_sub(1) else {
            return self;
        };
        self.right = panes.split_off(layout.current.min(last) + 1);
        if let Some(pane) = panes.pop() {
            self.pane = pane;
        }
        self.left = panes;
        self
    }
}

impl KeptPane {
    /// How `pane` in `document` is kept.
    fn of(document: &Document, pane: &Pane) -> Self {
        let kept = |visits: &[Visit]| {
            visits
                .iter()
                .map(|&visit| KeptVisit::of(document, visit))
                .collect()
        };
        let (back, forward) = pane.visits.parts();
        Self {
            at: KeptVisit::of(document, pane.visit()),
            back: kept(back),
            forward: kept(forward),
            show_completed: pane.show_completed,
            width: pane.width,
        }
    }

    /// The pane this is in `document` on `today`.
    fn restore(&self, document: &Document, today: Date) -> Pane {
        let mut pane = Pane::new(document, today, self.at.land(document), self.show_completed);
        let found = |visits: &[KeptVisit]| {
            visits
                .iter()
                .filter_map(|visit| visit.find(document))
                .collect()
        };
        pane.visits = Visits::from_parts(found(&self.back), found(&self.forward));
        pane.width = self.width;
        pane
    }
}

impl KeptVisit {
    /// How `visit` in `document` is kept.
    fn of(document: &Document, visit: Visit) -> Self {
        Self {
            shown: match visit.shown {
                Shown::Container(id) => Place::Container(TextPath::of(document, id)),
                Shown::View(view) => Place::View(view),
            },
            focused: visit.focused.map(|id| TextPath::of(document, id)),
        }
    }

    /// The visit this is in `document`, `None` when its container is not found exactly or does
    /// not open, a section reopening on its container (D-80); its focus where found exactly,
    /// else on such a section.
    fn find(&self, document: &Document) -> Option<Visit> {
        let (shown, at) = match &self.shown {
            Place::Container(path) => {
                let (id, at) = container::place(document, path.exact(document)?)?;
                (Shown::Container(id), at)
            }
            Place::View(view) => (Shown::View(*view), None),
        };
        Some(Visit {
            shown,
            focused: self.focused(document).or(at),
        })
    }

    /// Its focus in `document`, where found exactly.
    fn focused(&self, document: &Document) -> Option<ItemId> {
        self.focused.as_ref().and_then(|path| path.exact(document))
    }

    /// Where a pane kept at this visit lands in `document`: its container where found exactly,
    /// else the nearest one above it found, while that opens, a section reopening on its container
    /// (D-80), else the Inbox; its focus where found exactly, else on such a section.
    fn land(&self, document: &Document) -> Visit {
        let (shown, at) = match &self.shown {
            Place::Container(path) => {
                let (id, at) = path
                    .find(document)
                    .and_then(|(id, _)| container::place(document, id))
                    .unwrap_or((document.inbox(), None));
                (Shown::Container(id), at)
            }
            Place::View(view) => (Shown::View(*view), None),
        };
        Visit {
            shown,
            focused: self.focused(document).or(at),
        }
    }
}

#[cfg(test)]
mod tests {
    use jiff::civil::date;

    use super::*;
    use crate::{
        app::{Go, Opened, PickerInput, TextEdit},
        domain::{Item, ItemKind, Node, PlacementError},
    };

    const TODAY: Date = date(2026, 10, 5);

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    /// The Inbox holding `i`, and the area Work holding the task `loose` and the project Site,
    /// which holds the tasks `p` and `q`; with the ids of Work, Site and `q`.
    fn document() -> Result<(Document, [ItemId; 3]), PlacementError> {
        changed("Site", None)
    }

    /// The document of [`document`] with Site titled `site` and Work `deleted` then.
    fn changed(
        site: &str,
        deleted: Option<Date>,
    ) -> Result<(Document, [ItemId; 3]), PlacementError> {
        let mut document = Document::new();
        let add = |document: &mut Document, parent, kind, title| {
            document.push(parent, Node::Item(Item::new(kind, title)))
        };
        let inbox = document.inbox();
        add(&mut document, Some(inbox), ItemKind::Task, "i")?;
        let mut area = Item::new(ItemKind::Area, "Work");
        area.deleted = deleted;
        let work = document.push(None, Node::Item(area))?;
        add(&mut document, Some(work), ItemKind::Task, "loose")?;
        let site = add(&mut document, Some(work), ItemKind::Project, site)?;
        add(&mut document, Some(site), ItemKind::Task, "p")?;
        let q = add(&mut document, Some(site), ItemKind::Task, "q")?;
        Ok((document, [work, site, q]))
    }

    /// Opens `place` through the Navigator.
    fn navigate(state: &mut State, place: &str) {
        state.toggle_navigator();
        for c in place.chars() {
            state.navigator_input(PickerInput::Edit(TextEdit::Insert(c)));
        }
        state.navigator_input(PickerInput::Choose);
    }

    /// Each pane's place, focus, width and whether it shows completed items.
    fn panes(state: &State) -> Vec<(Shown, Option<ItemId>, Option<u16>, bool)> {
        state
            .left
            .iter()
            .chain([&state.pane])
            .chain(&state.right)
            .map(|pane| {
                (
                    pane.shown,
                    pane.selection.focused(),
                    pane.width,
                    pane.show_completed,
                )
            })
            .collect()
    }

    /// Site shown with `q` focused, and Today beside it showing completed items 40 columns
    /// wide, which went there from Site; the first pane has the keys.
    fn site_and_today(document: Document) -> State {
        let mut state = State::new(document, Opened::Demo, TODAY);
        navigate(&mut state, "Site");
        state.move_focus(crate::app::Focus::Next);
        state.open_in_new_pane();
        state.go(Go::View(View::Today));
        state.toggle_show_completed();
        state.set_pane_width(40);
        state.focus_pane(1);
        state
    }

    // FR-PANE-03
    #[test]
    fn every_pane_comes_back_with_its_place_focus_history_and_width() -> Result<(), PlacementError>
    {
        let (document, [_, site, q]) = document()?;
        let state = site_and_today(document.clone());
        let layout = state.layout();
        let mut restored = State::new(document, Opened::Demo, TODAY).with_layout(&layout);
        assert_eq!(
            panes(&restored),
            [
                (Shown::Container(site), Some(q), None, false),
                (Shown::View(View::Today), None, Some(40), true),
            ]
        );
        assert_eq!(restored.current_pane(), 0);
        assert_eq!(restored.layout(), layout);
        restored.go(Go::Back);
        assert_eq!(restored.container(), Some(restored.document().inbox()));
        restored.focus_pane(2);
        restored.go(Go::Back);
        assert_eq!(restored.pane.shown, Shown::Container(site));
        assert_eq!(restored.pane.selection.focused(), Some(q));
        Ok(())
    }

    // FR-PANE-03, FR-NAV-04
    #[test]
    fn the_visits_of_views_come_back_too() -> Result<(), PlacementError> {
        let (document, _) = document()?;
        let mut state = State::new(document.clone(), Opened::Demo, TODAY);
        state.go(Go::View(View::Upcoming));
        state.go(Go::Inbox);
        let mut restored = State::new(document, Opened::Demo, TODAY).with_layout(&state.layout());
        restored.go(Go::Back);
        assert_eq!(restored.shown(), Shown::View(View::Upcoming));
        Ok(())
    }

    // FR-PANE-03, F-FILE-06
    #[test]
    fn what_is_no_longer_found_gives_way() -> Result<(), PlacementError> {
        let (document, _) = document()?;
        let layout = site_and_today(document).layout();
        let (renamed, [work, ..]) = changed("Shop", None)?;
        let restored = State::new(renamed, Opened::Demo, TODAY).with_layout(&layout);
        let first = restored.rows().first().map(|row| row.id());
        assert_eq!(
            panes(&restored).first(),
            Some(&(Shown::Container(work), first, None, false)),
            "the nearest container above, focused as when it opens"
        );
        let (deleted, _) = changed("Site", Some(TODAY))?;
        let mut restored = State::new(deleted, Opened::Demo, TODAY).with_layout(&layout);
        let inbox = restored.document().inbox();
        assert_eq!(restored.container(), Some(inbox), "nothing above opens");
        restored.focus_pane(2);
        restored.go(Go::Back);
        assert_eq!(
            restored.pane.shown,
            Shown::View(View::Today),
            "the visit of a container gone leaves the history"
        );
        Ok(())
    }

    // FR-PANE-03
    #[test]
    fn a_layout_without_panes_leaves_them_and_a_current_past_the_last_is_the_last()
    -> Result<(), PlacementError> {
        let (document, _) = document()?;
        let state = State::new(document.clone(), Opened::Demo, TODAY);
        let empty = Layout {
            panes: Vec::new(),
            current: 0,
        };
        assert_eq!(state.clone().with_layout(&empty), state);
        let mut layout = site_and_today(document).layout();
        layout.current = 7;
        assert_eq!(state.with_layout(&layout).current_pane(), 1);
        Ok(())
    }

    // FR-PANE-03
    #[test]
    fn a_layout_is_kept_as_json_by_titles_and_view_names() -> TestResult {
        let (document, _) = document()?;
        let layout = site_and_today(document).layout();
        let json = serde_json::to_value(&layout)?;
        let site = serde_json::json!([
            {"kind": "area", "title": "Work", "nth": 0},
            {"kind": "project", "title": "Site", "nth": 0}
        ]);
        assert_eq!(json["panes"][0]["at"]["shown"]["container"], site);
        assert_eq!(json["panes"][1]["at"]["shown"]["view"], "today");
        assert_eq!(
            json["panes"][0]["back"][0]["shown"]["container"],
            serde_json::json!([{"kind": "inbox", "title": "Inbox", "nth": 0}])
        );
        let back: Layout = serde_json::from_value(json)?;
        assert_eq!(back, layout);
        Ok(())
    }
}
