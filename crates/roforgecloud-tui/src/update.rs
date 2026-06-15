use crossterm::event::{KeyCode, KeyModifiers};

use crate::app::{
    Action, App, EntriesCreateField, MemoryCreateField, MessagingField, OrderedCreateField,
    OrderedInputField, PendingConfirm, Screen, TextField, TreeTarget, ValueSource, SERVICE_ACCOUNT,
    SERVICE_DATA_STORES, SERVICE_MEMORY_STORES, SERVICE_MESSAGING, SERVICE_ORDERED_DATA_STORES,
    UNIVERSE_CHOICE_ENTER_ID, UNIVERSE_CHOICE_ITEMS, UNIVERSE_CHOICE_LIST_ALL,
};

fn enter_service(app: &mut App) -> Option<Action> {
    match app.pending_service {
        SERVICE_DATA_STORES => {
            app.screen = Screen::Stores;
            Some(Action::LoadStores)
        }
        SERVICE_MESSAGING => {
            app.messaging_topic.clear();
            app.messaging_message.clear();
            app.messaging_field = MessagingField::Topic;
            app.status.clear();
            app.screen = Screen::Messaging;
            app.resolve_current_universe_name();
            None
        }
        SERVICE_ORDERED_DATA_STORES => {
            app.ordered_data_store_id.clear();
            app.ordered_scope.set("global");
            app.ordered_input_field = OrderedInputField::StoreId;
            app.status.clear();
            app.screen = Screen::OrderedStoreInput;
            None
        }
        SERVICE_MEMORY_STORES => {
            app.memory_sorted_map_input.clear();
            app.status.clear();
            app.screen = Screen::MemoryStoreInput;
            None
        }
        _ => None,
    }
}

fn move_up(selected: &mut usize) {
    if *selected > 0 {
        *selected -= 1;
    }
}

fn move_down(selected: &mut usize, len: usize) {
    if *selected + 1 < len {
        *selected += 1;
    }
}

fn key_to_input(code: KeyCode, modifiers: KeyModifiers) -> tui_textarea::Input {
    let key = match code {
        KeyCode::Char(c) => tui_textarea::Key::Char(c),
        KeyCode::Backspace => tui_textarea::Key::Backspace,
        KeyCode::Enter => tui_textarea::Key::Enter,
        KeyCode::Left => tui_textarea::Key::Left,
        KeyCode::Right => tui_textarea::Key::Right,
        KeyCode::Up => tui_textarea::Key::Up,
        KeyCode::Down => tui_textarea::Key::Down,
        KeyCode::Tab => tui_textarea::Key::Tab,
        KeyCode::Delete => tui_textarea::Key::Delete,
        KeyCode::Home => tui_textarea::Key::Home,
        KeyCode::End => tui_textarea::Key::End,
        KeyCode::PageUp => tui_textarea::Key::PageUp,
        KeyCode::PageDown => tui_textarea::Key::PageDown,
        KeyCode::Esc => tui_textarea::Key::Esc,
        _ => tui_textarea::Key::Null,
    };
    tui_textarea::Input {
        key,
        ctrl: modifiers.contains(KeyModifiers::CONTROL),
        alt: modifiers.contains(KeyModifiers::ALT),
        shift: modifiers.contains(KeyModifiers::SHIFT),
    }
}

fn handle_text_field_key(
    field: &mut TextField,
    code: KeyCode,
    accept: impl Fn(char) -> bool,
) -> bool {
    match code {
        KeyCode::Char(c) if accept(c) => {
            field.insert(c);
            true
        }
        KeyCode::Backspace => {
            field.backspace();
            true
        }
        KeyCode::Delete => {
            field.delete();
            true
        }
        KeyCode::Left => {
            field.left();
            true
        }
        KeyCode::Right => {
            field.right();
            true
        }
        KeyCode::Home => {
            field.home();
            true
        }
        KeyCode::End => {
            field.end();
            true
        }
        _ => false,
    }
}

fn list_nav_key(code: KeyCode, selected: &mut usize, len: usize) -> Option<Option<Action>> {
    match code {
        KeyCode::Up | KeyCode::Char('k') => {
            move_up(selected);
            Some(None)
        }
        KeyCode::Down | KeyCode::Char('j') => {
            move_down(selected, len);
            Some(None)
        }
        _ => None,
    }
}

fn quit_key(code: KeyCode, app: &mut App) -> Option<Option<Action>> {
    if code != KeyCode::Char('q') {
        return None;
    }
    if app.needs_quit_confirm() {
        app.arm_confirm(PendingConfirm::Quit);
    } else {
        app.should_quit = true;
    }
    Some(None)
}

fn handle_pending_confirm(app: &mut App, code: KeyCode) -> Option<Option<Action>> {
    let pending = app.pending_confirm.take()?;
    app.confirm_deadline = None;

    match (pending, code) {
        (PendingConfirm::DeleteStore, KeyCode::Char('d')) => Some(Some(Action::DeleteDataStore)),
        (PendingConfirm::BulkDeleteStores, KeyCode::Char('d')) => {
            Some(Some(Action::BulkDeleteDataStores))
        }
        (PendingConfirm::BulkUndeleteStores, KeyCode::Char('u')) => {
            Some(Some(Action::BulkUndeleteDataStores))
        }
        (PendingConfirm::DeleteEntry, KeyCode::Char('d')) => Some(Some(Action::DeleteEntry)),
        (PendingConfirm::BulkDeleteEntries, KeyCode::Char('d')) => {
            Some(Some(Action::BulkDeleteEntries))
        }
        (PendingConfirm::DeleteOrderedEntry, KeyCode::Char('d')) => {
            Some(Some(Action::DeleteOrderedEntry))
        }
        (PendingConfirm::BulkDeleteOrderedEntries, KeyCode::Char('d')) => {
            Some(Some(Action::BulkDeleteOrderedEntries))
        }
        (PendingConfirm::DeleteMemoryItem, KeyCode::Char('d')) => {
            Some(Some(Action::DeleteMemoryItem))
        }
        (PendingConfirm::BulkDeleteMemoryItems, KeyCode::Char('d')) => {
            Some(Some(Action::BulkDeleteMemoryItems))
        }
        (PendingConfirm::Quit, KeyCode::Char('q')) => {
            app.should_quit = true;
            Some(None)
        }
        (PendingConfirm::TreeQuit, KeyCode::Esc)
        | (PendingConfirm::TreeQuit, KeyCode::Char('q')) => {
            app.exit_tree_mode();
            Some(None)
        }
        (PendingConfirm::TreeRefresh, KeyCode::Char('r')) => Some(Some(Action::RefreshTree)),
        _ => {
            app.status.clear();
            Some(None)
        }
    }
}

