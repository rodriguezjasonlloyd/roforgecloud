use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::Rect;
use ratatui::widgets::{Block, Borders, List, ListItem, ListState};
use ratatui::Frame;
use ratatui_which_key::Keymap;

use crate::app::{Action, App, Screen, SERVICE_ACCOUNT, SERVICE_DATA_STORES, SERVICE_MEMORY_STORES, SERVICE_MESSAGING, SERVICE_ORDERED_DATA_STORES};
use crate::ui::HIGHLIGHT_STYLE;
use crate::update::{self, Act, Category, Scope};

pub(crate) struct State {
    pub items: Vec<(&'static str, usize)>,
    pub selected: usize,
    pub pending_service: usize,
}

impl State {
    pub(crate) fn new() -> Self {
        Self {
            items: vec![
                ("Data Stores", SERVICE_DATA_STORES),
                ("Ordered Data Stores", SERVICE_ORDERED_DATA_STORES),
                ("Memory Stores", SERVICE_MEMORY_STORES),
                ("Messaging", SERVICE_MESSAGING),
                ("Account", SERVICE_ACCOUNT),
            ],
            selected: 0,
            pending_service: SERVICE_MESSAGING,
        }
    }
}

pub(crate) fn bind_keys(km: &mut Keymap<KeyEvent, Scope, Act, Category>) {
    update::bind(km, KeyCode::Enter, Act { desc: "open", handler: open }, Scope::Menu);
    update::bind(km, KeyCode::Char('l'), Act { desc: "open", handler: open }, Scope::Menu);
}

pub(crate) fn handle_key(app: &mut App, code: KeyCode, _mods: KeyModifiers) -> Option<Action> {
    let len = app.menu.items.len();
    if let Some(result) = update::list_nav_key(code, &mut app.menu.selected, len) {
        return result;
    }
    if let Some(result) = update::quit_key(code, app) {
        return result;
    }
    update::dispatch(app, Scope::Menu, code, KeyModifiers::empty())
}

pub(crate) fn draw(frame: &mut Frame, app: &App, area: Rect) {
    let items: Vec<ListItem> = app
        .menu
        .items
        .iter()
        .map(|(label, service)| {
            if *service == SERVICE_ACCOUNT {
                if app.logged_in {
                    ListItem::new("Logout")
                } else {
                    ListItem::new("Login")
                }
            } else {
                ListItem::new(*label)
            }
        })
        .collect();

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("roforgecloud"))
        .highlight_style(HIGHLIGHT_STYLE);

    let mut state = ListState::default();
    state.select(Some(app.menu.selected));
    frame.render_stateful_widget(list, area, &mut state);
}

fn open(app: &mut App) -> Option<Action> {
    let service = app.menu.items[app.menu.selected].1;
    match service {
        SERVICE_ACCOUNT if app.logged_in => Some(Action::Logout),
        SERVICE_ACCOUNT => Some(Action::Login),
        _ => {
            app.menu.pending_service = service;
            app.status.clear();
            app.universe_choice.selected = 0;
            app.screen = Screen::UniverseChoice;
            None
        }
    }
}
