use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::Rect;
use ratatui::widgets::{Block, Borders, List, ListItem, ListState};
use ratatui::Frame;
use ratatui_which_key::Keymap;

use crate::app::{
    Action, App, Screen, UNIVERSE_CHOICE_ENTER_ID, UNIVERSE_CHOICE_ITEMS, UNIVERSE_CHOICE_LIST_ALL,
};
use crate::update::{self, Act, Category, Scope};

pub(crate) struct State {
    pub selected: usize,
}

impl State {
    pub(crate) fn new() -> Self {
        Self { selected: 0 }
    }
}

pub(crate) fn bind_keys(km: &mut Keymap<KeyEvent, Scope, Act, Category>) {
    update::bind(km, KeyCode::Enter, Act { desc: "select", handler: select }, Scope::UniverseChoice);
    update::bind(km, KeyCode::Char('l'), Act { desc: "select", handler: select }, Scope::UniverseChoice);
}

pub(crate) fn handle_key(app: &mut App, code: KeyCode, _mods: KeyModifiers) -> Option<Action> {
    if let Some(result) = update::list_nav_key(code, &mut app.universe_choice.selected, UNIVERSE_CHOICE_ITEMS.len()) {
        return result;
    }
    if let Some(result) = update::back_key(code, app, Screen::Menu) {
        return result;
    }
    if let Some(result) = update::quit_key(code, app) {
        return result;
    }
    update::dispatch(app, Scope::UniverseChoice, code, KeyModifiers::empty())
}

pub(crate) fn draw(frame: &mut Frame, app: &App, area: Rect) {
    let items: Vec<ListItem> = UNIVERSE_CHOICE_ITEMS
        .iter()
        .map(|label| ListItem::new(*label))
        .collect();

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("Universe"))
        .highlight_style(crate::ui::HIGHLIGHT_STYLE);

    let mut state = ListState::default();
    state.select(Some(app.universe_choice.selected));
    frame.render_stateful_widget(list, area, &mut state);
}

fn select(app: &mut App) -> Option<Action> {
    match app.universe_choice.selected {
        UNIVERSE_CHOICE_ENTER_ID => {
            app.universe_input.field.clear();
            app.screen = Screen::UniverseInput;
            None
        }
        UNIVERSE_CHOICE_LIST_ALL => {
            if app.available_universes.is_empty() {
                Some(Action::LoadUniverses)
            } else {
                app.universe_select.selected = 0;
                app.screen = Screen::UniverseSelect;
                None
            }
        }
        _ => None,
    }
}