fn back_key(code: KeyCode, app: &mut App, screen: Screen) -> Option<Option<Action>> {
    match code {
        KeyCode::Esc | KeyCode::Backspace | KeyCode::Char('h') => {
            app.screen = screen;
            app.status.clear();
            Some(None)
        }
        _ => None,
    }
}

const MENU_KEYS: &[KeyAction] = &[KeyAction {
    keys: &[KeyCode::Enter, KeyCode::Char('l')],
    hint: |_| Some("enter/l: open"),
    handler: |app| {
        let service = app.menu_items[app.menu_selected].1;
        match service {
            SERVICE_ACCOUNT if app.logged_in => Some(Action::Logout),
            SERVICE_ACCOUNT => Some(Action::Login),
            _ => {
                app.pending_service = service;
                app.status.clear();
                app.universe_choice_selected = 0;
                app.screen = Screen::UniverseChoice;
                None
            }
        }
    },
}];

pub(crate) fn menu_hints(app: &App) -> String {
    MENU_KEYS
        .iter()
        .filter_map(|action| (action.hint)(app))
        .collect::<Vec<_>>()
        .join("   ")
}

pub(crate) fn handle_menu_key(app: &mut App, code: KeyCode) -> Option<Action> {
    let len = app.menu_items.len();
    if let Some(result) = list_nav_key(code, &mut app.menu_selected, len) {
        return result;
    }
    if let Some(result) = quit_key(code, app) {
        return result;
    }
    for action in MENU_KEYS {
        if action.keys.contains(&code) {
            return (action.handler)(app);
        }
    }
    None
}

const UNIVERSE_CHOICE_KEYS: &[KeyAction] = &[KeyAction {
    keys: &[KeyCode::Enter, KeyCode::Char('l')],
    hint: |_| Some("enter/l: select"),
    handler: |app| match app.universe_choice_selected {
        UNIVERSE_CHOICE_ENTER_ID => {
            app.universe_input.clear();
            app.screen = Screen::UniverseInput;
            None
        }
        UNIVERSE_CHOICE_LIST_ALL => {
            if app.available_universes.is_empty() {
                Some(Action::LoadUniverses)
            } else {
                app.universe_select_selected = 0;
                app.screen = Screen::UniverseSelect;
                None
            }
        }
        _ => None,
    },
}];

pub(crate) fn universe_choice_hints(app: &App) -> String {
    UNIVERSE_CHOICE_KEYS
        .iter()
        .filter_map(|action| (action.hint)(app))
        .collect::<Vec<_>>()
        .join("   ")
}

pub(crate) fn handle_universe_choice_key(app: &mut App, code: KeyCode) -> Option<Action> {
    if let Some(result) = list_nav_key(
        code,
        &mut app.universe_choice_selected,
        UNIVERSE_CHOICE_ITEMS.len(),
    ) {
        return result;
    }
    if let Some(result) = back_key(code, app, Screen::Menu) {
        return result;
    }
    if let Some(result) = quit_key(code, app) {
        return result;
    }
    for action in UNIVERSE_CHOICE_KEYS {
        if action.keys.contains(&code) {
            return (action.handler)(app);
        }
    }
    None
}

const UNIVERSE_SELECT_KEYS: &[KeyAction] = &[
    KeyAction {
        keys: &[KeyCode::Char('/')],
        hint: |_| Some("/: search"),
        handler: |app| {
            app.universe_search_active = true;
            None
        },
    },
    KeyAction {
        keys: &[KeyCode::Enter, KeyCode::Char('l')],
        hint: |_| Some("enter/l: select"),
        handler: |app| {
            let visible = app.visible_universe_indices();
            let &index = visible.get(app.universe_select_selected)?;
            let universe_id = app.available_universes[index];
            app.universe_id = universe_id;
            enter_service(app)
        },
    },
];

pub(crate) fn universe_select_hints(app: &App) -> String {
    UNIVERSE_SELECT_KEYS
        .iter()
        .filter_map(|action| (action.hint)(app))
        .collect::<Vec<_>>()
        .join("   ")
}

pub(crate) fn handle_universe_select_key(app: &mut App, code: KeyCode) -> Option<Action> {
    if app.universe_search_active {
        match code {
            KeyCode::Enter | KeyCode::Esc => {
                app.universe_search_active = false;
                app.status.clear();
            }
            _ => {
                if handle_text_field_key(&mut app.universe_search, code, |_| true) {
                    app.universe_select_selected = 0;
                }
            }
        }
        return None;
    }

    let visible_len = app.visible_universe_indices().len();

    if let Some(result) = list_nav_key(code, &mut app.universe_select_selected, visible_len) {
        return result;
    }
    if let Some(result) = quit_key(code, app) {
        return result;
    }

    if matches!(code, KeyCode::Esc | KeyCode::Backspace | KeyCode::Char('h')) {
        if !app.universe_search.value.is_empty() {
            app.universe_search.clear();
            app.universe_select_selected = 0;
            app.status.clear();
            return None;
        }
        app.screen = Screen::UniverseChoice;
        app.status.clear();
        return None;
    }

    for action in UNIVERSE_SELECT_KEYS {
        if action.keys.contains(&code) {
            return (action.handler)(app);
        }
    }
    None
}

pub(crate) fn handle_universe_input_key(app: &mut App, code: KeyCode) -> Option<Action> {
    match code {
        KeyCode::Backspace if app.universe_input.value.is_empty() => {
            app.screen = Screen::UniverseChoice;
            app.status.clear();
            None
        }
        KeyCode::Esc => {
            app.screen = Screen::UniverseChoice;
            app.status.clear();
            None
        }
        KeyCode::Enter => {
            let Ok(id) = app.universe_input.value.parse::<u64>() else {
                return None;
            };
            app.universe_id = id;
            enter_service(app)
        }
        _ => {
            handle_text_field_key(&mut app.universe_input, code, |c| c.is_ascii_digit());
            None
        }
    }
}

pub(crate) fn handle_messaging_key(app: &mut App, code: KeyCode) -> Option<Action> {
    match code {
        KeyCode::Tab | KeyCode::BackTab => {
            app.messaging_field = match app.messaging_field {
                MessagingField::Topic => MessagingField::Message,
                MessagingField::Message => MessagingField::Topic,
            };
            None
        }
        KeyCode::Enter => Some(Action::PublishMessage),
        KeyCode::Esc => {
            app.screen = Screen::UniverseChoice;
            app.status.clear();
            None
        }
        _ => {
            let field = match app.messaging_field {
                MessagingField::Topic => &mut app.messaging_topic,
                MessagingField::Message => &mut app.messaging_message,
            };
            handle_text_field_key(field, code, |_| true);
            None
        }
    }
}

struct KeyAction {
    keys: &'static [KeyCode],
    hint: fn(&App) -> Option<&'static str>,
    handler: fn(&mut App) -> Option<Action>,
}

