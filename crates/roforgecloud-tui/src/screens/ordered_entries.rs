use std::collections::HashSet;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState};
use ratatui::Frame;
use ratatui_which_key::Keymap;
use roforgecloud_core::opencloud::ordered_datastore::OrderedDataStoreEntry;

use crate::app::{Action, App, OrderedCreateField, PendingConfirm, Screen, TextField};
use crate::update::{self, Act, Category, Scope, bind, dispatch, handle_pending_confirm, handle_text_field_key, list_nav_key, quit_key};
use crate::ui::{HIGHLIGHT_STYLE, centered_rect_lines, field_box, field_paragraph_box};

pub(crate) struct State {
    pub items: Vec<OrderedDataStoreEntry>,
    pub selected: usize,
    pub next_page_token: Option<String>,
    pub page_tokens: Vec<Option<String>>,
    pub marked: HashSet<usize>,
    pub search: TextField,
    pub search_active: bool,
    pub create_id: TextField,
    pub create_value: TextField,
    pub create_field: OrderedCreateField,
    pub create_active: bool,
    pub create_choosing: bool,
}

impl State {
    pub(crate) fn new() -> Self {
        Self {
            items: Vec::new(),
            selected: 0,
            next_page_token: None,
            page_tokens: vec![None],
            marked: HashSet::new(),
            search: TextField::default(),
            search_active: false,
            create_id: TextField::default(),
            create_value: TextField::default(),
            create_field: OrderedCreateField::Id,
            create_active: false,
            create_choosing: false,
        }
    }

    pub(crate) fn visible_indices(&self) -> Vec<usize> {
        if self.search.value.is_empty() {
            return (0..self.items.len()).collect();
        }
        let needle = self.search.value.to_lowercase();
        self.items
            .iter()
            .enumerate()
            .filter(|(_, entry)| entry.id.to_lowercase().contains(&needle))
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
}

pub(crate) fn bind_keys(km: &mut Keymap<KeyEvent, Scope, Act, Category>) {
    bind(km, KeyCode::Char('n'), Act { desc: "next page", handler: |_| Some(Action::LoadNextOrderedEntriesPage) }, Scope::OrderedEntries);
    bind(km, KeyCode::Char('p'), Act { desc: "prev page", handler: |_| Some(Action::LoadPrevOrderedEntriesPage) }, Scope::OrderedEntries);
    bind(km, KeyCode::Char('r'), Act { desc: "refresh", handler: |_| Some(Action::RefreshOrderedEntries) }, Scope::OrderedEntries);
    bind(
        km,
        KeyCode::Char('/'),
        Act {
            desc: "search",
            handler: |app| {
                app.ordered_entries.search_active = true;
                app.status = "loading all entries for search...".to_string();
                Some(Action::LoadAllOrderedEntriesForSearch)
            },
        },
        Scope::OrderedEntries,
    );
    bind(
        km,
        KeyCode::Char('c'),
        Act { desc: "create", handler: |app| { app.ordered_entries.create_choosing = true; None } },
        Scope::OrderedEntries,
    );
    bind(
        km,
        KeyCode::Char(' '),
        Act { desc: "select", handler: |app| { app.ordered_entries.toggle_mark(); None } },
        Scope::OrderedEntries,
    );
    bind(
        km,
        KeyCode::Char('a'),
        Act { desc: "select all", handler: |app| { app.ordered_entries.toggle_select_all_visible(); None } },
        Scope::OrderedEntries,
    );
    bind(km, KeyCode::Char('d'), Act { desc: "delete", handler: delete }, Scope::OrderedEntries);
    bind(km, KeyCode::Enter, Act { desc: "view", handler: view }, Scope::OrderedEntries);
    bind(km, KeyCode::Char('l'), Act { desc: "view", handler: view }, Scope::OrderedEntries);
}

pub(crate) fn handle_key(app: &mut App, code: KeyCode, _mods: KeyModifiers) -> Option<Action> {
    if app.ordered_entries.create_active {
        return handle_create_key(app, code);
    }

    if app.ordered_entries.create_choosing {
        app.ordered_entries.create_choosing = false;
        return match code {
            KeyCode::Char('n') => {
                app.ordered_entries.create_id.clear();
                app.ordered_entries.create_value.clear();
                app.ordered_entries.create_field = OrderedCreateField::Id;
                app.ordered_entries.create_active = true;
                None
            }
            KeyCode::Char('e') => Some(Action::CreateOrderedEntryExternal),
            _ => None,
        };
    }

    if app.ordered_entries.search_active {
        return match code {
            KeyCode::Enter | KeyCode::Esc => {
                app.ordered_entries.search_active = false;
                app.ordered_entries.search.clear();
                app.status.clear();
                Some(Action::RefreshOrderedEntries)
            }
            _ => {
                if handle_text_field_key(&mut app.ordered_entries.search, code, |_| true) {
                    app.ordered_entries.selected = 0;
                }
                None
            }
        };
    }

    if let Some(result) = handle_pending_confirm(app, code) {
        return result;
    }

    let visible = app.ordered_entries.visible_indices().len();

    if let Some(result) = list_nav_key(code, &mut app.ordered_entries.selected, visible) {
        return result;
    }
    if let Some(result) = quit_key(code, app) {
        return result;
    }

    if matches!(code, KeyCode::Esc | KeyCode::Backspace | KeyCode::Char('h')) {
        if !app.ordered_entries.search.value.is_empty() {
            app.ordered_entries.search.clear();
            app.ordered_entries.selected = 0;
            app.status.clear();
            return None;
        }
        app.screen = Screen::OrderedStoreInput;
        app.status.clear();
        return None;
    }

    dispatch(app, Scope::OrderedEntries, code, KeyModifiers::empty())
}

pub(crate) fn draw(frame: &mut Frame, app: &App, area: Rect) {
    let visible = app.ordered_entries.visible_indices();

    let items: Vec<ListItem> = visible
        .iter()
        .map(|&i| {
            let entry = &app.ordered_entries.items[i];
            let mut spans = Vec::new();
            if !app.ordered_entries.marked.is_empty() {
                let marker = if app.ordered_entries.marked.contains(&i) { "[x] " } else { "[ ] " };
                spans.push(Span::raw(marker));
            }
            spans.push(Span::raw(format!("{}  =  {}", entry.id, entry.value)));
            ListItem::new(Line::from(spans))
        })
        .collect();

    let store_label = match app.universe_names.get(&app.universe_id) {
        Some(name) => format!(
            "{} (scope: {}, universe {} ({name}))",
            app.ordered_store_input.store_id.value, app.ordered_store_input.scope.value, app.universe_id
        ),
        None => format!(
            "{} (scope: {})",
            app.ordered_store_input.store_id.value, app.ordered_store_input.scope.value
        ),
    };
    let title = if app.ordered_entries.search.value.is_empty() {
        if app.ordered_entries.marked.is_empty() {
            store_label
        } else {
            format!("{store_label} ({} selected)", app.ordered_entries.marked.len())
        }
    } else if app.ordered_entries.marked.is_empty() {
        format!("{store_label} (search: {})", app.ordered_entries.search.value)
    } else {
        format!(
            "{store_label} (search: {}, {} selected)",
            app.ordered_entries.search.value,
            app.ordered_entries.marked.len()
        )
    };

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(title))
        .highlight_style(HIGHLIGHT_STYLE);

