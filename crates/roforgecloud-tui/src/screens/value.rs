use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::Rect;
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Frame;
use ratatui_which_key::Keymap;

use crate::app::{Action, App, PendingConfirm, Screen, ValueSource};
use crate::update::{Act, Category, Scope, bind, back_key, dispatch, handle_pending_confirm, handle_text_field_key, handle_tree_key, quit_key};
use crate::ui::draw_tree;

pub(crate) struct State {
    pub title: String,
    pub text: String,
    pub revision: Option<String>,
    pub scroll: u16,
    pub viewport_height: u16,
    pub edit_text: String,
    pub source: ValueSource,
}

impl State {
    pub(crate) fn new() -> Self {
        Self {
            title: String::new(),
            text: String::new(),
            revision: None,
            scroll: 0,
            viewport_height: 0,
            edit_text: String::new(),
            source: ValueSource::DataStore,
        }
    }

    pub(crate) fn max_scroll(&self) -> u16 {
        let total_lines = self.text.lines().count() as u16;
        total_lines.saturating_sub(self.viewport_height)
    }
}

pub(crate) fn bind_keys(km: &mut Keymap<KeyEvent, Scope, Act, Category>) {
    bind(km, KeyCode::Char('r'), Act { desc: "refresh", handler: |_| Some(Action::LoadValue) }, Scope::Value);
    bind(
        km,
        KeyCode::Enter,
        Act { desc: "tree edit", handler: |app| { app.enter_tree_mode(); None } },
        Scope::Value,
    );
    bind(
        km,
        KeyCode::Char('l'),
        Act { desc: "tree edit", handler: |app| { app.enter_tree_mode(); None } },
        Scope::Value,
    );
    bind(km, KeyCode::Char('e'), Act { desc: "edit in $EDITOR", handler: |_| Some(Action::EditValueExternal) }, Scope::Value);
    bind(
        km,
        KeyCode::Char('d'),
        Act {
            desc: "delete",
            handler: |app| {
                let pending = match app.value.source {
                    ValueSource::DataStore => PendingConfirm::DeleteEntry,
                    ValueSource::MemoryStoreSortedMap => PendingConfirm::DeleteMemoryItem,
                };
                app.arm_confirm(pending);
                None
            },
        },
        Scope::Value,
    );
    bind(
        km,
        KeyCode::Char('t'),
        Act {
            desc: "edit ttl",
            handler: |app| {
                if app.value.source != ValueSource::MemoryStoreSortedMap {
                    return None;
                }
                app.memory_entries.ttl_edit.set(app.memory_item_ttl_seconds.to_string());
                app.memory_entries.ttl_editing = true;
                None
            },
        },
        Scope::Value,
    );
    bind(km, KeyCode::Up, Act { desc: "scroll up", handler: scroll_up }, Scope::Value);
    bind(km, KeyCode::Char('k'), Act { desc: "scroll up", handler: scroll_up }, Scope::Value);
    bind(km, KeyCode::Down, Act { desc: "scroll down", handler: scroll_down }, Scope::Value);
    bind(km, KeyCode::Char('j'), Act { desc: "scroll down", handler: scroll_down }, Scope::Value);
    bind(
        km,
        KeyCode::PageUp,
        Act { desc: "scroll up x10", handler: |app| { app.value.scroll = app.value.scroll.saturating_sub(10); None } },
        Scope::Value,
    );
    bind(
        km,
        KeyCode::PageDown,
        Act {
            desc: "scroll down x10",
            handler: |app| {
                let max = app.value.max_scroll();
                app.value.scroll = (app.value.scroll + 10).min(max);
                None
            },
        },
        Scope::Value,
    );
}

pub(crate) fn handle_key(app: &mut App, code: KeyCode, modifiers: KeyModifiers) -> Option<Action> {
    if app.tree_editor.is_some() {
        return handle_tree_key(app, code, modifiers);
    }

    if app.memory_entries.ttl_editing {
        return match code {
            KeyCode::Enter => Some(Action::SaveMemoryTtl),
            KeyCode::Esc => {
                app.memory_entries.ttl_editing = false;
                app.status.clear();
                None
            }
            _ => {
                handle_text_field_key(&mut app.memory_entries.ttl_edit, code, |c| c.is_ascii_digit());
                None
            }
        };
    }

    if let Some(result) = handle_pending_confirm(app, code) {
        return result;
    }
    if let Some(result) = quit_key(code, app) {
        return result;
    }
    let back_screen = match app.value.source {
        ValueSource::DataStore => Screen::Entries,
        ValueSource::MemoryStoreSortedMap => Screen::MemoryStoreEntries,
    };
    if let Some(result) = back_key(code, app, back_screen) {
        return result;
    }

    dispatch(app, Scope::Value, code, modifiers)
}

pub(crate) fn draw(frame: &mut Frame, app: &App, area: Rect) {
    if app.tree_editor.is_some() {
        draw_tree(frame, app, area);
        return;
    }

    let paragraph = Paragraph::new(crate::json_highlight::highlight(&app.value.text))
        .block(Block::default().borders(Borders::ALL).title(app.value.title.clone()))
        .wrap(Wrap { trim: false })
        .scroll((app.value.scroll, 0));

    frame.render_widget(paragraph, area);
}

fn scroll_up(app: &mut App) -> Option<Action> {
    app.value.scroll = app.value.scroll.saturating_sub(1);
    None
}

fn scroll_down(app: &mut App) -> Option<Action> {
    let max = app.value.max_scroll();
    app.value.scroll = (app.value.scroll + 1).min(max);
    None
}