const STORES_KEYS: &[KeyAction] = &[
    KeyAction {
        keys: &[KeyCode::Enter, KeyCode::Char('l')],
        hint: |app| match app.stores.get(app.stores_selected) {
            Some(store) if store.state.as_deref().is_some_and(|s| s != "ACTIVE") => None,
            Some(_) => Some("enter/l: open"),
            None => None,
        },
        handler: |app| {
            let store = app.stores.get(app.stores_selected)?;
            if store.state.as_deref().is_some_and(|s| s != "ACTIVE") {
                return None;
            }
            app.data_store_id = store.id.clone();
            app.entries_next_page_token = None;
            app.screen = Screen::Entries;
            Some(Action::LoadEntries)
        },
    },
    KeyAction {
        keys: &[KeyCode::Char(' ')],
        hint: |_| Some("space: select"),
        handler: |app| {
            app.toggle_store_mark();
            None
        },
    },
    KeyAction {
        keys: &[KeyCode::Char('a')],
        hint: |_| Some("a: select all"),
        handler: |app| {
            app.toggle_select_all_stores();
            None
        },
    },
    KeyAction {
        keys: &[KeyCode::Char('r')],
        hint: |_| Some("r: refresh"),
        handler: |_| Some(Action::LoadStores),
    },
    KeyAction {
        keys: &[KeyCode::Char('c')],
        hint: |_| Some("c: create entry in new store"),
        handler: |app| {
            app.stores_new_id.clear();
            app.stores_new_active = true;
            None
        },
    },
    KeyAction {
        keys: &[KeyCode::Char('d')],
        hint: |app| {
            if !app.stores_marked.is_empty() {
                return Some("d: delete (selected)");
            }
            match app.stores.get(app.stores_selected) {
                Some(store) if store.state.as_deref().is_some_and(|s| s != "ACTIVE") => None,
                Some(_) => Some("d: delete"),
                None => None,
            }
        },
        handler: |app| {
            if !app.stores_marked.is_empty() {
                app.arm_confirm(PendingConfirm::BulkDeleteStores);
                return None;
            }
            let store = app.stores.get(app.stores_selected)?;
            if store.state.as_deref().is_some_and(|s| s != "ACTIVE") {
                return None;
            }
            app.arm_confirm(PendingConfirm::DeleteStore);
            None
        },
    },
    KeyAction {
        keys: &[KeyCode::Char('u')],
        hint: |app| {
            if !app.stores_marked.is_empty() {
                Some("u: undelete (selected)")
            } else if app
                .stores
                .get(app.stores_selected)
                .is_some_and(|s| s.state.as_deref() != Some("ACTIVE"))
            {
                Some("u: undelete")
            } else {
                None
            }
        },
        handler: |app| {
            if !app.stores_marked.is_empty() {
                app.arm_confirm(PendingConfirm::BulkUndeleteStores);
                return None;
            }
            let store = app.stores.get(app.stores_selected)?;
            if store.state.as_deref() != Some("ACTIVE") {
                return Some(Action::UndeleteDataStore);
            }
            None
        },
    },
];

pub(crate) fn stores_hints(app: &App) -> String {
    STORES_KEYS
        .iter()
        .filter_map(|action| (action.hint)(app))
        .collect::<Vec<_>>()
        .join("   ")
}

fn handle_stores_new_key(app: &mut App, code: KeyCode) -> Option<Action> {
    match code {
        KeyCode::Enter => {
            let id = app.stores_new_id.value.trim().to_string();
            if id.is_empty() {
                return None;
            }
            app.stores_new_active = false;
            app.data_store_id = id;
            app.entries.clear();
            app.entries_selected = 0;
            app.entries_next_page_token = None;
            app.entries_page_tokens = vec![None];
            app.entries_marked.clear();
            app.entries_search.clear();
            app.entries_create_id.clear();
            app.entries_create_value.clear();
            app.entries_create_field = EntriesCreateField::Id;
            app.entries_create_active = true;
            app.status.clear();
            app.screen = Screen::Entries;
            None
        }
        KeyCode::Esc => {
            app.stores_new_active = false;
            app.status.clear();
            None
        }
        _ => {
            handle_text_field_key(&mut app.stores_new_id, code, |_| true);
            None
        }
    }
}

pub(crate) fn handle_stores_key(app: &mut App, code: KeyCode) -> Option<Action> {
    if app.stores_new_active {
        return handle_stores_new_key(app, code);
    }

    if let Some(result) = handle_pending_confirm(app, code) {
        return result;
    }

    let len = app.stores.len();
    if let Some(result) = list_nav_key(code, &mut app.stores_selected, len) {
        return result;
    }
    if let Some(result) = back_key(code, app, Screen::UniverseChoice) {
        return result;
    }
    if let Some(result) = quit_key(code, app) {
        return result;
    }

    for action in STORES_KEYS {
        if action.keys.contains(&code) {
            return (action.handler)(app);
        }
    }
    None
}

const ENTRIES_KEYS: &[KeyAction] = &[
    KeyAction {
        keys: &[KeyCode::Char('n')],
        hint: |app| {
            app.entries_next_page_token
                .is_some()
                .then_some("n: next page")
        },
        handler: |_| Some(Action::LoadNextEntriesPage),
    },
    KeyAction {
        keys: &[KeyCode::Char('p')],
        hint: |app| (app.entries_page_tokens.len() > 1).then_some("p: prev page"),
        handler: |_| Some(Action::LoadPrevEntriesPage),
    },
    KeyAction {
        keys: &[KeyCode::Char('r')],
        hint: |_| Some("r: refresh"),
        handler: |_| Some(Action::RefreshEntries),
    },
    KeyAction {
        keys: &[KeyCode::Char('/')],
        hint: |_| Some("/: search"),
        handler: |app| {
            app.entries_search_active = true;
            app.status = "loading all entries for search...".to_string();
            Some(Action::LoadAllEntriesForSearch)
        },
    },
    KeyAction {
        keys: &[KeyCode::Char('c')],
        hint: |_| Some("c: create"),
        handler: |app| {
            app.entries_create_choosing = true;
            None
        },
    },
    KeyAction {
        keys: &[KeyCode::Char(' ')],
        hint: |_| Some("space: select"),
        handler: |app| {
            app.toggle_entry_mark();
            None
        },
    },
    KeyAction {
        keys: &[KeyCode::Char('a')],
        hint: |_| Some("a: select all"),
        handler: |app| {
            app.toggle_select_all_visible();
            None
        },
    },
    KeyAction {
        keys: &[KeyCode::Char('d')],
        hint: |app| {
            if app.visible_entry_indices().is_empty() && app.entries_marked.is_empty() {
                None
            } else if app.entries_marked.is_empty() {
                Some("d: delete")
            } else {
                Some("d: delete (selected)")
            }
        },
        handler: |app| {
            if !app.entries_marked.is_empty() {
                app.arm_confirm(PendingConfirm::BulkDeleteEntries);
                return None;
            }
            if app.visible_entry_indices().is_empty() {
                return None;
            }
            app.arm_confirm(PendingConfirm::DeleteEntry);
            None
        },
    },
    KeyAction {
        keys: &[KeyCode::Enter, KeyCode::Char('l')],
        hint: |_| Some("enter/l: view"),
        handler: |app| {
            if app.visible_entry_indices().is_empty() {
                return None;
            }
            app.screen = Screen::Value;
            Some(Action::LoadValue)
        },
    },
];

