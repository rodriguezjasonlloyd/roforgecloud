use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::widgets::{Block, Borders};
use ratatui::Frame;

use crate::app::{Action, App, MessagingField, Screen, TextField};
use crate::ui;
use crate::update;

pub(crate) struct State {
    pub topic: TextField,
    pub message: TextField,
    pub field: MessagingField,
}

impl State {
    pub(crate) fn new() -> Self {
        Self {
            topic: TextField::default(),
            message: TextField::default(),
            field: MessagingField::Topic,
        }
    }

    pub(crate) fn reset(&mut self) {
        self.topic.clear();
        self.message.clear();
        self.field = MessagingField::Topic;
    }
}

pub(crate) fn handle_key(app: &mut App, code: KeyCode, _mods: KeyModifiers) -> Option<Action> {
    match code {
        KeyCode::Tab | KeyCode::BackTab => {
            app.messaging.field = match app.messaging.field {
                MessagingField::Topic => MessagingField::Message,
                MessagingField::Message => MessagingField::Topic,
            };
            None
        }
        KeyCode::Enter => Some(Action::PublishMessage),
        KeyCode::Esc => {
            app.screen = Screen::UniverseChoice;
            app.status.clear();
            None
        }
        _ => {
            let field = match app.messaging.field {
                MessagingField::Topic => &mut app.messaging.topic,
                MessagingField::Message => &mut app.messaging.message,
            };
            update::handle_text_field_key(field, code, |_| true);
            None
        }
    }
}

pub(crate) fn draw(frame: &mut Frame, app: &App, area: Rect) {
    let uni = ui::universe_label(app);
    let block = Block::default()
        .borders(Borders::ALL)
        .title(ui::breadcrumb(&[uni.as_str(), "messaging"], None));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(3)])
        .split(inner);

    let topic_active = app.messaging.field == MessagingField::Topic;
    let message_active = app.messaging.field == MessagingField::Message;

    ui::field_box(frame, rows[0], "Topic", &app.messaging.topic, topic_active);
    ui::field_paragraph_box(frame, rows[1], "Message", &app.messaging.message, message_active);
}
