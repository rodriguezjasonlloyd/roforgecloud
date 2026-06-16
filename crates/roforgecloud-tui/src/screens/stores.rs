use std::collections::HashSet;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState};
use ratatui::Frame;
use ratatui_which_key::Keymap;
use roforgecloud_core::opencloud::datastore::DataStoreInfo;

use crate::app::{Action, App, EntriesCreateField, PendingConfirm, Screen, TextField, TextFieldExt};
use crate::update::{Act, Category, Scope, bind, back_key, dispatch, handle_pending_confirm, handle_text_field_key, list_nav_key, quit_key};
use crate::ui::{HIGHLIGHT_STYLE, centered_rect_lines, field_box};

pub(crate) struct State {
    pub items: Vec<DataStoreInfo>,
    pub selected: usize,
    pub marked: HashSet<usize>,
    pub new_id: TextField,
    pub new_active: bool,
    pub data_store_id: String,
}

impl State {
    pub(crate) fn new() -> Self {
        Self {
            items: Vec::new(),
            selected: 0,
            marked: HashSet::new(),
            new_id: TextField::default(),
            new_active: false,
            data_store_id: String::new(),
        }
    }

    pub(crate) fn toggle_mark(&mut self) {
        if !self.items.is_empty() && !self.marked.remove(&self.selected) {
            self.marked.insert(self.selected);
        }
    }

    pub(crate) fn toggle_select_all(&mut self) {
        if self.items.is_empty() {
            return;
        }
        if self.marked.len() == self.items.len() {
            self.marked.clear();
        } else {
            self.marked = (0..self.items.len()).collect();
        }
    }
}

pub(crate) fn bind_keys(km: &mut Keymap<KeyEvent, Scope, Act, Category>) {
    bind(km, KeyCode::Enter, Act { desc: "open", handler: open }, Scope::Stores);
    bind(km, KeyCode::Char('l'), Act { desc: "open", handler: open }, Scope::Stores);
    bind(
        km,
        KeyCode::Char(' '),
        Act { desc: "select", handler: |app| { app.stores.toggle_mark(); None } },
        Scope::Stores,
    );
    bind(
        km,
        KeyCode::Char('a'),
        Act { desc: "select all", handler: |app| { app.stores.toggle_select_all(); None } },
        Scope::Stores,
    );
    bind(km, KeyCode::Char('r'), Act { desc: "refresh", handler: |_| Some(Action::LoadStores) }, Scope::Stores);
    bind(
        km,
        KeyCode::Char('c'),
        Act {
            desc: "create entry in new store",
            handler: |app| {
                app.stores.new_id.clear();
                app.stores.new_active = true;
                None
            },
        },
        Scope::Stores,
    );
    bind(km, KeyCode::Char('d'), Act { desc: "delete", handler: delete }, Scope::Stores);
    bind(km, KeyCode::Char('u'), Act { desc: "undelete", handler: undelete }, Scope::Stores);
}

pub(crate) fn handle_key(app: &mut App, code: KeyCode, _mods: KeyModifiers) -> Option<Action> {
    if app.stores.new_active {
        return handle_new_key(app, code);
    }

    if let Some(result) = handle_pending_confirm(app, code) {
        return result;
    }

    let len = app.stores.items.len();
    if let Some(result) = list_nav_key(code, &mut app.stores.selected, len) {
        return result;
    }
    if let Some(result) = back_key(code, app, Screen::UniverseChoice) {
        return result;
    }
    if let Some(result) = quit_key(code, app) {
        return result;
    }

    dispatch(app, Scope::Stores, code, KeyModifiers::empty())
}

pub(crate) fn draw(frame: &mut Frame, app: &App, area: Rect) {
    let items: Vec<ListItem> = app
        .stores
        .items
        .iter()
        .enumerate()
        .map(|(i, store)| {
            let mut spans = Vec::new();
            if !app.stores.marked.is_empty() {
                let marker = if app.stores.marked.contains(&i) { "[x] " } else { "[ ] " };
                spans.push(Span::raw(marker));
            }
            spans.push(Span::raw(store.id.clone()));
            if store.state.as_deref().is_some_and(|s| s != "ACTIVE") {
                spans.push(Span::styled("  [SCHEDULED DELETION]", Style::default().fg(Color::DarkGray)));
            }
            ListItem::new(Line::from(spans))
        })
        .collect();

    let universe = match app.universe_names.get(&app.universe_id) {
        Some(name) => format!("{} ({name})", app.universe_id),
        None => app.universe_id.to_string(),
    };
    let base_title = format!("Data Stores (universe {universe})");
    let title = if app.stores.marked.is_empty() {
        base_title
    } else {
        format!("{base_title} ({} selected)", app.stores.marked.len())
    };

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(title))
        .highlight_style(HIGHLIGHT_STYLE);

    let mut state = ListState::default();
    if !app.stores.items.is_empty() {
        state.select(Some(app.stores.selected));
    }
    frame.render_stateful_widget(list, area, &mut state);

    if app.stores.new_active {
        draw_new_popup(frame, app, area);
    }
}

fn draw_new_popup(frame: &mut Frame, app: &App, area: Rect) {
    let popup = centered_rect_lines(50, 5, area);
    frame.render_widget(Clear, popup);
    let block = Block::default().borders(Borders::ALL).title("Create entry in new store");
    let inner = block.inner(popup);
    frame.render_widget(block, popup);
    field_box(frame, inner, "Data Store ID", &app.stores.new_id, true);
}

fn open(app: &mut App) -> Option<Action> {
    let store = app.stores.items.get(app.stores.selected)?;
    if store.state.as_deref().is_some_and(|s| s != "ACTIVE") {
        return None;
    }
    app.stores.data_store_id = store.id.clone();
    app.entries.next_page_token = None;
    app.screen = Screen::Entries;
    Some(Action::LoadEntries)
}

fn delete(app: &mut App) -> Option<Action> {
    if !app.stores.marked.is_empty() {
        app.arm_confirm(PendingConfirm::BulkDeleteStores);
        return None;
    }
    let store = app.stores.items.get(app.stores.selected)?;
    if store.state.as_deref().is_some_and(|s| s != "ACTIVE") {
        return None;
    }
    app.arm_confirm(PendingConfirm::DeleteStore);
    None
}

fn undelete(app: &mut App) -> Option<Action> {
    if !app.stores.marked.is_empty() {
        app.arm_confirm(PendingConfirm::BulkUndeleteStores);
        return None;
    }
    let store = app.stores.items.get(app.stores.selected)?;
    if store.state.as_deref() != Some("ACTIVE") {
        return Some(Action::UndeleteDataStore);
    }
    None
}

fn handle_new_key(app: &mut App, code: KeyCode) -> Option<Action> {
    match code {
        KeyCode::Enter => {
            let id = app.stores.new_id.get_value().trim().to_string();
            if id.is_empty() {
                return None;
            }
            app.stores.new_active = false;
            app.stores.data_store_id = id;
            app.entries.items.clear();
            app.entries.selected = 0;
            app.entries.next_page_token = None;
            app.entries.page_tokens = vec![None];
            app.entries.marked.clear();
            app.entries.search.clear();
            app.entries.create_id.clear();
            app.entries.create_value.clear();
            app.entries.create_field = EntriesCreateField::Id;
            app.entries.create_active = true;
            app.status.clear();
            app.screen = Screen::Entries;
            None
        }
        KeyCode::Esc => {
            app.stores.new_active = false;
            app.status.clear();
            None
        }
        _ => {
            handle_text_field_key(&mut app.stores.new_id, code, |_| true);
            None
        }
    }
}
