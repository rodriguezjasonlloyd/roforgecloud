use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::widgets::{Block, Borders};
use ratatui::Frame;

use crate::app::{Action, App, Screen, TextField};
use crate::update;

pub(crate) struct State {
    pub field: TextField,
}

impl State {
    pub(crate) fn new() -> Self {
        Self { field: TextField::default() }
    }
}

pub(crate) fn handle_key(app: &mut App, code: KeyCode, _mods: KeyModifiers) -> Option<Action> {
    match code {
        KeyCode::Backspace if app.universe_input.field.value.is_empty() => {
            app.screen = Screen::UniverseChoice;
            app.status.clear();
            None
        }
        KeyCode::Esc => {
            app.screen = Screen::UniverseChoice;
            app.status.clear();
            None
        }
        KeyCode::Enter => {
            let Ok(id) = app.universe_input.field.value.parse::<u64>() else {
                return None;
            };
            app.universe_id = id;
            update::enter_service(app)
        }
        _ => {
            update::handle_text_field_key(&mut app.universe_input.field, code, |c| c.is_ascii_digit());
            None
        }
    }
}

pub(crate) fn draw(frame: &mut Frame, app: &App, area: Rect) {
    let block = Block::default().borders(Borders::ALL).title("roforgecloud");
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(inner);

    crate::ui::field_box(frame, rows[0], "Universe ID", &app.universe_input.field, true);
}