pub(crate) fn entries_hints(app: &App) -> String {
    ENTRIES_KEYS
        .iter()
        .filter_map(|action| (action.hint)(app))
        .collect::<Vec<_>>()
        .join("   ")
}

const ENTRIES_CREATE_KEYS: &[KeyAction] = &[
    KeyAction {
        keys: &[KeyCode::Tab, KeyCode::BackTab],
        hint: |_| Some("tab: switch field"),
        handler: |_| None,
    },
    KeyAction {
        keys: &[KeyCode::Enter],
        hint: |_| Some("enter: create"),
        handler: |_| None,
    },
    KeyAction {
        keys: &[KeyCode::Char('t')],
        hint: |app| {
            (app.entries_create_field == EntriesCreateField::Value)
                .then_some("ctrl+t: tree edit value")
        },
        handler: |_| None,
    },
    KeyAction {
        keys: &[KeyCode::Esc],
        hint: |_| Some("esc: cancel"),
        handler: |_| None,
    },
];

pub(crate) fn entries_create_hints(app: &App) -> String {
    ENTRIES_CREATE_KEYS
        .iter()
        .filter_map(|action| (action.hint)(app))
        .collect::<Vec<_>>()
        .join("   ")
}

fn handle_entries_create_key(
    app: &mut App,
    code: KeyCode,
    modifiers: KeyModifiers,
) -> Option<Action> {
    if app.entries_create_field == EntriesCreateField::Value
        && code == KeyCode::Char('t')
        && modifiers.contains(KeyModifiers::CONTROL)
    {
        app.enter_tree_mode_for(TreeTarget::EntriesCreate);
        return None;
    }

    match code {
        KeyCode::Tab | KeyCode::BackTab => {
            app.entries_create_field = match app.entries_create_field {
                EntriesCreateField::Id => EntriesCreateField::Value,
                EntriesCreateField::Value => EntriesCreateField::Id,
            };
            None
        }
        KeyCode::Enter => Some(Action::CreateEntry),
        KeyCode::Esc => {
            app.entries_create_active = false;
            app.status.clear();
            None
        }
        _ => {
            let field = match app.entries_create_field {
                EntriesCreateField::Id => &mut app.entries_create_id,
                EntriesCreateField::Value => &mut app.entries_create_value,
            };
            handle_text_field_key(field, code, |_| true);
            None
        }
    }
}

pub(crate) fn handle_entries_key(app: &mut App, code: KeyCode, modifiers: KeyModifiers) -> Option<Action> {
    if app.tree_editor.is_some() {
        return handle_tree_key(app, code, modifiers);
    }

    if app.entries_create_active {
        return handle_entries_create_key(app, code, modifiers);
    }

    if app.entries_create_choosing {
        app.entries_create_choosing = false;
        return match code {
            KeyCode::Char('n') => {
                app.entries_create_id.clear();
                app.entries_create_value.clear();
                app.entries_create_field = EntriesCreateField::Id;
                app.entries_create_active = true;
                None
            }
            KeyCode::Char('e') => Some(Action::CreateEntryExternal),
            _ => None,
        };
    }

    if app.entries_search_active {
        return match code {
            KeyCode::Enter | KeyCode::Esc => {
                app.entries_search_active = false;
                app.entries_search.clear();
                app.status.clear();
                Some(Action::RefreshEntries)
            }
            _ => {
                if handle_text_field_key(&mut app.entries_search, code, |_| true) {
                    app.entries_selected = 0;
                }
                None
            }
        };
    }

    if let Some(result) = handle_pending_confirm(app, code) {
        return result;
    }

    let visible = app.visible_entry_indices().len();

    if let Some(result) = list_nav_key(code, &mut app.entries_selected, visible) {
        return result;
    }
    if let Some(result) = quit_key(code, app) {
        return result;
    }

    if matches!(code, KeyCode::Esc | KeyCode::Backspace | KeyCode::Char('h')) {
        if !app.entries_search.value.is_empty() {
            app.entries_search.clear();
            app.entries_selected = 0;
            app.status.clear();
            return None;
        }
        app.screen = Screen::Stores;
        app.status.clear();
        return Some(Action::LoadStores);
    }

    for action in ENTRIES_KEYS {
        if action.keys.contains(&code) {
            return (action.handler)(app);
        }
    }
    None
}

const VALUE_KEYS: &[KeyAction] = &[
    KeyAction {
        keys: &[KeyCode::Char('r')],
        hint: |_| Some("r: refresh"),
        handler: |_| Some(Action::LoadValue),
    },
    KeyAction {
        keys: &[KeyCode::Enter, KeyCode::Char('l')],
        hint: |_| Some("enter/l: tree edit"),
        handler: |app| {
            app.enter_tree_mode();
            None
        },
    },
    KeyAction {
        keys: &[KeyCode::Char('e')],
        hint: |_| Some("e: edit in $EDITOR"),
        handler: |_| Some(Action::EditValueExternal),
    },
    KeyAction {
        keys: &[KeyCode::Char('d')],
        hint: |_| Some("d: delete"),
        handler: |app| {
            let pending = match app.value_source {
                ValueSource::DataStore => PendingConfirm::DeleteEntry,
                ValueSource::MemoryStoreSortedMap => PendingConfirm::DeleteMemoryItem,
            };
            app.arm_confirm(pending);
            None
        },
    },
    KeyAction {
        keys: &[KeyCode::Char('t')],
        hint: |app| {
            (app.value_source == ValueSource::MemoryStoreSortedMap).then_some("t: edit ttl")
        },
        handler: |app| {
            if app.value_source != ValueSource::MemoryStoreSortedMap {
                return None;
            }
            app.memory_ttl_edit
                .set(app.memory_item_ttl_seconds.to_string());
            app.memory_ttl_editing = true;
            None
        },
    },
    KeyAction {
        keys: &[KeyCode::Up, KeyCode::Char('k')],
        hint: |_| None,
        handler: |app| {
            app.value_scroll = app.value_scroll.saturating_sub(1);
            None
        },
    },
    KeyAction {
        keys: &[KeyCode::Down, KeyCode::Char('j')],
        hint: |_| None,
        handler: |app| {
            let max_scroll = app.max_value_scroll();
            app.value_scroll = (app.value_scroll + 1).min(max_scroll);
            None
        },
    },
    KeyAction {
        keys: &[KeyCode::PageUp],
        hint: |_| None,
        handler: |app| {
            app.value_scroll = app.value_scroll.saturating_sub(10);
            None
        },
    },
    KeyAction {
        keys: &[KeyCode::PageDown],
        hint: |_| None,
        handler: |app| {
            let max_scroll = app.max_value_scroll();
            app.value_scroll = (app.value_scroll + 10).min(max_scroll);
            None
        },
    },
];

