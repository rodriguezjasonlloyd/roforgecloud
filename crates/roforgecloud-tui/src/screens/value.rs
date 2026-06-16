use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::Rect;
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Frame;
use ratatui_which_key::Keymap;

use crate::app::{Action, App, PendingConfirm, Screen, TextFieldExt, ValueSource};
use crate::update::{Act, Category, Scope, bind, bind_quit, dispatch, handle_pending_confirm, handle_text_field_key, handle_tree_key};
use crate::json_highlight;
use crate::ui::{breadcrumb, draw_tree, universe_label};

pub(crate) struct State {
    pub title: String,
    pub scope: String,
    pub expire: String,
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
            scope: String::new(),
            expire: String::new(),
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
    for scope in [Scope::DataStoreValue, Scope::MemoryStoreValue] {
        bind_quit(km, scope.clone());
        bind(km, KeyCode::Char('h'), Act { desc: "back", handler: |app| {
            app.screen = match app.value.source {
                ValueSource::DataStore => Screen::Entries,
                ValueSource::MemoryStoreSortedMap => Screen::MemoryStoreEntries,
            };
            app.status.clear();
            None
        }}, scope.clone());
        bind(km, KeyCode::Char('r'), Act { desc: "refresh", handler: |_| Some(Action::LoadValue) }, scope.clone());
        km.describe_group_for_scope("e", "edit", scope.clone());
        km.bind("et", Act { desc: "tree edit", handler: |app| { app.enter_tree_mode(); None } }, Category::General, scope.clone());
        km.bind("ee", Act { desc: "edit in $EDITOR", handler: |_| Some(Action::EditValueExternal) }, Category::General, scope.clone());
        bind(km, KeyCode::Char('d'), Act {
            desc: "delete",
            handler: |app| {
                let pending = match app.value.source {
                    ValueSource::DataStore => PendingConfirm::DeleteEntry,
                    ValueSource::MemoryStoreSortedMap => PendingConfirm::DeleteMemoryItem,
                };
                app.arm_confirm(pending);
                None
            },
        }, scope.clone());
        bind(km, KeyCode::Up, Act { desc: "scroll up", handler: scroll_up }, scope.clone());
        bind(km, KeyCode::Char('k'), Act { desc: "scroll up", handler: scroll_up }, scope.clone());
        bind(km, KeyCode::Down, Act { desc: "scroll down", handler: scroll_down }, scope.clone());
        bind(km, KeyCode::Char('j'), Act { desc: "scroll down", handler: scroll_down }, scope.clone());
        bind(km, KeyCode::PageUp, Act { desc: "scroll up x10", handler: |app| { app.value.scroll = app.value.scroll.saturating_sub(10); None } }, scope.clone());
        bind(km, KeyCode::PageDown, Act { desc: "scroll down x10", handler: |app| { let max = app.value.max_scroll(); app.value.scroll = (app.value.scroll + 10).min(max); None } }, scope.clone());
    }
    bind(km, KeyCode::Char('t'), Act {
        desc: "edit ttl",
        handler: |app| {
            app.memory_entries.ttl_edit.set_value(app.memory_item_ttl_seconds.to_string());
            app.memory_entries.ttl_editing = true;
            None
        },
    }, Scope::MemoryStoreValue);
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
    let scope = match app.value.source {
        ValueSource::DataStore => Scope::DataStoreValue,
        ValueSource::MemoryStoreSortedMap => Scope::MemoryStoreValue,
    };
    dispatch(app, scope, code, modifiers)
}

pub(crate) fn draw(frame: &mut Frame, app: &App, area: Rect) {
    if app.tree_editor.is_some() {
        draw_tree(frame, app, area);
        return;
    }

    let uni = universe_label(app);
    let title = match app.value.source {
        ValueSource::DataStore => {
            let suffix = if app.value.scope.is_empty() { None } else { Some(app.value.scope.as_str()) };
            breadcrumb(&[uni.as_str(), "data stores", &app.stores.data_store_id, &app.value.title], suffix)
        }
        ValueSource::MemoryStoreSortedMap => {
            let suffix = if app.value.expire.is_empty() { None } else { Some(app.value.expire.as_str()) };
            breadcrumb(&[uni.as_str(), "memory stores", &app.memory_store_input.id, &app.memory_item_editing_id], suffix)
        }
    };

    let paragraph = Paragraph::new(json_highlight::highlight(&app.value.text))
        .block(Block::default().borders(Borders::ALL).title(title))
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
