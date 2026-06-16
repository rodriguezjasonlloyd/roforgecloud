use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::widgets::{Block, Borders};
use ratatui::Frame;

use crate::app::{Action, App, OrderedInputField, Screen, TextField};
use crate::update;

pub(crate) struct State {
    pub store_id: TextField,
    pub scope: TextField,
    pub input_field: OrderedInputField,
}

impl State {
    pub(crate) fn new() -> Self {
        let mut scope = TextField::default();
        scope.set("global");
        Self {
            store_id: TextField::default(),
            scope,
            input_field: OrderedInputField::StoreId,
        }
    }

    pub(crate) fn reset(&mut self) {
        self.store_id.clear();
        self.scope.set("global");
        self.input_field = OrderedInputField::StoreId;
    }
}

pub(crate) fn handle_key(app: &mut App, code: KeyCode, _mods: KeyModifiers) -> Option<Action> {
    match code {
        KeyCode::Tab | KeyCode::BackTab => {
            app.ordered_store_input.input_field = match app.ordered_store_input.input_field {
                OrderedInputField::StoreId => OrderedInputField::Scope,
                OrderedInputField::Scope => OrderedInputField::StoreId,
            };
            None
        }
        KeyCode::Esc => {
            app.screen = Screen::UniverseChoice;
            app.status.clear();
            None
        }
        KeyCode::Backspace
            if app.ordered_store_input.input_field == OrderedInputField::StoreId
                && app.ordered_store_input.store_id.value.is_empty() =>
        {
            app.screen = Screen::UniverseChoice;
            app.status.clear();
            None
        }
        KeyCode::Enter => {
            if app.ordered_store_input.store_id.value.is_empty() {
                return None;
            }
            if app.ordered_store_input.scope.value.is_empty() {
                app.ordered_store_input.scope.set("global");
            }
            app.ordered_entries.next_page_token = None;
            app.screen = Screen::OrderedEntries;
            Some(Action::LoadOrderedEntries)
        }
        _ => {
            let field = match app.ordered_store_input.input_field {
                OrderedInputField::StoreId => &mut app.ordered_store_input.store_id,
                OrderedInputField::Scope => &mut app.ordered_store_input.scope,
            };
            update::handle_text_field_key(field, code, |_| true);
            None
        }
    }
}

pub(crate) fn draw(frame: &mut Frame, app: &App, area: Rect) {
    let universe = match app.universe_names.get(&app.universe_id) {
        Some(name) => format!("{} ({name})", app.universe_id),
        None => app.universe_id.to_string(),
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .title(format!("Ordered Data Stores (universe {universe})"));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Length(3), Constraint::Min(0)])
        .split(inner);

    let id_active = app.ordered_store_input.input_field == OrderedInputField::StoreId;
    let scope_active = app.ordered_store_input.input_field == OrderedInputField::Scope;

    crate::ui::field_box(frame, rows[0], "Ordered Data Store ID", &app.ordered_store_input.store_id, id_active);
    crate::ui::field_box(frame, rows[1], "Scope", &app.ordered_store_input.scope, scope_active);
}
