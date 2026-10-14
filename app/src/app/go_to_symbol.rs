//! Go to Symbol: every area, project, section and task by its title, filtered as the user types, opening
//! the one chosen where it is, as VS Code's Go to Symbol does (FR-NAV-05, D-4). Hides which items
//! it lists and where each one opens.

use jiff::civil::Date;

use super::{
    Candidate, Picked, Picker, PickerInput, PickerView, State, TextEdit, container, navigator,
    view::{self, Shown, View},
    visits::Visit,
};
use crate::domain::{Document, ItemId, ItemKind};

/// An item Go to Symbol lists.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Symbol {
    /// The item.
    pub id: ItemId,
    /// What the item is: an area, a project, a section or a task.
    pub kind: ItemKind,
}

impl State {
    /// Go to Symbol, while it is open.
    #[must_use]
    pub fn go_to_symbol(&self) -> Option<PickerView<'_, Symbol>> {
        self.go_to_symbol.as_ref().map(Picker::view)
    }

    /// Opens Go to Symbol on every item it lists. No other picker is open then, since none takes
    /// this command.
    pub(super) fn open_go_to_symbol(&mut self) {
        self.go_to_symbol = Some(Picker::new(symbols(&self.document)));
    }

    /// Applies `input` to the open Go to Symbol: choosing an item closes it and shows the item
    /// where it is in the pane, or in a new pane (FR-PANE-02). Nothing happens while it is closed.
    pub(super) fn go_to_symbol_input(&mut self, input: PickerInput) {
        let Some(picked) = self
            .go_to_symbol
            .as_mut()
            .and_then(|picker| picker.input(input))
        else {
            return;
        };
        self.go_to_symbol = None;
        let Picked::Chosen(symbol) = picked else {
            return;
        };
        let visit = visit(&self.document, self.today, symbol);
        if input == PickerInput::ChooseInNewPane {
            self.open_pane(visit);
        } else if visit.shown == self.pane().shown {
            let (document, today) = (&self.document, self.today);
            self.pane.arrive(document, today, visit);
        } else {
            self.show(visit.shown, visit.focused);
        }
    }
}

/// Go to Symbol over `document` as [`State::open_go_to_symbol`] opens it, with `query` typed
/// again, as after a reload, which lists the items anew.
pub(super) fn reopen(document: &Document, query: &str) -> Picker<Symbol> {
    let mut picker = Picker::new(symbols(document));
    for c in query.chars() {
        picker.input(PickerInput::Edit(TextEdit::Insert(c)));
    }
    picker
}

/// Every area, project and section, then every task, each in the order of the file, so that an empty
/// query lists the places first. Each matches by its title and is told apart by the titles above
/// it. Items in the Trash and completed ones are left out with what they hold, as the Navigator
/// leaves them out (FR-ORG-09).
fn symbols(document: &Document) -> Vec<Candidate<Symbol>> {
    let mut symbols = Vec::new();
    let tops = std::iter::once(document.inbox()).chain(document.top_level().iter().copied());
    for id in tops {
        list(document, id, &mut Vec::new(), &mut symbols);
    }
    symbols.sort_by_key(|candidate| candidate.value.kind == ItemKind::Task);
    symbols
}

/// Adds the symbol `id` is, if it is one, and those inside it to `symbols`, under the titles
/// `above`.
fn list<'a>(
    document: &'a Document,
    id: ItemId,
    above: &mut Vec<&'a str>,
    symbols: &mut Vec<Candidate<Symbol>>,
) {
    let Some(item) = navigator::shown(document, id) else {
        return;
    };
    if matches!(
        item.kind,
        ItemKind::Area | ItemKind::Project | ItemKind::Section | ItemKind::Task
    ) {
        symbols.push(Candidate {
            value: Symbol {
                id,
                kind: item.kind,
            },
            text: item.title.clone(),
            detail: above.join(" › "),
        });
    }
    above.push(&item.title);
    for &child in document.children(id) {
        list(document, child, above, symbols);
    }
    above.pop();
}

/// Where `symbol` opens in `document` on `today`: an area or a project shows itself, as the Navigator opens
/// it; a section shows its project, focused on its heading; a task shows the container listing
/// it, else the first view that lists it, as a scheduled task in the Inbox, focused on it. All
/// lists every task Go to Symbol does.
fn visit(document: &Document, today: Date, symbol: Symbol) -> Visit {
    let Symbol { id, kind } = symbol;
    let shown = match kind {
        ItemKind::Area | ItemKind::Project => {
            return Visit {
                shown: Shown::Container(id),
                focused: None,
            };
        }
        ItemKind::Section => Shown::Container(
            document
                .position(id)
                .and_then(|(parent, _)| parent)
                .expect("a section sits in an area or a project"),
        ),
        _ => container::listing(document, id).map_or_else(
            || {
                view::REGISTRY
                    .iter()
                    .map(|entry| Shown::View(entry.view))
                    .find(|&shown| {
                        view::rows(document, shown, today, false, "")
                            .iter()
                            .any(|row| row.id() == id)
                    })
                    .unwrap_or(Shown::View(View::All))
            },
            Shown::Container,
        ),
    };
    Visit {
        shown,
        focused: Some(id),
    }
}

