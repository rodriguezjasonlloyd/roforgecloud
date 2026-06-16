use std::collections::{HashMap, HashSet};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState};
use ratatui::Frame;
use ratatui_which_key::Keymap;
use roforgecloud_core::opencloud::datastore::DataStoreEntryInfo;

use crate::app::{Action, App, EntriesCreateField, PendingConfirm, Screen, TextField, TextFieldExt, TreeTarget};
use crate::status;
use crate::user_lookup;
use crate::update::{
    Act, Category, Scope, bind, dispatch, handle_pending_confirm, handle_text_field_key,
    handle_tree_key, list_nav_key, quit_key,
};
use crate::ui::{HIGHLIGHT_STYLE, centered_rect, centered_rect_lines, draw_tree, field_box, field_paragraph_box};

pub(crate) struct State {
    pub items: Vec<DataStoreEntryInfo>,
    pub usernames: HashMap<u64, String>,
    pub username_rx: tokio::sync::mpsc::UnboundedReceiver<HashMap<u64, String>>,
    pub username_tx: tokio::sync::mpsc::UnboundedSender<HashMap<u64, String>>,
    pub http: reqwest::Client,
    pub selected: usize,
    pub next_page_token: Option<String>,
    pub page_tokens: Vec<Option<String>>,
    pub marked: HashSet<usize>,
    pub search: TextField,
    pub search_active: bool,
    pub create_id: TextField,
    pub create_value: TextField,
    pub create_field: EntriesCreateField,
    pub create_active: bool,
    pub create_choosing: bool,
}

impl State {
    pub(crate) fn new() -> Self {
        let (username_tx, username_rx) = tokio::sync::mpsc::unbounded_channel();
        Self {
            items: Vec::new(),
            usernames: HashMap::new(),
            username_rx,
            username_tx,
            http: reqwest::Client::new(),
            selected: 0,
            next_page_token: None,
            page_tokens: vec![None],
            marked: HashSet::new(),
            search: TextField::default(),
            search_active: false,
            create_id: TextField::default(),
            create_value: TextField::default(),
            create_field: EntriesCreateField::Id,
            create_active: false,
            create_choosing: false,
        }
    }

    pub(crate) fn visible_indices(&self) -> Vec<usize> {
        if self.search.get_value().is_empty() {
            return (0..self.items.len()).collect();
        }
        let needle = self.search.get_value().to_lowercase();
        self.items
            .iter()
            .enumerate()
            .filter(|(_, entry)| {
                if entry.id.to_lowercase().contains(&needle) {
                    return true;
                }
                user_lookup::extract_id(&entry.id)
                    .and_then(|id| self.usernames.get(&id))
                    .is_some_and(|name| name.to_lowercase().contains(&needle))
            })
            .map(|(i, _)| i)
            .collect()
    }

    pub(crate) fn current_index(&self) -> Option<usize> {
        self.visible_indices().get(self.selected).copied()
    }

    pub(crate) fn toggle_mark(&mut self) {
        if let Some(index) = self.current_index() {
            if !self.marked.remove(&index) {
                self.marked.insert(index);
            }
        }
    }

    pub(crate) fn toggle_select_all_visible(&mut self) {
        let visible = self.visible_indices();
        if visible.iter().all(|i| self.marked.contains(i)) {
            for i in &visible {
                self.marked.remove(i);
            }
        } else {
            self.marked.extend(visible);
        }
    }

    pub(crate) fn current_scope_key(&self) -> Option<(String, String)> {
        let entry = self.items.get(self.current_index()?)?;
        Some(match entry.id.split_once('/') {
            Some((scope, key)) => (scope.to_string(), key.to_string()),
            None => ("global".to_string(), entry.id.clone()),
        })
    }

    pub(crate) fn resolve_usernames(&self) {
        let ids: Vec<u64> = self
            .items
            .iter()
            .filter_map(|entry| user_lookup::extract_id(&entry.id))
            .filter(|id| !self.usernames.contains_key(id))
            .collect();
        if ids.is_empty() {
            return;
        }
        let client = self.http.clone();
        let tx = self.username_tx.clone();
        tokio::spawn(async move {
            if let Ok(resolved) = user_lookup::resolve_usernames(&client, &ids).await {
                let _ = tx.send(resolved);
            }
        });
    }
}