pub(crate) fn value_hints(app: &App) -> String {
    VALUE_KEYS
        .iter()
        .filter_map(|action| (action.hint)(app))
        .collect::<Vec<_>>()
        .join("   ")
}

pub(crate) fn handle_value_key(app: &mut App, code: KeyCode, modifiers: KeyModifiers) -> Option<Action> {
    if app.tree_editor.is_some() {
        return handle_tree_key(app, code, modifiers);
    }

    if app.memory_ttl_editing {
        return match code {
            KeyCode::Enter => Some(Action::SaveMemoryTtl),
            KeyCode::Esc => {
                app.memory_ttl_editing = false;
                app.status.clear();
                None
            }
            _ => {
                handle_text_field_key(&mut app.memory_ttl_edit, code, |c| c.is_ascii_digit());
                None
            }
        };
    }

    if let Some(result) = handle_pending_confirm(app, code) {
        return result;
    }
    if let Some(result) = quit_key(code, app) {
        return result;
    }
    let back_screen = match app.value_source {
        ValueSource::DataStore => Screen::Entries,
        ValueSource::MemoryStoreSortedMap => Screen::MemoryStoreEntries,
    };
    if let Some(result) = back_key(code, app, back_screen) {
        return result;
    }

    for action in VALUE_KEYS {
        if action.keys.contains(&code) {
            return (action.handler)(app);
        }
    }
    None
}

const TREE_KEYS: &[KeyAction] = &[
    KeyAction {
        keys: &[KeyCode::Char('s')],
        hint: |_| Some("ctrl+s: save"),
        handler: |_| None,
    },
    KeyAction {
        keys: &[KeyCode::Up, KeyCode::Char('k')],
        hint: |_| None,
        handler: |app| {
            app.tree_editor.as_mut().unwrap().move_cursor(-1);
            None
        },
    },
    KeyAction {
        keys: &[KeyCode::Down, KeyCode::Char('j')],
        hint: |_| None,
        handler: |app| {
            app.tree_editor.as_mut().unwrap().move_cursor(1);
            None
        },
    },
    KeyAction {
        keys: &[KeyCode::Char(' ')],
        hint: |_| Some("space: fold/unfold"),
        handler: |app| {
            app.tree_editor.as_mut().unwrap().toggle();
            None
        },
    },
    KeyAction {
        keys: &[KeyCode::Enter],
        hint: |_| None,
        handler: |app| {
            app.tree_editor.as_mut().unwrap().edit_leaf();
            None
        },
    },
    KeyAction {
        keys: &[KeyCode::Char('y')],
        hint: |_| Some("y: yank"),
        handler: |app| {
            let mut clipboard = app.clipboard.take();
            if let Some(status) = app.tree_editor.as_ref().unwrap().yank(&mut clipboard) {
                app.status = status;
            }
            app.clipboard = clipboard;
            None
        },
    },
    KeyAction {
        keys: &[KeyCode::Char('p')],
        hint: |_| Some("p: paste"),
        handler: |app| {
            let mut clipboard = app.clipboard.take();
            if let Some(status) = app.tree_editor.as_mut().unwrap().paste(&mut clipboard) {
                app.status = status;
            }
            app.clipboard = clipboard;
            None
        },
    },
    KeyAction {
        keys: &[KeyCode::Char('d')],
        hint: |app| (app.tree_target == TreeTarget::Value).then_some("d: delete entry"),
        handler: |app| {
            if app.tree_target != TreeTarget::Value {
                return None;
            }
            let pending = match app.value_source {
                ValueSource::DataStore => PendingConfirm::DeleteEntry,
                ValueSource::MemoryStoreSortedMap => PendingConfirm::DeleteMemoryItem,
            };
            app.arm_confirm(pending);
            None
        },
    },
    KeyAction {
        keys: &[KeyCode::Char('a')],
        hint: |_| Some("a: add entry"),
        handler: |app| {
            app.tree_editor.as_mut().unwrap().add_entry();
            None
        },
    },
    KeyAction {
        keys: &[KeyCode::Char('x')],
        hint: |_| Some("x: delete node"),
        handler: |app| {
            app.tree_editor.as_mut().unwrap().delete_current();
            None
        },
    },
    KeyAction {
        keys: &[KeyCode::Char('e')],
        hint: |_| Some("e: edit"),
        handler: |app| {
            app.tree_editor.as_mut().unwrap().set_pending_leader(true);
            None
        },
    },
    KeyAction {
        keys: &[KeyCode::Char('r')],
        hint: |app| (app.tree_target == TreeTarget::Value).then_some("r: refresh"),
        handler: |app| {
            if app.tree_target != TreeTarget::Value {
                return None;
            }
            if app.tree_editor.as_ref().unwrap().dirty() {
                app.arm_confirm(PendingConfirm::TreeRefresh);
                None
            } else {
                Some(Action::RefreshTree)
            }
        },
    },
    KeyAction {
        keys: &[KeyCode::Esc, KeyCode::Char('q')],
        hint: |_| Some("esc/q: exit tree"),
        handler: |app| {
            if app.tree_editor.as_ref().unwrap().dirty() {
                app.arm_confirm(PendingConfirm::TreeQuit);
            } else {
                app.exit_tree_mode();
            }
            None
        },
    },
];

pub(crate) fn tree_hints(app: &App) -> String {
    TREE_KEYS
        .iter()
        .filter_map(|action| (action.hint)(app))
        .collect::<Vec<_>>()
        .join("   ")
}

pub(crate) fn handle_ordered_store_input_key(app: &mut App, code: KeyCode) -> Option<Action> {
    match code {
        KeyCode::Tab | KeyCode::BackTab => {
            app.ordered_input_field = match app.ordered_input_field {
                OrderedInputField::StoreId => OrderedInputField::Scope,
                OrderedInputField::Scope => OrderedInputField::StoreId,
            };
            None
        }
        KeyCode::Esc => {
            app.screen = Screen::UniverseChoice;
            app.status.clear();
            None
        }
        KeyCode::Backspace
            if app.ordered_input_field == OrderedInputField::StoreId
                && app.ordered_data_store_id.value.is_empty() =>
        {
            app.screen = Screen::UniverseChoice;
            app.status.clear();
            None
        }
        KeyCode::Enter => {
            if app.ordered_data_store_id.value.is_empty() {
                return None;
            }
            if app.ordered_scope.value.is_empty() {
                app.ordered_scope.set("global");
            }
            app.ordered_entries_next_page_token = None;
            app.screen = Screen::OrderedEntries;
            Some(Action::LoadOrderedEntries)
        }
        _ => {
            let field = match app.ordered_input_field {
                OrderedInputField::StoreId => &mut app.ordered_data_store_id,
                OrderedInputField::Scope => &mut app.ordered_scope,
            };
            handle_text_field_key(field, code, |_| true);
            None
        }
    }
}

