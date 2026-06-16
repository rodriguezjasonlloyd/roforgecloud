pub(crate) mod menu;
pub(crate) mod universe_choice;
pub(crate) mod universe_input;
pub(crate) mod universe_select;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::Rect;
use ratatui::Frame;
use ratatui_which_key::Keymap;

use crate::app::{Action, App, Screen};
use crate::update::{Act, Category, Scope};

pub(crate) struct ScreenDef {
    pub scope: Option<Scope>,
    pub handle_key: fn(&mut App, KeyCode, KeyModifiers) -> Option<Action>,
    pub draw: fn(&mut Frame, &App, Rect),
    pub bind_keys: Option<fn(&mut Keymap<KeyEvent, Scope, Act, Category>)>,
}

pub(crate) fn def(screen: Screen) -> &'static ScreenDef {
    &SCREENS[screen as usize]
}

pub(crate) const SCREENS: [ScreenDef; 13] = [
    // Screen::Menu (0)
    ScreenDef {
        scope: Some(Scope::Menu),
        handle_key: menu::handle_key,
        draw: menu::draw,
        bind_keys: Some(menu::bind_keys),
    },
    // Screen::UniverseChoice (1)
    ScreenDef {
        scope: Some(Scope::UniverseChoice),
        handle_key: universe_choice::handle_key,
        draw: universe_choice::draw,
        bind_keys: Some(universe_choice::bind_keys),
    },
    // Screen::UniverseSelect (2)
    ScreenDef {
        scope: Some(Scope::UniverseSelect),
        handle_key: universe_select::handle_key,
        draw: universe_select::draw,
        bind_keys: Some(universe_select::bind_keys),
    },
    // Screen::UniverseInput (3)
    ScreenDef {
        scope: None,
        handle_key: universe_input::handle_key,
        draw: universe_input::draw,
        bind_keys: None,
    },
    // Screen::Stores (4)
    ScreenDef {
        scope: Some(Scope::Stores),
        handle_key: crate::update::handle_stores_key,
        draw: crate::ui::draw_stores,
        bind_keys: None,
    },
    // Screen::Entries (5)
    ScreenDef {
        scope: Some(Scope::Entries),
        handle_key: crate::update::handle_entries_key,
        draw: crate::ui::draw_entries,
        bind_keys: None,
    },
    // Screen::Value (6)
    ScreenDef {
        scope: Some(Scope::Value),
        handle_key: crate::update::handle_value_key,
        draw: crate::ui::draw_value,
        bind_keys: None,
    },
    // Screen::Messaging (7)
    ScreenDef {
        scope: None,
        handle_key: crate::update::handle_messaging_key,
        draw: crate::ui::draw_messaging,
        bind_keys: None,
    },
    // Screen::OrderedStoreInput (8)
    ScreenDef {
        scope: None,
        handle_key: crate::update::handle_ordered_store_input_key,
        draw: crate::ui::draw_ordered_store_input,
        bind_keys: None,
    },
    // Screen::OrderedEntries (9)
    ScreenDef {
        scope: Some(Scope::OrderedEntries),
        handle_key: crate::update::handle_ordered_entries_key,
        draw: crate::ui::draw_ordered_entries,
        bind_keys: None,
    },
    // Screen::OrderedValue (10)
    ScreenDef {
        scope: Some(Scope::OrderedValue),
        handle_key: crate::update::handle_ordered_value_key,
        draw: crate::ui::draw_ordered_value,
        bind_keys: None,
    },
    // Screen::MemoryStoreInput (11)
    ScreenDef {
        scope: None,
        handle_key: crate::update::handle_memory_store_input_key,
        draw: crate::ui::draw_memory_store_input,
        bind_keys: None,
    },
    // Screen::MemoryStoreEntries (12)
    ScreenDef {
        scope: Some(Scope::MemoryEntries),
        handle_key: crate::update::handle_memory_entries_key,
        draw: crate::ui::draw_memory_entries,
        bind_keys: None,
    },
];
