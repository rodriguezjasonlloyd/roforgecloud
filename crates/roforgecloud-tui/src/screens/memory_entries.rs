use std::collections::HashSet;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState};
use ratatui::Frame;
use ratatui_which_key::Keymap;
use roforgecloud_core::opencloud::memory_store::SortedMapItem;

use crate::app::{Action, App, MemoryCreateField, PendingConfirm, Screen, TextField, TextFieldExt, TreeTarget};
use crate::status;
use crate::update::{self, Act, Category, Scope, bind, dispatch, handle_pending_confirm, handle_text_field_key, list_nav_key, quit_key};
use crate::ui::{HIGHLIGHT_STYLE, breadcrumb, centered_rect_lines, centered_rect, field_box, field_paragraph_box, draw_tree, universe_label};

pub(crate) struct State {
    pub items: Vec<SortedMapItem>,
    pub selected: usize,
    pub next_page_token: Option<String>,
    pub page_tokens: Vec<Option<String>>,
    pub marked: HashSet<usize>,
    pub search: TextField,
    pub search_active: bool,
    pub create_id: TextField,
    pub create_value: TextField,
    pub create_ttl: TextField,
    pub create_field: MemoryCreateField,
    pub create_active: bool,
    pub create_choosing: bool,
    pub ttl_edit: TextField,
    pub ttl_editing: bool,
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
            create_ttl: { let mut f = tui_textarea::TextArea::default(); f.set_value("3600"); f },
            create_field: MemoryCreateField::Id,
            create_active: false,
            create_choosing: false,
            ttl_edit: TextField::default(),
            ttl_editing: false,
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
            .filter(|(_, item)| item.id.to_lowercase().contains(&needle))
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
    bind(km, KeyCode::Char('n'), Act { desc: "next page", handler: |_| Some(Action::LoadNextMemoryItemsPage) }, Scope::MemoryEntries);
    bind(km, KeyCode::Char('p'), Act { desc: "prev page", handler: |_| Some(Action::LoadPrevMemoryItemsPage) }, Scope::MemoryEntries);
    bind(km, KeyCode::Char('r'), Act { desc: "refresh", handler: |_| Some(Action::RefreshMemoryItems) }, Scope::MemoryEntries);
    bind(
        km,
        KeyCode::Char('/'),
        Act {
            desc: "search",
            handler: |app| {
                app.memory_entries.search_active = true;
                app.status = status::loading_search("items");
                Some(Action::LoadAllMemoryItemsForSearch)
            },
        },
        Scope::MemoryEntries,
    );
    bind(
        km,
        KeyCode::Char('c'),
        Act { desc: "create", handler: |app| { app.memory_entries.create_choosing = true; None } },
        Scope::MemoryEntries,
    );
    bind(
        km,
        KeyCode::Char(' '),
        Act { desc: "select", handler: |app| { app.memory_entries.toggle_mark(); None } },
        Scope::MemoryEntries,
    );
    bind(
        km,
        KeyCode::Char('a'),
        Act { desc: "select all", handler: |app| { app.memory_entries.toggle_select_all_visible(); None } },
        Scope::MemoryEntries,
    );
    bind(
        km,
        KeyCode::Char('t'),
        Act {
            desc: "edit ttl",
            handler: |app| {
                if app.memory_entries.visible_indices().is_empty() {
                    return None;
                }
                app.memory_entries.ttl_edit.set_value("3600");
                app.memory_entries.ttl_editing = true;
                None
            },
        },
        Scope::MemoryEntries,
    );
    bind(km, KeyCode::Char('d'), Act { desc: "delete", handler: delete }, Scope::MemoryEntries);
    bind(km, KeyCode::Char('l'), Act { desc: "view", handler: view }, Scope::MemoryEntries);
}