#[cfg(test)]
mod tests {
    use jiff::civil::date;

    use super::*;
    use crate::{
        app::{Command, Go, Message, Opened, update},
        domain::{Completion, Item, Node, PlacementError},
    };
    use ItemKind::{Area, ChecklistItem, Project, Section, Task};

    const TODAY: Date = date(2026, 10, 14);

    /// A document with what each test goes to, and the ids of its items by title.
    struct Sample {
        document: Document,
        work: ItemId,
        site: ItemId,
        launch: ItemId,
        write: ItemId,
        recruiter: ItemId,
        later: ItemId,
    }

    fn add(
        document: &mut Document,
        parent: Option<ItemId>,
        kind: ItemKind,
        title: &str,
    ) -> Result<ItemId, PlacementError> {
        document.push(parent, Node::Item(Item::new(kind, title)))
    }

    fn sample() -> Result<Sample, PlacementError> {
        let mut document = Document::new();
        let inbox = document.inbox();
        add(&mut document, Some(inbox), Task, "Buy coffee")?;
        let mut later = Item::new(Task, "Plan the trip");
        later.start = Some(date(2026, 10, 20));
        let later = document.push(Some(inbox), Node::Item(later))?;
        let work = add(&mut document, None, Area, "Work")?;
        let recruiter = add(&mut document, Some(work), Task, "Answer the recruiter")?;
        let site = add(&mut document, Some(work), Project, "Site")?;
        let launch = add(&mut document, Some(site), Section, "Launch")?;
        let write = add(&mut document, Some(launch), Task, "Write the announcement")?;
        add(&mut document, Some(write), ChecklistItem, "Outline")?;
        let mut gone = Item::new(Task, "Gone");
        gone.deleted = Some(date(2026, 10, 1));
        document.push(Some(launch), Node::Item(gone))?;
        let mut done = Item::new(Project, "Done");
        done.completed = Some(Completion::Undated);
        let done = document.push(Some(work), Node::Item(done))?;
        add(&mut document, Some(done), Task, "Inside done")?;
        Ok(Sample {
            document,
            work,
            site,
            launch,
            write,
            recruiter,
            later,
        })
    }

    fn give(state: &mut State, message: Message) {
        update(state, message, TODAY);
    }

    /// Opens Go to Symbol, types `query` and gives `input`.
    fn go_to(state: &mut State, query: &str, input: PickerInput) {
        give(state, Message::Command(Command::GoToSymbol));
        for c in query.chars() {
            give(
                state,
                Message::GoToSymbol(PickerInput::Edit(TextEdit::Insert(c))),
            );
        }
        give(state, Message::GoToSymbol(input));
    }

    fn at(state: &State) -> (Shown, Option<ItemId>) {
        (state.shown(), state.selection().focused())
    }

    // FR-NAV-05
    #[test]
    fn lists_areas_projects_and_sections_then_tasks_each_in_file_order_under_the_titles_above_them()
    -> Result<(), PlacementError> {
        let sample = sample()?;
        let listed: Vec<(String, String)> = symbols(&sample.document)
            .into_iter()
            .map(|candidate| (candidate.text, candidate.detail))
            .collect();
        let entry = |text: &str, detail: &str| (text.to_owned(), detail.to_owned());
        assert_eq!(
            listed,
            [
                entry("Work", ""),
                entry("Site", "Work"),
                entry("Launch", "Work › Site"),
                entry("Buy coffee", "Inbox"),
                entry("Plan the trip", "Inbox"),
                entry("Answer the recruiter", "Work"),
                entry("Write the announcement", "Work › Site › Launch"),
            ],
            "no Inbox, checklist item, item in the Trash, completed item or what one holds"
        );
        Ok(())
    }