const ORDERED_ENTRIES_KEYS: &[KeyAction] = &[
    KeyAction {
        keys: &[KeyCode::Char('n')],
        hint: |app| {
            app.ordered_entries_next_page_token
                .is_some()
                .then_some("n: next page")
        },
        handler: |_| Some(Action::LoadNextOrderedEntriesPage),
    },
    KeyAction {
        keys: &[KeyCode::Char('p')],
        hint: |app| (app.ordered_entries_page_tokens.len() > 1).then_some("p: prev page"),
        handler: |_| Some(Action::LoadPrevOrderedEntriesPage),
    },
    KeyAction {
        keys: &[KeyCode::Char('r')],
        hint: |_| Some("r: refresh"),
        handler: |_| Some(Action::RefreshOrderedEntries),
    },
    KeyAction {
        keys: &[KeyCode::Char('/')],
        hint: |_| Some("/: search"),
        handler: |app| {
            app.ordered_entries_search_active = true;
            app.status = "loading all entries for search...".to_string();
            Some(Action::LoadAllOrderedEntriesForSearch)
        },
    },
    KeyAction {
        keys: &[KeyCode::Char('c')],
        hint: |_| Some("c: create"),
        handler: |app| {
            app.ordered_create_choosing = true;
            None
        },
    },
    KeyAction {
        keys: &[KeyCode::Char(' ')],
        hint: |_| Some("space: select"),
        handler: |app| {
            app.toggle_ordered_entry_mark();
            None
        },
    },
    KeyAction {
        keys: &[KeyCode::Char('a')],
        hint: |_| Some("a: select all"),
        handler: |app| {
            app.toggle_select_all_ordered_visible();
            None
        },
    },
    KeyAction {
        keys: &[KeyCode::Char('d')],
        hint: |app| {
            if app.visible_ordered_entry_indices().is_empty()
                && app.ordered_entries_marked.is_empty()
            {
                None
            } else if app.ordered_entries_marked.is_empty() {
                Some("d: delete")
            } else {
                Some("d: delete (selected)")
            }
        },
        handler: |app| {
            if !app.ordered_entries_marked.is_empty() {
                app.arm_confirm(PendingConfirm::BulkDeleteOrderedEntries);
                return None;
            }
            if app.visible_ordered_entry_indices().is_empty() {
                return None;
            }
            app.arm_confirm(PendingConfirm::DeleteOrderedEntry);
            None
        },
    },
    KeyAction {
        keys: &[KeyCode::Enter, KeyCode::Char('l')],
        hint: |_| Some("enter/l: view"),
        handler: |app| {
            if app.visible_ordered_entry_indices().is_empty() {
                return None;
            }
            app.screen = Screen::OrderedValue;
            Some(Action::LoadOrderedValue)
        },
    },
];

pub(crate) fn ordered_entries_hints(app: &App) -> String {
    ORDERED_ENTRIES_KEYS
        .iter()
        .filter_map(|action| (action.hint)(app))
        .collect::<Vec<_>>()
        .join("   ")
}

fn is_numeric_input_char(c: char) -> bool {
    c.is_ascii_digit() || c == '.' || c == '-'
}

fn handle_ordered_create_key(app: &mut App, code: KeyCode) -> Option<Action> {
    match code {
        KeyCode::Tab | KeyCode::BackTab => {
            app.ordered_create_field = match app.ordered_create_field {
                OrderedCreateField::Id => OrderedCreateField::Value,
                OrderedCreateField::Value => OrderedCreateField::Id,
            };
            None
        }
        KeyCode::Enter => Some(Action::CreateOrderedEntry),
        KeyCode::Esc => {
            app.ordered_create_active = false;
            app.status.clear();
            None
        }
        _ => {
            match app.ordered_create_field {
                OrderedCreateField::Id => {
                    handle_text_field_key(&mut app.ordered_create_id, code, |_| true)
                }
                OrderedCreateField::Value => handle_text_field_key(
                    &mut app.ordered_create_value,
                    code,
                    is_numeric_input_char,
                ),
            };
            None
        }
    }
}

pub(crate) fn handle_ordered_entries_key(app: &mut App, code: KeyCode) -> Option<Action> {
    if app.ordered_create_active {
        return handle_ordered_create_key(app, code);
    }

    if app.ordered_create_choosing {
        app.ordered_create_choosing = false;
        return match code {
            KeyCode::Char('n') => {
                app.ordered_create_id.clear();
                app.ordered_create_value.clear();
                app.ordered_create_field = OrderedCreateField::Id;
                app.ordered_create_active = true;
                None
            }
            KeyCode::Char('e') => Some(Action::CreateOrderedEntryExternal),
            _ => None,
        };
    }

    if app.ordered_entries_search_active {
        return match code {
            KeyCode::Enter | KeyCode::Esc => {
                app.ordered_entries_search_active = false;
                app.ordered_entries_search.clear();
                app.status.clear();
                Some(Action::RefreshOrderedEntries)
            }
            _ => {
                if handle_text_field_key(&mut app.ordered_entries_search, code, |_| true) {
                    app.ordered_entries_selected = 0;
                }
                None
            }
        };
    }

    if let Some(result) = handle_pending_confirm(app, code) {
        return result;
    }

    let visible = app.visible_ordered_entry_indices().len();

    if let Some(result) = list_nav_key(code, &mut app.ordered_entries_selected, visible) {
        return result;
    }
    if let Some(result) = quit_key(code, app) {
        return result;
    }

    if matches!(code, KeyCode::Esc | KeyCode::Backspace | KeyCode::Char('h')) {
        if !app.ordered_entries_search.value.is_empty() {
            app.ordered_entries_search.clear();
            app.ordered_entries_selected = 0;
            app.status.clear();
            return None;
        }
        app.screen = Screen::OrderedStoreInput;
        app.status.clear();
        return None;
    }

    for action in ORDERED_ENTRIES_KEYS {
        if action.keys.contains(&code) {
            return (action.handler)(app);
        }
    }
    None
}