pub(crate) fn handle_key(app: &mut App, code: KeyCode, modifiers: KeyModifiers) -> Option<Action> {
    if app.tree_editor.is_some() {
        return update::handle_tree_key(app, code, modifiers);
    }

    if app.memory_entries.create_active {
        return handle_create_key(app, code, modifiers);
    }

    if app.memory_entries.create_choosing {
        app.memory_entries.create_choosing = false;
        return match code {
            KeyCode::Char('n') => {
                app.memory_entries.create_id.clear();
                app.memory_entries.create_value.clear();
                app.memory_entries.create_ttl.set_value("3600");
                app.memory_entries.create_field = MemoryCreateField::Id;
                app.memory_entries.create_active = true;
                None
            }
            KeyCode::Char('e') => Some(Action::CreateMemoryItemExternal),
            _ => None,
        };
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

    if app.memory_entries.search_active {
        return match code {
            KeyCode::Enter | KeyCode::Esc => {
                app.memory_entries.search_active = false;
                app.memory_entries.search.clear();
                app.status.clear();
                Some(Action::RefreshMemoryItems)
            }
            _ => {
                if handle_text_field_key(&mut app.memory_entries.search, code, |_| true) {
                    app.memory_entries.selected = 0;
                }
                None
            }
        };
    }

    if let Some(result) = handle_pending_confirm(app, code) {
        return result;
    }

    let visible = app.memory_entries.visible_indices().len();

    if let Some(result) = list_nav_key(code, &mut app.memory_entries.selected, visible) {
        return result;
    }
    if let Some(result) = quit_key(code, app) {
        return result;
    }

    if matches!(code, KeyCode::Char('h')) {
        if !app.memory_entries.search.get_value().is_empty() {
            app.memory_entries.search.clear();
            app.memory_entries.selected = 0;
            app.status.clear();
            return None;
        }
        app.screen = Screen::MemoryStoreInput;
        app.status.clear();
        return None;
    }

    dispatch(app, Scope::MemoryEntries, code, modifiers)
}

pub(crate) fn draw(frame: &mut Frame, app: &App, area: Rect) {
    let visible = app.memory_entries.visible_indices();

    let items: Vec<ListItem> = visible
        .iter()
        .map(|&i| {
            let item = &app.memory_entries.items[i];
            let mut spans = Vec::new();
            if !app.memory_entries.marked.is_empty() {
                let marker = if app.memory_entries.marked.contains(&i) { "[x] " } else { "[ ] " };
                spans.push(Span::raw(marker));
            }
            let preview = serde_json::to_string(&item.value).unwrap_or_default();
            let expire = item.expire_time.as_deref().unwrap_or("—");
            spans.push(Span::raw(format!("{}  =  {preview}  (expires: {expire})", item.id)));
            ListItem::new(Line::from(spans))
        })
        .collect();

    let uni = universe_label(app);
    let q = app.memory_entries.search.get_value();
    let n = app.memory_entries.marked.len();
    let suffix = match (!q.is_empty(), n > 0) {
        (true, true)  => Some(format!("search: {q}  ·  {n} selected")),
        (true, false) => Some(format!("search: {q}")),
        (false, true) => Some(format!("{n} selected")),
        (false, false) => None,
    };
    let title = breadcrumb(&[uni.as_str(), "memory stores", &app.memory_store_input.id], suffix.as_deref());

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(title))
        .highlight_style(HIGHLIGHT_STYLE);

    let mut state = ListState::default();
    if !visible.is_empty() {
        state.select(Some(app.memory_entries.selected));
    }
    frame.render_stateful_widget(list, area, &mut state);

    if app.memory_entries.create_active {
        draw_create_popup(frame, app, area);
    }

    if app.memory_entries.ttl_editing {
        draw_ttl_popup(frame, app, area);
    }
}

fn draw_create_popup(frame: &mut Frame, app: &App, area: Rect) {
    if app.tree_editor.is_some() && app.tree_target == TreeTarget::MemoryCreate {
        let popup = centered_rect(80, 80, area);
        frame.render_widget(Clear, popup);
        draw_tree(frame, app, popup);
        return;
    }

    let id_active = app.memory_entries.create_field == MemoryCreateField::Id;
    let value_active = app.memory_entries.create_field == MemoryCreateField::Value;
    let ttl_active = app.memory_entries.create_field == MemoryCreateField::Ttl;
    let max_lines = 5;

    let popup = centered_rect_lines(50, max_lines + 2 + 3 + 3 + 2, area);
    frame.render_widget(Clear, popup);
    let block = Block::default().borders(Borders::ALL).title("Create Item");
    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0), Constraint::Length(3)])
        .split(inner);

    field_box(frame, rows[0], "Id", &app.memory_entries.create_id, id_active);
    field_paragraph_box(frame, rows[1], "Value (JSON)", &app.memory_entries.create_value, value_active);
    field_box(frame, rows[2], "TTL (seconds)", &app.memory_entries.create_ttl, ttl_active);
}

fn draw_ttl_popup(frame: &mut Frame, app: &App, area: Rect) {
    let popup = centered_rect_lines(40, 3, area);
    frame.render_widget(Clear, popup);
    field_box(frame, popup, "Edit TTL (seconds)", &app.memory_entries.ttl_edit, true);
}

fn delete(app: &mut App) -> Option<Action> {
    if !app.memory_entries.marked.is_empty() {
        app.arm_confirm(PendingConfirm::BulkDeleteMemoryItems);
        return None;
    }
    if app.memory_entries.visible_indices().is_empty() {
        return None;
    }
    app.arm_confirm(PendingConfirm::DeleteMemoryItem);
    None
}

fn view(app: &mut App) -> Option<Action> {
    if app.memory_entries.visible_indices().is_empty() {
        return None;
    }
    Some(Action::LoadMemoryValue)
}

fn handle_create_key(app: &mut App, code: KeyCode, modifiers: KeyModifiers) -> Option<Action> {
    if app.memory_entries.create_field == MemoryCreateField::Value
        && code == KeyCode::Char('t')
        && modifiers.contains(KeyModifiers::CONTROL)
    {
        app.enter_tree_mode_for(TreeTarget::MemoryCreate);
        return None;
    }

    match code {
        KeyCode::Tab | KeyCode::BackTab => {
            app.memory_entries.create_field = match app.memory_entries.create_field {
                MemoryCreateField::Id => MemoryCreateField::Value,
                MemoryCreateField::Value => MemoryCreateField::Ttl,
                MemoryCreateField::Ttl => MemoryCreateField::Id,
            };
            None
        }
        KeyCode::Enter => Some(Action::CreateMemoryItem),
        KeyCode::Esc => {
            app.memory_entries.create_active = false;
            app.status.clear();
            None
        }
        _ => {
            match app.memory_entries.create_field {
                MemoryCreateField::Id => handle_text_field_key(&mut app.memory_entries.create_id, code, |_| true),
                MemoryCreateField::Value => handle_text_field_key(&mut app.memory_entries.create_value, code, |_| true),
                MemoryCreateField::Ttl => handle_text_field_key(&mut app.memory_entries.create_ttl, code, |c| c.is_ascii_digit()),
            };
            None
        }
    }
}