    let mut state = ListState::default();
    if !visible.is_empty() {
        state.select(Some(app.ordered_entries.selected));
    }
    frame.render_stateful_widget(list, area, &mut state);

    if app.ordered_entries.create_active {
        draw_create_popup(frame, app, area);
    }
}

fn draw_create_popup(frame: &mut Frame, app: &App, area: Rect) {
    let id_active = app.ordered_entries.create_field == OrderedCreateField::Id;
    let value_active = app.ordered_entries.create_field == OrderedCreateField::Value;
    let max_lines = 5;

    let popup = centered_rect_lines(40, max_lines + 2 + 3 + 2, area);
    frame.render_widget(Clear, popup);
    let block = Block::default().borders(Borders::ALL).title("Create Entry");
    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(inner);

    field_box(frame, rows[0], "Id", &app.ordered_entries.create_id, id_active);
    field_paragraph_box(frame, rows[1], "Value", &app.ordered_entries.create_value, value_active);
}

fn delete(app: &mut App) -> Option<Action> {
    if !app.ordered_entries.marked.is_empty() {
        app.arm_confirm(PendingConfirm::BulkDeleteOrderedEntries);
        return None;
    }
    if app.ordered_entries.visible_indices().is_empty() {
        return None;
    }
    app.arm_confirm(PendingConfirm::DeleteOrderedEntry);
    None
}

fn view(app: &mut App) -> Option<Action> {
    if app.ordered_entries.visible_indices().is_empty() {
        return None;
    }
    app.screen = Screen::OrderedValue;
    Some(Action::LoadOrderedValue)
}

fn handle_create_key(app: &mut App, code: KeyCode) -> Option<Action> {
    match code {
        KeyCode::Tab | KeyCode::BackTab => {
            app.ordered_entries.create_field = match app.ordered_entries.create_field {
                OrderedCreateField::Id => OrderedCreateField::Value,
                OrderedCreateField::Value => OrderedCreateField::Id,
            };
            None
        }
        KeyCode::Enter => Some(Action::CreateOrderedEntry),
        KeyCode::Esc => {
            app.ordered_entries.create_active = false;
            app.status.clear();
            None
        }
        _ => {
            match app.ordered_entries.create_field {
                OrderedCreateField::Id => {
                    handle_text_field_key(&mut app.ordered_entries.create_id, code, |_| true)
                }
                OrderedCreateField::Value => handle_text_field_key(
                    &mut app.ordered_entries.create_value,
                    code,
                    update::is_numeric_input_char,
                ),
            };
            None
        }
    }
}
