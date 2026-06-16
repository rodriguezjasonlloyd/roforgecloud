use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;
use ratatui_which_key::Keymap;
use crossterm::event::KeyEvent;

use crate::app::{Action, App, PendingConfirm, Screen};
use crate::update::{self, Act, Category, Scope, bind, dispatch, back_key, quit_key, handle_pending_confirm};

pub(crate) struct State {
    pub title: String,
    pub value: f64,
    pub edit: String,
    pub editing: bool,
    pub increment_edit: String,
    pub increment_editing: bool,
}

impl State {
    pub(crate) fn new() -> Self {
        Self {
            title: String::new(),
            value: 0.0,
            edit: String::new(),
            editing: false,
            increment_edit: String::new(),
            increment_editing: false,
        }
    }


}

pub(crate) fn bind_keys(km: &mut Keymap<KeyEvent, Scope, Act, Category>) {
    bind(km, KeyCode::Char('r'), Act { desc: "refresh", handler: |_| Some(Action::LoadOrderedValue) }, Scope::OrderedValue);
    bind(km, KeyCode::Enter, Act { desc: "edit", handler: value_edit }, Scope::OrderedValue);
    bind(km, KeyCode::Char('e'), Act { desc: "edit", handler: value_edit }, Scope::OrderedValue);
    bind(
        km,
        KeyCode::Char('i'),
        Act {
            desc: "increment",
            handler: |app| {
                app.ordered_value.increment_edit.clear();
                app.ordered_value.increment_editing = true;
                None
            },
        },
        Scope::OrderedValue,
    );
    bind(
        km,
        KeyCode::Char('d'),
        Act {
            desc: "delete",
            handler: |app| {
                app.arm_confirm(PendingConfirm::DeleteOrderedEntry);
                None
            },
        },
        Scope::OrderedValue,
    );
}

pub(crate) fn handle_key(app: &mut App, code: KeyCode, _mods: KeyModifiers) -> Option<Action> {
    if app.ordered_value.editing {
        match code {
            KeyCode::Esc => {
                app.ordered_value.editing = false;
                app.status.clear();
            }
            KeyCode::Enter => return Some(Action::SaveOrderedValue),
            KeyCode::Backspace => {
                app.ordered_value.edit.pop();
            }
            KeyCode::Char(c) if update::is_numeric_input_char(c) => app.ordered_value.edit.push(c),
            _ => {}
        }
        return None;
    }

    if app.ordered_value.increment_editing {
        match code {
            KeyCode::Esc => {
                app.ordered_value.increment_editing = false;
                app.status.clear();
            }
            KeyCode::Enter => return Some(Action::IncrementOrderedEntry),
            KeyCode::Backspace => {
                app.ordered_value.increment_edit.pop();
            }
            KeyCode::Char(c) if update::is_numeric_input_char(c) => app.ordered_value.increment_edit.push(c),
            _ => {}
        }
        return None;
    }

    if let Some(result) = handle_pending_confirm(app, code) {
        return result;
    }
    if let Some(result) = quit_key(code, app) {
        return result;
    }
    if let Some(result) = back_key(code, app, Screen::OrderedEntries) {
        return result;
    }

    dispatch(app, Scope::OrderedValue, code, KeyModifiers::empty())
}

pub(crate) fn draw(frame: &mut Frame, app: &App, area: Rect) {
    let mut lines = vec![Line::from("")];

    if app.ordered_value.editing {
        lines.push(Line::from(vec![
            Span::raw("value: "),
            Span::styled(
                format!("{}█", app.ordered_value.edit),
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::UNDERLINED),
            ),
        ]));
    } else {
        lines.push(Line::from(Span::styled(
            app.ordered_value.value.to_string(),
            Style::default().fg(Color::Yellow),
        )));
    }

    if app.ordered_value.increment_editing {
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::raw("increment by: "),
            Span::styled(
                format!("{}█", app.ordered_value.increment_edit),
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::UNDERLINED),
            ),
        ]));
    }

    let paragraph = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .title(app.ordered_value.title.clone()),
    );
    frame.render_widget(paragraph, area);
}

fn value_edit(app: &mut App) -> Option<Action> {
    app.ordered_value.edit = app.ordered_value.value.to_string();
    app.ordered_value.editing = true;
    None
}
