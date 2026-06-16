pub(crate) mod entries;
pub(crate) mod memory_entries;
pub(crate) mod memory_store_input;
pub(crate) mod menu;
pub(crate) mod messaging;
pub(crate) mod ordered_entries;
pub(crate) mod ordered_store_input;
pub(crate) mod ordered_value;
pub(crate) mod stores;
pub(crate) mod universe_choice;
pub(crate) mod universe_input;
pub(crate) mod universe_select;
pub(crate) mod value;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::Rect;
use ratatui::Frame;
use ratatui_which_key::Keymap;

use crate::app::{Action, App, Screen};
use crate::update::{Act, Category, Scope};

type BindKeysFn = fn(&mut Keymap<KeyEvent, Scope, Act, Category>);

pub(crate) struct ScreenDef {
    pub handle_key: fn(&mut App, KeyCode, KeyModifiers) -> Option<Action>,
    pub draw: fn(&mut Frame, &App, Rect),
    pub bind_keys: Option<BindKeysFn>,
}

pub(crate) fn def(screen: Screen) -> &'static ScreenDef {
    &SCREENS[screen as usize]
}

pub(crate) const SCREENS: [ScreenDef; 13] = [
    // Screen::Menu (0)
    ScreenDef {
        handle_key: menu::handle_key,
        draw: menu::draw,
        bind_keys: Some(menu::bind_keys),
    },
    // Screen::UniverseChoice (1)
    ScreenDef {
        handle_key: universe_choice::handle_key,
        draw: universe_choice::draw,
        bind_keys: Some(universe_choice::bind_keys),
    },
    // Screen::UniverseSelect (2)
    ScreenDef {
        handle_key: universe_select::handle_key,
        draw: universe_select::draw,
        bind_keys: Some(universe_select::bind_keys),
    },
    // Screen::UniverseInput (3)
    ScreenDef {
        handle_key: universe_input::handle_key,
        draw: universe_input::draw,
        bind_keys: None,
    },
    // Screen::Stores (4)
    ScreenDef {
        handle_key: stores::handle_key,
        draw: stores::draw,
        bind_keys: Some(stores::bind_keys),
    },
    // Screen::Entries (5)
    ScreenDef {
        handle_key: entries::handle_key,
        draw: entries::draw,
        bind_keys: Some(entries::bind_keys),
    },
    // Screen::Value (6)
    ScreenDef {
        handle_key: value::handle_key,
        draw: value::draw,
        bind_keys: Some(value::bind_keys),
    },
    // Screen::Messaging (7)
    ScreenDef {
        handle_key: messaging::handle_key,
        draw: messaging::draw,
        bind_keys: None,
    },
    // Screen::OrderedStoreInput (8)
    ScreenDef {
        handle_key: ordered_store_input::handle_key,
        draw: ordered_store_input::draw,
        bind_keys: None,
    },
    // Screen::OrderedEntries (9)
    ScreenDef {
        handle_key: ordered_entries::handle_key,
        draw: ordered_entries::draw,
        bind_keys: Some(ordered_entries::bind_keys),
    },
    // Screen::OrderedValue (10)
    ScreenDef {
        handle_key: ordered_value::handle_key,
        draw: ordered_value::draw,
        bind_keys: Some(ordered_value::bind_keys),
    },
    // Screen::MemoryStoreInput (11)
    ScreenDef {
        handle_key: memory_store_input::handle_key,
        draw: memory_store_input::draw,
        bind_keys: None,
    },
    // Screen::MemoryStoreEntries (12)
    ScreenDef {
        handle_key: memory_entries::handle_key,
        draw: memory_entries::draw,
        bind_keys: Some(memory_entries::bind_keys),
    },
];
