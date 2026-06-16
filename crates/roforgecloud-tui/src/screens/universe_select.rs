use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState};
use ratatui::Frame;
use ratatui_which_key::Keymap;

use crate::app::{Action, App, Screen, TextField, TextFieldExt};
use crate::ui::{HIGHLIGHT_STYLE, breadcrumb};
use crate::update::{self, Act, Category, Scope};

pub(crate) struct State {
    pub selected: usize,
    pub search: TextField,
    pub search_active: bool,
}

impl State {
    pub(crate) fn new() -> Self {
        Self {
            selected: 0,
            search: TextField::default(),
            search_active: false,
        }
    }
}

pub(crate) fn bind_keys(km: &mut Keymap<KeyEvent, Scope, Act, Category>) {
    update::bind(
        km,
        KeyCode::Char('/'),
        Act {
            desc: "search",
            handler: |app| {
                app.universe_select.search_active = true;
                None
            },
        },
        Scope::UniverseSelect,
    );
    update::bind(km, KeyCode::Enter, Act { desc: "select", handler: choose }, Scope::UniverseSelect);
    update::bind(km, KeyCode::Char('l'), Act { desc: "select", handler: choose }, Scope::UniverseSelect);
}

pub(crate) fn handle_key(app: &mut App, code: KeyCode, _mods: KeyModifiers) -> Option<Action> {
    if app.universe_select.search_active {
        match code {
            KeyCode::Enter | KeyCode::Esc => {
                app.universe_select.search_active = false;
                app.status.clear();
            }
            _ => {
                if update::handle_text_field_key(&mut app.universe_select.search, code, |_| true) {
                    app.universe_select.selected = 0;
                }
            }
        }
        return None;
    }

    let visible_len = app.visible_universe_indices().len();

    if let Some(result) = update::list_nav_key(code, &mut app.universe_select.selected, visible_len) {
        return result;
    }
    if let Some(result) = update::quit_key(code, app) {
        return result;
    }

    if matches!(code, KeyCode::Esc | KeyCode::Backspace | KeyCode::Char('h')) {
        if !app.universe_select.search.get_value().is_empty() {
            app.universe_select.search.clear();
            app.universe_select.selected = 0;
            app.status.clear();
            return None;
        }
        app.screen = Screen::UniverseChoice;
        app.status.clear();
        return None;
    }

    update::dispatch(app, Scope::UniverseSelect, code, KeyModifiers::empty())
}

pub(crate) fn draw(frame: &mut Frame, app: &App, area: Rect) {
    let visible = app.visible_universe_indices();

    let items: Vec<ListItem> = visible
        .iter()
        .map(|&i| {
            let id = app.available_universes[i];
            let mut spans = vec![Span::raw(id.to_string())];
            if let Some(name) = app.universe_names.get(&id) {
                spans.push(Span::styled(
                    format!("  ({name})"),
                    Style::default().fg(ratatui::style::Color::DarkGray),
                ));
            }
            ListItem::new(Line::from(spans))
        })
        .collect();

    let q = app.universe_select.search.get_value();
    let suffix = (!q.is_empty()).then(|| format!("search: {q}"));
    let title = breadcrumb(&["roforgecloud", "select universe"], suffix.as_deref());

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(title))
        .highlight_style(HIGHLIGHT_STYLE);

    let mut state = ListState::default();
    if !visible.is_empty() {
        state.select(Some(app.universe_select.selected));
    }
    frame.render_stateful_widget(list, area, &mut state);
}

fn choose(app: &mut App) -> Option<Action> {
    let visible = app.visible_universe_indices();
    let &index = visible.get(app.universe_select.selected)?;
    let universe_id = app.available_universes[index];
    app.universe_id = universe_id;
    update::enter_service(app)
}