const ORDERED_VALUE_KEYS: &[KeyAction] = &[
    KeyAction {
        keys: &[KeyCode::Char('r')],
        hint: |_| Some("r: refresh"),
        handler: |_| Some(Action::LoadOrderedValue),
    },
    KeyAction {
        keys: &[KeyCode::Enter, KeyCode::Char('e')],
        hint: |_| Some("enter/e: edit"),
        handler: |app| {
            app.ordered_value_edit = app.ordered_value.to_string();
            app.ordered_value_editing = true;
            None
        },
    },
    KeyAction {
        keys: &[KeyCode::Char('i')],
        hint: |_| Some("i: increment"),
        handler: |app| {
            app.ordered_increment_edit.clear();
            app.ordered_increment_editing = true;
            None
        },
    },
    KeyAction {
        keys: &[KeyCode::Char('d')],
        hint: |_| Some("d: delete"),
        handler: |app| {
            app.arm_confirm(PendingConfirm::DeleteOrderedEntry);
            None
        },
    },
];

pub(crate) fn ordered_value_hints(app: &App) -> String {
    ORDERED_VALUE_KEYS
        .iter()
        .filter_map(|action| (action.hint)(app))
        .collect::<Vec<_>>()
        .join("   ")
}

pub(crate) fn handle_ordered_value_key(app: &mut App, code: KeyCode) -> Option<Action> {
    if app.ordered_value_editing {
        match code {
            KeyCode::Esc => {
                app.ordered_value_editing = false;
                app.status.clear();
            }
            KeyCode::Enter => return Some(Action::SaveOrderedValue),
            KeyCode::Backspace => {
                app.ordered_value_edit.pop();
            }
            KeyCode::Char(c) if is_numeric_input_char(c) => app.ordered_value_edit.push(c),
            _ => {}
        }
        return None;
    }

    if app.ordered_increment_editing {
        match code {
            KeyCode::Esc => {
                app.ordered_increment_editing = false;
                app.status.clear();
            }
            KeyCode::Enter => return Some(Action::IncrementOrderedEntry),
            KeyCode::Backspace => {
                app.ordered_increment_edit.pop();
            }
            KeyCode::Char(c) if is_numeric_input_char(c) => app.ordered_increment_edit.push(c),
            _ => {}
        }
        return None;
    }

    if let Some(result) = handle_pending_confirm(app, code) {
        return result;
    }
    if let Some(result) = quit_key(code, app) {
        return result;
    }
    if let Some(result) = back_key(code, app, Screen::OrderedEntries) {
        return result;
    }

    for action in ORDERED_VALUE_KEYS {
        if action.keys.contains(&code) {
            return (action.handler)(app);
        }
    }
    None
}

fn handle_tree_key(app: &mut App, code: KeyCode, modifiers: KeyModifiers) -> Option<Action> {
    let editor = app.tree_editor.as_mut().unwrap();
    if editor.editing_key() {
        match code {
            KeyCode::Esc => editor.cancel_edit(),
            KeyCode::Enter => editor.confirm_key(),
            KeyCode::Tab | KeyCode::BackTab => {
                editor.confirm_key();
                editor.edit_leaf();
            }
            _ => {
                handle_text_field_key(editor.edit_key_mut(), code, |_| true);
            }
        }
        return None;
    }

    if editor.editing() {
        match code {
            KeyCode::Esc => editor.cancel_edit(),
            KeyCode::Enter
                if modifiers.contains(KeyModifiers::ALT)
                    || modifiers.contains(KeyModifiers::SHIFT) =>
            {
                editor.edit_text_mut().insert_newline();
            }
            KeyCode::Enter => editor.confirm_edit(),
            KeyCode::Tab | KeyCode::BackTab => {
                editor.confirm_edit();
                editor.edit_key_start();
            }
            _ => {
                editor.edit_text_mut().input(key_to_input(code, modifiers));
            }
        }
        return None;
    }

    if let Some(result) = handle_pending_confirm(app, code) {
        return result;
    }

    if code == KeyCode::Char('s') && modifiers.contains(KeyModifiers::CONTROL) {
        return Some(Action::SaveTree);
    }

    let editor = app.tree_editor.as_mut().unwrap();
    if editor.pending_leader() {
        editor.set_pending_leader(false);
        return match code {
            KeyCode::Char('v') => {
                editor.edit_leaf();
                None
            }
            KeyCode::Char('k') => {
                editor.edit_key_start();
                None
            }
            KeyCode::Char('e') => Some(Action::EditTreeValueExternal),
            _ => None,
        };
    }

    for action in TREE_KEYS {
        if action.keys.contains(&code) {
            return (action.handler)(app);
        }
    }
    None
}

pub(crate) fn handle_memory_store_input_key(app: &mut App, code: KeyCode) -> Option<Action> {
    match code {
        KeyCode::Esc => {
            app.screen = Screen::UniverseChoice;
            app.status.clear();
            None
        }
        KeyCode::Backspace if app.memory_sorted_map_input.value.is_empty() => {
            app.screen = Screen::UniverseChoice;
            app.status.clear();
            None
        }
        KeyCode::Enter => {
            if app.memory_sorted_map_input.value.is_empty() {
                return None;
            }
            app.memory_sorted_map_id = app.memory_sorted_map_input.value.clone();
            app.memory_items_next_page_token = None;
            app.screen = Screen::MemoryStoreEntries;
            Some(Action::LoadMemoryItems)
        }
        _ => {
            handle_text_field_key(&mut app.memory_sorted_map_input, code, |_| true);
            None
        }
    }
}