    // FR-NAV-05, FR-UI-09
    #[test]
    fn an_area_or_project_opens_itself_a_section_its_project_and_a_task_its_container_focused_on_it()
    -> Result<(), PlacementError> {
        let sample = sample()?;
        let mut state = State::new(sample.document, Opened::Demo, TODAY);
        go_to(&mut state, "work", PickerInput::Choose);
        assert_eq!(state.shown(), Shown::Container(sample.work));
        go_to(&mut state, "site", PickerInput::Choose);
        assert!(state.go_to_symbol().is_none(), "choosing closes it");
        assert_eq!(state.shown(), Shown::Container(sample.site));
        go_to(&mut state, "launch", PickerInput::Choose);
        assert_eq!(
            at(&state),
            (Shown::Container(sample.site), Some(sample.launch))
        );
        go_to(&mut state, "announcement", PickerInput::Choose);
        assert_eq!(
            at(&state),
            (Shown::Container(sample.site), Some(sample.write)),
            "a section's task is listed under its heading"
        );
        go_to(&mut state, "recruiter", PickerInput::Choose);
        assert_eq!(
            at(&state),
            (Shown::Container(sample.work), Some(sample.recruiter))
        );
        give(&mut state, Message::Command(Command::Go(Go::Back)));
        assert_eq!(
            state.shown(),
            Shown::Container(sample.site),
            "Back returns to where the pane was"
        );
        Ok(())
    }

    // FR-NAV-05, FR-VIEW-01
    #[test]
    fn a_task_no_container_lists_opens_in_the_first_view_that_does() -> Result<(), PlacementError> {
        let sample = sample()?;
        let mut state = State::new(sample.document, Opened::Demo, TODAY);
        go_to(&mut state, "trip", PickerInput::Choose);
        assert_eq!(
            at(&state),
            (Shown::View(View::Upcoming), Some(sample.later))
        );
        Ok(())
    }

    // FR-NAV-05, FR-PANE-02
    #[test]
    fn choosing_in_a_new_pane_opens_it_there() -> Result<(), PlacementError> {
        let sample = sample()?;
        let mut state = State::new(sample.document, Opened::Demo, TODAY);
        go_to(&mut state, "recruiter", PickerInput::ChooseInNewPane);
        assert_eq!(state.panes().count(), 2);
        assert_eq!(state.current_pane(), 1);
        assert_eq!(
            at(&state),
            (Shown::Container(sample.work), Some(sample.recruiter))
        );
        Ok(())
    }

    // FR-NAV-05
    #[test]
    fn an_item_in_the_container_shown_only_takes_the_focus() -> Result<(), PlacementError> {
        let sample = sample()?;
        let mut state = State::new(sample.document, Opened::Demo, TODAY);
        go_to(&mut state, "site", PickerInput::Choose);
        give(&mut state, Message::Command(Command::TogglePicked));
        give(
            &mut state,
            Message::Command(Command::Focus(crate::app::Focus::Last)),
        );
        go_to(&mut state, "launch", PickerInput::Choose);
        assert_eq!(
            at(&state),
            (Shown::Container(sample.site), Some(sample.launch))
        );
        assert_eq!(state.selection().items(&state.rows()), [sample.launch]);
        give(&mut state, Message::Command(Command::Go(Go::Back)));
        assert_eq!(
            state.shown(),
            Shown::Container(state.document().inbox()),
            "no visit to go back to is added"
        );
        Ok(())
    }

    // FR-NAV-05, FR-UI-09
    #[test]
    fn dismissing_or_choosing_nothing_goes_nowhere_and_the_navigator_takes_its_place()
    -> Result<(), PlacementError> {
        let sample = sample()?;
        let mut state = State::new(sample.document, Opened::Demo, TODAY);
        let before = state.clone();
        give(&mut state, Message::GoToSymbol(PickerInput::Choose));
        assert_eq!(state, before, "nothing happens while it is closed");
        go_to(&mut state, "zzzz", PickerInput::Choose);
        assert!(
            state.go_to_symbol().is_some(),
            "nothing matches, so nothing is chosen"
        );
        assert!(!state.takes(Command::GoToSymbol) && state.takes(Command::Navigator));
        give(&mut state, Message::GoToSymbol(PickerInput::Dismiss));
        assert!(state.go_to_symbol().is_none());
        assert_eq!(state.shown(), before.shown());
        give(&mut state, Message::Command(Command::GoToSymbol));
        give(&mut state, Message::Command(Command::Navigator));
        assert!(state.go_to_symbol().is_none() && state.navigator().is_some());
        Ok(())
    }

    // FR-NAV-05
    #[test]
    fn reopening_lists_the_items_anew_with_the_query_typed() -> Result<(), PlacementError> {
        let mut sample = sample()?;
        let picker = reopen(&sample.document, "wri");
        assert_eq!(picker.view().query.text(), "wri");
        let first = |picker: &Picker<Symbol>| {
            picker
                .view()
                .entries
                .iter()
                .next()
                .map(|entry| entry.value.id)
        };
        assert_eq!(first(&picker), Some(sample.write));
        add(&mut sample.document, None, Project, "Writing")?;
        let picker = reopen(&sample.document, "writing");
        assert_eq!(
            picker
                .view()
                .entries
                .iter()
                .map(|entry| entry.text.as_str())
                .collect::<Vec<_>>(),
            ["Writing"]
        );
        Ok(())
    }
}