pub(crate) fn bind_keys(km: &mut Keymap<KeyEvent, Scope, Act, Category>) {
    bind(km, KeyCode::Char('n'), Act { desc: "next page", handler: |_| Some(Action::LoadNextEntriesPage) }, Scope::Entries);
    bind(km, KeyCode::Char('p'), Act { desc: "prev page", handler: |_| Some(Action::LoadPrevEntriesPage) }, Scope::Entries);
    bind(km, KeyCode::Char('r'), Act { desc: "refresh", handler: |_| Some(Action::RefreshEntries) }, Scope::Entries);
    bind(
        km,
        KeyCode::Char('/'),
        Act {
            desc: "search",
            handler: |app| {
                app.entries.search_active = true;
                app.status = status::loading_search("entries");
                Some(Action::LoadAllEntriesForSearch)
            },
        },
        Scope::Entries,
    );
    bind(
        km,
        KeyCode::Char('c'),
        Act { desc: "create", handler: |app| { app.entries.create_choosing = true; None } },
        Scope::Entries,
    );
    bind(
        km,
        KeyCode::Char(' '),
        Act { desc: "select", handler: |app| { app.entries.toggle_mark(); None } },
        Scope::Entries,
    );
    bind(
        km,
        KeyCode::Char('a'),
        Act { desc: "select all", handler: |app| { app.entries.toggle_select_all_visible(); None } },
        Scope::Entries,
    );
    bind(km, KeyCode::Char('d'), Act { desc: "delete", handler: delete }, Scope::Entries);
    bind(km, KeyCode::Enter, Act { desc: "view", handler: view }, Scope::Entries);
    bind(km, KeyCode::Char('l'), Act { desc: "view", handler: view }, Scope::Entries);
}

pub(crate) fn handle_key(app: &mut App, code: KeyCode, modifiers: KeyModifiers) -> Option<Action> {
    if app.tree_editor.is_some() {
        return handle_tree_key(app, code, modifiers);
    }

    if app.entries.create_active {
        return handle_create_key(app, code, modifiers);
    }

    if app.entries.create_choosing {
        app.entries.create_choosing = false;
        return match code {
            KeyCode::Char('n') => {
                app.entries.create_id.clear();
                app.entries.create_value.clear();
                app.entries.create_field = EntriesCreateField::Id;
                app.entries.create_active = true;
                None
            }
            KeyCode::Char('e') => Some(Action::CreateEntryExternal),
            _ => None,
        };
    }

    if app.entries.search_active {
        return match code {
            KeyCode::Enter | KeyCode::Esc => {
                app.entries.search_active = false;
                app.entries.search.clear();
                app.status.clear();
                Some(Action::RefreshEntries)
            }
            _ => {
                if handle_text_field_key(&mut app.entries.search, code, |_| true) {
                    app.entries.selected = 0;
                }
                None
            }
        };
    }

    if let Some(result) = handle_pending_confirm(app, code) {
        return result;
    }

    let visible = app.entries.visible_indices().len();

    if let Some(result) = list_nav_key(code, &mut app.entries.selected, visible) {
        return result;
    }
    if let Some(result) = quit_key(code, app) {
        return result;
    }

    if matches!(code, KeyCode::Esc | KeyCode::Backspace | KeyCode::Char('h')) {
        if !app.entries.search.get_value().is_empty() {
            app.entries.search.clear();
            app.entries.selected = 0;
            app.status.clear();
            return None;
        }
        app.screen = Screen::Stores;
        app.status.clear();
        return Some(Action::LoadStores);
    }

    dispatch(app, Scope::Entries, code, modifiers)
}