const MEMORY_ENTRIES_KEYS: &[KeyAction] = &[
    KeyAction {
        keys: &[KeyCode::Char('n')],
        hint: |app| {
            app.memory_items_next_page_token
                .is_some()
                .then_some("n: next page")
        },
        handler: |_| Some(Action::LoadNextMemoryItemsPage),
    },
    KeyAction {
        keys: &[KeyCode::Char('p')],
        hint: |app| (app.memory_items_page_tokens.len() > 1).then_some("p: prev page"),
        handler: |_| Some(Action::LoadPrevMemoryItemsPage),
    },
    KeyAction {
        keys: &[KeyCode::Char('r')],
        hint: |_| Some("r: refresh"),
        handler: |_| Some(Action::RefreshMemoryItems),
    },
    KeyAction {
        keys: &[KeyCode::Char('/')],
        hint: |_| Some("/: search"),
        handler: |app| {
            app.memory_items_search_active = true;
            app.status = "loading all items for search...".to_string();
            Some(Action::LoadAllMemoryItemsForSearch)
        },
    },
    KeyAction {
        keys: &[KeyCode::Char('c')],
        hint: |_| Some("c: create"),
        handler: |app| {
            app.memory_create_choosing = true;
            None
        },
    },
    KeyAction {
        keys: &[KeyCode::Char(' ')],
        hint: |_| Some("space: select"),
        handler: |app| {
            app.toggle_memory_item_mark();
            None
        },
    },
    KeyAction {
        keys: &[KeyCode::Char('a')],
        hint: |_| Some("a: select all"),
        handler: |app| {
            app.toggle_select_all_memory_visible();
            None
        },
    },
    KeyAction {
        keys: &[KeyCode::Char('t')],
        hint: |app| (!app.visible_memory_item_indices().is_empty()).then_some("t: edit ttl"),
        handler: |app| {
            if app.visible_memory_item_indices().is_empty() {
                return None;
            }
            app.memory_ttl_edit.set("3600");
            app.memory_ttl_editing = true;
            None
        },
    },
    KeyAction {
        keys: &[KeyCode::Char('d')],
        hint: |app| {
            if app.visible_memory_item_indices().is_empty() && app.memory_items_marked.is_empty() {
                None
            } else if app.memory_items_marked.is_empty() {
                Some("d: delete")
            } else {
                Some("d: delete (selected)")
            }
        },
        handler: |app| {
            if !app.memory_items_marked.is_empty() {
                app.arm_confirm(PendingConfirm::BulkDeleteMemoryItems);
                return None;
            }
            if app.visible_memory_item_indices().is_empty() {
                return None;
            }
            app.arm_confirm(PendingConfirm::DeleteMemoryItem);
            None
        },
    },
    KeyAction {
        keys: &[KeyCode::Enter, KeyCode::Char('l')],
        hint: |_| Some("enter/l: view"),
        handler: |app| {
            if app.visible_memory_item_indices().is_empty() {
                return None;
            }
            Some(Action::LoadMemoryValue)
        },
    },
];

pub(crate) fn memory_store_input_hints(_app: &App) -> String {
    "type a sorted map name   enter: confirm   esc: back".to_string()
}

pub(crate) fn memory_entries_hints(app: &App) -> String {
    MEMORY_ENTRIES_KEYS
        .iter()
        .filter_map(|action| (action.hint)(app))
        .collect::<Vec<_>>()
        .join("   ")
}

const MEMORY_CREATE_KEYS: &[KeyAction] = &[
    KeyAction {
        keys: &[KeyCode::Tab, KeyCode::BackTab],
        hint: |_| Some("tab: switch field"),
        handler: |_| None,
    },
    KeyAction {
        keys: &[KeyCode::Enter],
        hint: |_| Some("enter: create"),
        handler: |_| None,
    },
    KeyAction {
        keys: &[KeyCode::Char('t')],
        hint: |app| {
            (app.memory_create_field == MemoryCreateField::Value)
                .then_some("ctrl+t: tree edit value")
        },
        handler: |_| None,
    },
    KeyAction {
        keys: &[KeyCode::Esc],
        hint: |_| Some("esc: cancel"),
        handler: |_| None,
    },
];

pub(crate) fn memory_create_hints(app: &App) -> String {
    MEMORY_CREATE_KEYS
        .iter()
        .filter_map(|action| (action.hint)(app))
        .collect::<Vec<_>>()
        .join("   ")
}

fn handle_memory_create_key(
    app: &mut App,
    code: KeyCode,
    modifiers: KeyModifiers,
) -> Option<Action> {
    if app.memory_create_field == MemoryCreateField::Value
        && code == KeyCode::Char('t')
        && modifiers.contains(KeyModifiers::CONTROL)
    {
        app.enter_tree_mode_for(TreeTarget::MemoryCreate);
        return None;
    }

    match code {
        KeyCode::Tab | KeyCode::BackTab => {
            app.memory_create_field = match app.memory_create_field {
                MemoryCreateField::Id => MemoryCreateField::Value,
                MemoryCreateField::Value => MemoryCreateField::Ttl,
                MemoryCreateField::Ttl => MemoryCreateField::Id,
            };
            None
        }
        KeyCode::Enter => Some(Action::CreateMemoryItem),
        KeyCode::Esc => {
            app.memory_create_active = false;
            app.status.clear();
            None
        }
        _ => {
            match app.memory_create_field {
                MemoryCreateField::Id => {
                    handle_text_field_key(&mut app.memory_create_id, code, |_| true)
                }
                MemoryCreateField::Value => {
                    handle_text_field_key(&mut app.memory_create_value, code, |_| true)
                }
                MemoryCreateField::Ttl => {
                    handle_text_field_key(&mut app.memory_create_ttl, code, |c| c.is_ascii_digit())
                }
            };
            None
        }
    }
}

pub(crate) fn handle_memory_entries_key(
    app: &mut App,
    code: KeyCode,
    modifiers: KeyModifiers,
) -> Option<Action> {
    if app.tree_editor.is_some() {
        return handle_tree_key(app, code, modifiers);
    }

    if app.memory_create_active {
        return handle_memory_create_key(app, code, modifiers);
    }

    if app.memory_create_choosing {
        app.memory_create_choosing = false;
        return match code {
            KeyCode::Char('n') => {
                app.memory_create_id.clear();
                app.memory_create_value.clear();
                app.memory_create_ttl.set("3600");
                app.memory_create_field = MemoryCreateField::Id;
                app.memory_create_active = true;
                None
            }
            KeyCode::Char('e') => Some(Action::CreateMemoryItemExternal),
            _ => None,
        };
    }

    if app.memory_ttl_editing {
        return match code {
            KeyCode::Enter => Some(Action::SaveMemoryTtl),
            KeyCode::Esc => {
                app.memory_ttl_editing = false;
                app.status.clear();
                None
            }
            _ => {
                handle_text_field_key(&mut app.memory_ttl_edit, code, |c| c.is_ascii_digit());
                None
            }
        };
    }

    if app.memory_items_search_active {
        return match code {
            KeyCode::Enter | KeyCode::Esc => {
                app.memory_items_search_active = false;
                app.memory_items_search.clear();
                app.status.clear();
                Some(Action::RefreshMemoryItems)
            }
            _ => {
                if handle_text_field_key(&mut app.memory_items_search, code, |_| true) {
                    app.memory_items_selected = 0;
                }
                None
            }
        };
    }

    if let Some(result) = handle_pending_confirm(app, code) {
        return result;
    }

    let visible = app.visible_memory_item_indices().len();

    if let Some(result) = list_nav_key(code, &mut app.memory_items_selected, visible) {
        return result;
    }
    if let Some(result) = quit_key(code, app) {
        return result;
    }

    if matches!(code, KeyCode::Esc | KeyCode::Backspace | KeyCode::Char('h')) {
        if !app.memory_items_search.value.is_empty() {
            app.memory_items_search.clear();
            app.memory_items_selected = 0;
            app.status.clear();
            return None;
        }
        app.screen = Screen::MemoryStoreInput;
        app.status.clear();
        return None;
    }

    for action in MEMORY_ENTRIES_KEYS {
        if action.keys.contains(&code) {
            return (action.handler)(app);
        }
    }
    None
}
