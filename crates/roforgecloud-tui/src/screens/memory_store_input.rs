use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::widgets::{Block, Borders};
use ratatui::Frame;

use crate::app::{Action, App, Screen, TextField};
use crate::update;

pub(crate) struct State {
    pub id: String,
    pub input: TextField,
}

impl State {
    pub(crate) fn new() -> Self {
        Self {
            id: String::new(),
            input: TextField::default(),
        }
    }

    pub(crate) fn reset(&mut self) {
        self.input.clear();
    }
}

pub(crate) fn handle_key(app: &mut App, code: KeyCode, _mods: KeyModifiers) -> Option<Action> {
    match code {
        KeyCode::Esc => {
            app.screen = Screen::UniverseChoice;
            app.status.clear();
            None
        }
        KeyCode::Backspace if app.memory_store_input.input.value.is_empty() => {
            app.screen = Screen::UniverseChoice;
            app.status.clear();
            None
        }
        KeyCode::Enter => {
            if app.memory_store_input.input.value.is_empty() {
                return None;
            }
            app.memory_store_input.id = app.memory_store_input.input.value.clone();
            app.memory_entries.next_page_token = None;
            app.screen = Screen::MemoryStoreEntries;
            Some(Action::LoadMemoryItems)
        }
        _ => {
            update::handle_text_field_key(&mut app.memory_store_input.input, code, |_| true);
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
        .title(format!("Memory Store (universe {universe})"));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(inner);

    crate::ui::field_box(frame, rows[0], "Sorted Map Name", &app.memory_store_input.input, true);
}