pub(crate) fn draw(frame: &mut Frame, app: &App, area: Rect) {
    let visible = app.entries.visible_indices();

    let items: Vec<ListItem> = visible
        .iter()
        .map(|&i| {
            let entry = &app.entries.items[i];
            let mut spans = Vec::new();
            if !app.entries.marked.is_empty() {
                let marker = if app.entries.marked.contains(&i) { "[x] " } else { "[ ] " };
                spans.push(Span::raw(marker));
            }
            spans.push(Span::raw(entry.id.clone()));
            if let Some(username) =
                user_lookup::extract_id(&entry.id).and_then(|id| app.entries.usernames.get(&id))
            {
                spans.push(Span::styled(
                    format!("  ({username})"),
                    Style::default().fg(ratatui::style::Color::DarkGray),
                ));
            }
            ListItem::new(Line::from(spans))
        })
        .collect();

    let store_label = match app.universe_names.get(&app.universe_id) {
        Some(name) => format!("{} (universe {} ({name}))", app.stores.data_store_id, app.universe_id),
        None => app.stores.data_store_id.clone(),
    };
    let title = if app.entries.search.get_value().is_empty() {
        if app.entries.marked.is_empty() {
            store_label
        } else {
            format!("{store_label} ({} selected)", app.entries.marked.len())
        }
    } else if app.entries.marked.is_empty() {
        format!("{store_label} (search: {})", app.entries.search.get_value())
    } else {
        format!(
            "{store_label} (search: {}, {} selected)",
            app.entries.search.get_value(),
            app.entries.marked.len()
        )
    };

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(title))
        .highlight_style(HIGHLIGHT_STYLE);

    let mut state = ListState::default();
    if !visible.is_empty() {
        state.select(Some(app.entries.selected));
    }
    frame.render_stateful_widget(list, area, &mut state);

    if app.entries.create_active {
        draw_create_popup(frame, app, area);
    }
}

fn draw_create_popup(frame: &mut Frame, app: &App, area: Rect) {
    if app.tree_editor.is_some() && app.tree_target == TreeTarget::EntriesCreate {
        let popup = centered_rect(80, 80, area);
        frame.render_widget(Clear, popup);
        draw_tree(frame, app, popup);
        return;
    }

    let id_active = app.entries.create_field == EntriesCreateField::Id;
    let value_active = app.entries.create_field == EntriesCreateField::Value;
    let max_lines = 6;

    let popup = centered_rect_lines(50, max_lines + 2 + 3 + 2, area);
    frame.render_widget(Clear, popup);
    let block = Block::default().borders(Borders::ALL).title("Create Entry");
    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(inner);

    field_box(frame, rows[0], "Id", &app.entries.create_id, id_active);
    field_paragraph_box(frame, rows[1], "Value (JSON)", &app.entries.create_value, value_active);
}

fn delete(app: &mut App) -> Option<Action> {
    if !app.entries.marked.is_empty() {
        app.arm_confirm(PendingConfirm::BulkDeleteEntries);
        return None;
    }
    if app.entries.visible_indices().is_empty() {
        return None;
    }
    app.arm_confirm(PendingConfirm::DeleteEntry);
    None
}

fn view(app: &mut App) -> Option<Action> {
    if app.entries.visible_indices().is_empty() {
        return None;
    }
    app.screen = Screen::Value;
    Some(Action::LoadValue)
}

fn handle_create_key(app: &mut App, code: KeyCode, modifiers: KeyModifiers) -> Option<Action> {
    if app.entries.create_field == EntriesCreateField::Value
        && code == KeyCode::Char('t')
        && modifiers.contains(KeyModifiers::CONTROL)
    {
        app.enter_tree_mode_for(TreeTarget::EntriesCreate);
        return None;
    }

    match code {
        KeyCode::Tab | KeyCode::BackTab => {
            app.entries.create_field = match app.entries.create_field {
                EntriesCreateField::Id => EntriesCreateField::Value,
                EntriesCreateField::Value => EntriesCreateField::Id,
            };
            None
        }
        KeyCode::Enter => Some(Action::CreateEntry),
        KeyCode::Esc => {
            app.entries.create_active = false;
            app.status.clear();
            None
        }
        _ => {
            let field = match app.entries.create_field {
                EntriesCreateField::Id => &mut app.entries.create_id,
                EntriesCreateField::Value => &mut app.entries.create_value,
            };
            handle_text_field_key(field, code, |_| true);
            None
        }
    }
}
