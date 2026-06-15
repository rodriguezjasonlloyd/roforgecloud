use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui_which_key::{Key, Keymap};
pub(crate) use ratatui_which_key::WhichKeyState;

use crate::app::{
    Action, App, EntriesCreateField, MemoryCreateField, MessagingField, OrderedCreateField,
    OrderedInputField, PendingConfirm, Screen, TextField, TreeTarget, ValueSource, SERVICE_ACCOUNT,
    SERVICE_DATA_STORES, SERVICE_MEMORY_STORES, SERVICE_MESSAGING, SERVICE_ORDERED_DATA_STORES,
    UNIVERSE_CHOICE_ENTER_ID, UNIVERSE_CHOICE_ITEMS, UNIVERSE_CHOICE_LIST_ALL,
};

/// Which-key scope: one variant per screen that has a key dispatch table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Scope {
    Menu,
    UniverseChoice,
    UniverseSelect,
    Stores,
    Entries,
    Value,
    Tree,
    OrderedEntries,
    OrderedValue,
    MemoryEntries,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Category {
    General,
}

impl std::fmt::Display for Category {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "general")
    }
}

/// A key-bound action: a short description (shown in the hint bar and
/// which-key popup) plus the handler that runs when the key is pressed.
#[derive(Clone, Copy)]
pub(crate) struct Act {
    pub desc: &'static str,
    pub handler: fn(&mut App) -> Option<Action>,
}

impl std::fmt::Display for Act {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.desc)
    }
}

/// Newtype around our crossterm version's `KeyEvent`, needed because
/// `ratatui-which-key`'s built-in `Key` impl targets its own crossterm
/// version (which may differ from the workspace's).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct AppKey(pub KeyEvent);

impl Key for AppKey {
    fn display(&self) -> String {
        if self.0.modifiers.contains(KeyModifiers::CONTROL) {
            if let KeyCode::Char(c) = self.0.code {
                return format!("<C-{}>", c.to_ascii_lowercase());
            }
        }
        if self.0.modifiers.contains(KeyModifiers::ALT) {
            if let KeyCode::Char(c) = self.0.code {
                return format!("<M-{}>", c.to_ascii_lowercase());
            }
        }
        match self.0.code {
            KeyCode::Char(' ') => "Space".to_string(),
            KeyCode::Char(c) => c.to_string(),
            KeyCode::Tab => "Tab".to_string(),
            KeyCode::Enter => "Enter".to_string(),
            KeyCode::Backspace => "Backspace".to_string(),
            KeyCode::Esc => "Esc".to_string(),
            KeyCode::Up => "Up".to_string(),
            KeyCode::Down => "Down".to_string(),
            KeyCode::Left => "Left".to_string(),
            KeyCode::Right => "Right".to_string(),
            KeyCode::Home => "Home".to_string(),
            KeyCode::End => "End".to_string(),
            KeyCode::PageUp => "PageUp".to_string(),
            KeyCode::PageDown => "PageDown".to_string(),
            KeyCode::F(n) => format!("F{n}"),
            _ => "?".to_string(),
        }
    }

    fn is_backspace(&self) -> bool {
        matches!(self.0.code, KeyCode::Backspace)
    }

    fn space() -> Self {
        AppKey(KeyEvent::new(KeyCode::Char(' '), KeyModifiers::empty()))
    }

    fn from_char(c: char) -> Option<Self> {
        Some(AppKey(KeyEvent::new(KeyCode::Char(c), KeyModifiers::empty())))
    }

    fn from_special_name(name: &str) -> Option<Self> {
        let lower = name.to_ascii_lowercase();

        if lower.starts_with("c-") && lower.len() == 3 {
            let c = lower.chars().nth(2)?;
            return Some(AppKey(KeyEvent::new(KeyCode::Char(c), KeyModifiers::CONTROL)));
        }
        if lower.starts_with("m-") && lower.len() == 3 {
            let c = lower.chars().nth(2)?;
            return Some(AppKey(KeyEvent::new(KeyCode::Char(c), KeyModifiers::ALT)));
        }

        let code = match lower.as_str() {
            "tab" => KeyCode::Tab,
            "enter" => KeyCode::Enter,
            "bs" | "backspace" => KeyCode::Backspace,
            "esc" | "escape" => KeyCode::Esc,
            "up" => KeyCode::Up,
            "down" => KeyCode::Down,
            "left" => KeyCode::Left,
            "right" => KeyCode::Right,
            "home" => KeyCode::Home,
            "end" => KeyCode::End,
            "pgup" | "pageup" => KeyCode::PageUp,
            "pgdn" | "pagedown" => KeyCode::PageDown,
            "space" => KeyCode::Char(' '),
            "lt" => KeyCode::Char('<'),
            "gt" => KeyCode::Char('>'),
            s if s.starts_with('f') && s.len() > 1 => {
                let num: u8 = s[1..].parse().ok()?;
                if !(1..=12).contains(&num) {
                    return None;
                }
                KeyCode::F(num)
            }
            _ => return None,
        };

        Some(AppKey(KeyEvent::new(code, KeyModifiers::empty())))
    }
}

pub(crate) type Keys = WhichKeyState<AppKey, Scope, Act, Category>;

fn bind(keymap: &mut Keymap<AppKey, Scope, Act, Category>, code: KeyCode, act: Act, scope: Scope) {
    let seq = match code {
        KeyCode::Char(c) => c.to_string(),
        KeyCode::Enter => "<enter>".to_string(),
        KeyCode::Esc => "<esc>".to_string(),
        KeyCode::Up => "<up>".to_string(),
        KeyCode::Down => "<down>".to_string(),
        KeyCode::PageUp => "<pgup>".to_string(),
        KeyCode::PageDown => "<pgdn>".to_string(),
        KeyCode::Tab => "<tab>".to_string(),
        KeyCode::Backspace => "<bs>".to_string(),
        _ => return,
    };
    keymap.bind(&seq, act, Category::General, scope);
}

/// Dispatch the next key through the which-key keymap for `scope`, falling
/// back to `None` (no action) if the key has no binding in that scope.
fn dispatch(app: &mut App, scope: Scope, code: KeyCode, modifiers: KeyModifiers) -> Option<Action> {
    app.which_key.set_scope(scope);
    let act = app
        .which_key
        .handle_key(AppKey(KeyEvent::new(code, modifiers)))?;
    (act.handler)(app)
}

/// Render the bottom hint bar for `scope` from the which-key keymap.
pub(crate) fn hint_bar(app: &App, scope: Scope) -> String {
    let mut keys = app.which_key.clone();
    keys.set_scope(scope);
    keys.current_bindings()
        .iter()
        .flat_map(|group| group.bindings.iter())
        .map(|binding| format!("{}: {}", binding.key.display(), binding.description))
        .collect::<Vec<_>>()
        .join("   ")
}

pub(crate) fn build_keymap() -> Keymap<AppKey, Scope, Act, Category> {
    let mut keymap = Keymap::new();

    // Menu
    bind(
        &mut keymap,
        KeyCode::Enter,
        Act { desc: "open", handler: menu_open },
        Scope::Menu,
    );
    bind(&mut keymap, KeyCode::Char('l'), Act { desc: "open", handler: menu_open }, Scope::Menu);

    // UniverseChoice
    bind(
        &mut keymap,
        KeyCode::Enter,
        Act { desc: "select", handler: universe_choice_select },
        Scope::UniverseChoice,
    );
    bind(
        &mut keymap,
        KeyCode::Char('l'),
        Act { desc: "select", handler: universe_choice_select },
        Scope::UniverseChoice,
    );

    // UniverseSelect
    bind(
        &mut keymap,
        KeyCode::Char('/'),
        Act {
            desc: "search",
            handler: |app| {
                app.universe_search_active = true;
                None
            },
        },
        Scope::UniverseSelect,
    );
    bind(
        &mut keymap,
        KeyCode::Enter,
        Act { desc: "select", handler: universe_select_choose },
        Scope::UniverseSelect,
    );
    bind(
        &mut keymap,
        KeyCode::Char('l'),
        Act { desc: "select", handler: universe_select_choose },
        Scope::UniverseSelect,
    );

    // Stores
    bind(&mut keymap, KeyCode::Enter, Act { desc: "open", handler: stores_open }, Scope::Stores);
    bind(&mut keymap, KeyCode::Char('l'), Act { desc: "open", handler: stores_open }, Scope::Stores);
    bind(
        &mut keymap,
        KeyCode::Char(' '),
        Act {
            desc: "select",
            handler: |app| {
                app.toggle_store_mark();
                None
            },
        },
        Scope::Stores,
    );
    bind(
        &mut keymap,
        KeyCode::Char('a'),
        Act {
            desc: "select all",
            handler: |app| {
                app.toggle_select_all_stores();
                None
            },
        },
        Scope::Stores,
    );
    bind(
        &mut keymap,
        KeyCode::Char('r'),
        Act { desc: "refresh", handler: |_| Some(Action::LoadStores) },
        Scope::Stores,
    );
    bind(
        &mut keymap,
        KeyCode::Char('c'),
        Act {
            desc: "create entry in new store",
            handler: |app| {
                app.stores_new_id.clear();
                app.stores_new_active = true;
                None
            },
        },
        Scope::Stores,
    );
    bind(
        &mut keymap,
        KeyCode::Char('d'),
        Act { desc: "delete", handler: stores_delete },
        Scope::Stores,
    );
    bind(
        &mut keymap,
        KeyCode::Char('u'),
        Act { desc: "undelete", handler: stores_undelete },
        Scope::Stores,
    );

    // Entries
    bind(
        &mut keymap,
        KeyCode::Char('n'),
        Act { desc: "next page", handler: |_| Some(Action::LoadNextEntriesPage) },
        Scope::Entries,
    );
    bind(
        &mut keymap,
        KeyCode::Char('p'),
        Act { desc: "prev page", handler: |_| Some(Action::LoadPrevEntriesPage) },
        Scope::Entries,
    );
    bind(
        &mut keymap,
        KeyCode::Char('r'),
        Act { desc: "refresh", handler: |_| Some(Action::RefreshEntries) },
        Scope::Entries,
    );
    bind(
        &mut keymap,
        KeyCode::Char('/'),
        Act {
            desc: "search",
            handler: |app| {
                app.entries_search_active = true;
                app.status = "loading all entries for search...".to_string();
                Some(Action::LoadAllEntriesForSearch)
            },
        },
        Scope::Entries,
    );
    bind(
        &mut keymap,
        KeyCode::Char('c'),
        Act {
            desc: "create",
            handler: |app| {
                app.entries_create_choosing = true;
                None
            },
        },
        Scope::Entries,
    );
    bind(
        &mut keymap,
        KeyCode::Char(' '),
        Act {
            desc: "select",
            handler: |app| {
                app.toggle_entry_mark();
                None
            },
        },
        Scope::Entries,
    );
    bind(
        &mut keymap,
        KeyCode::Char('a'),
        Act {
            desc: "select all",
            handler: |app| {
                app.toggle_select_all_visible();
                None
            },
        },
        Scope::Entries,
    );
    bind(
        &mut keymap,
        KeyCode::Char('d'),
        Act { desc: "delete", handler: entries_delete },
        Scope::Entries,
    );
    bind(
        &mut keymap,
        KeyCode::Enter,
        Act { desc: "view", handler: entries_view },
        Scope::Entries,
    );
    bind(
        &mut keymap,
        KeyCode::Char('l'),
        Act { desc: "view", handler: entries_view },
        Scope::Entries,
    );

    // Value
    bind(
        &mut keymap,
        KeyCode::Char('r'),
        Act { desc: "refresh", handler: |_| Some(Action::LoadValue) },
        Scope::Value,
    );
    bind(
        &mut keymap,
        KeyCode::Enter,
        Act {
            desc: "tree edit",
            handler: |app| {
                app.enter_tree_mode();
                None
            },
        },
        Scope::Value,
    );
    bind(
        &mut keymap,
        KeyCode::Char('l'),
        Act {
            desc: "tree edit",
            handler: |app| {
                app.enter_tree_mode();
                None
            },
        },
        Scope::Value,
    );
    bind(
        &mut keymap,
        KeyCode::Char('e'),
        Act { desc: "edit in $EDITOR", handler: |_| Some(Action::EditValueExternal) },
        Scope::Value,
    );
    bind(
        &mut keymap,
        KeyCode::Char('d'),
        Act {
            desc: "delete",
            handler: |app| {
                let pending = match app.value_source {
                    ValueSource::DataStore => PendingConfirm::DeleteEntry,
                    ValueSource::MemoryStoreSortedMap => PendingConfirm::DeleteMemoryItem,
                };
                app.arm_confirm(pending);
                None
            },
        },
        Scope::Value,
    );
    bind(
        &mut keymap,
        KeyCode::Char('t'),
        Act {
            desc: "edit ttl",
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
        Scope::Value,
    );
    bind(
        &mut keymap,
        KeyCode::Up,
        Act { desc: "scroll up", handler: value_scroll_up },
        Scope::Value,
    );
    bind(
        &mut keymap,
        KeyCode::Char('k'),
        Act { desc: "scroll up", handler: value_scroll_up },
        Scope::Value,
    );
    bind(
        &mut keymap,
        KeyCode::Down,
        Act { desc: "scroll down", handler: value_scroll_down },
        Scope::Value,
    );
    bind(
        &mut keymap,
        KeyCode::Char('j'),
        Act { desc: "scroll down", handler: value_scroll_down },
        Scope::Value,
    );
    bind(
        &mut keymap,
        KeyCode::PageUp,
        Act {
            desc: "scroll up x10",
            handler: |app| {
                app.value_scroll = app.value_scroll.saturating_sub(10);
                None
            },
        },
        Scope::Value,
    );
    bind(
        &mut keymap,
        KeyCode::PageDown,
        Act {
            desc: "scroll down x10",
            handler: |app| {
                let max_scroll = app.max_value_scroll();
                app.value_scroll = (app.value_scroll + 10).min(max_scroll);
                None
            },
        },
        Scope::Value,
    );

    // Tree
    keymap.bind(
        "<C-s>",
        Act { desc: "save", handler: |_| Some(Action::SaveTree) },
        Category::General,
        Scope::Tree,
    );
    bind(
        &mut keymap,
        KeyCode::Up,
        Act {
            desc: "move up",
            handler: |app| {
                app.tree_editor.as_mut().unwrap().move_cursor(-1);
                None
            },
        },
        Scope::Tree,
    );
    bind(
        &mut keymap,
        KeyCode::Char('k'),
        Act {
            desc: "move up",
            handler: |app| {
                app.tree_editor.as_mut().unwrap().move_cursor(-1);
                None
            },
        },
        Scope::Tree,
    );
    bind(
        &mut keymap,
        KeyCode::Down,
        Act {
            desc: "move down",
            handler: |app| {
                app.tree_editor.as_mut().unwrap().move_cursor(1);
                None
            },
        },
        Scope::Tree,
    );
    bind(
        &mut keymap,
        KeyCode::Char('j'),
        Act {
            desc: "move down",
            handler: |app| {
                app.tree_editor.as_mut().unwrap().move_cursor(1);
                None
            },
        },
        Scope::Tree,
    );
    bind(
        &mut keymap,
        KeyCode::Char(' '),
        Act {
            desc: "fold/unfold",
            handler: |app| {
                app.tree_editor.as_mut().unwrap().toggle();
                None
            },
        },
        Scope::Tree,
    );
    bind(
        &mut keymap,
        KeyCode::Enter,
        Act {
            desc: "edit value",
            handler: |app| {
                app.tree_editor.as_mut().unwrap().edit_leaf();
                None
            },
        },
        Scope::Tree,
    );
    bind(
        &mut keymap,
        KeyCode::Char('y'),
        Act { desc: "yank", handler: tree_yank },
        Scope::Tree,
    );
    bind(
        &mut keymap,
        KeyCode::Char('p'),
        Act { desc: "paste", handler: tree_paste },
        Scope::Tree,
    );
    bind(
        &mut keymap,
        KeyCode::Char('d'),
        Act { desc: "delete entry", handler: tree_delete_entry },
        Scope::Tree,
    );
    bind(
        &mut keymap,
        KeyCode::Char('a'),
        Act {
            desc: "add entry",
            handler: |app| {
                app.tree_editor.as_mut().unwrap().add_entry();
                None
            },
        },
        Scope::Tree,
    );
    bind(
        &mut keymap,
        KeyCode::Char('x'),
        Act {
            desc: "delete node",
            handler: |app| {
                app.tree_editor.as_mut().unwrap().delete_current();
                None
            },
        },
        Scope::Tree,
    );
    bind(
        &mut keymap,
        KeyCode::Char('e'),
        Act {
            desc: "edit (e+v/k/e)",
            handler: |app| {
                app.tree_editor.as_mut().unwrap().set_pending_leader(true);
                None
            },
        },
        Scope::Tree,
    );
    bind(
        &mut keymap,
        KeyCode::Char('r'),
        Act { desc: "refresh", handler: tree_refresh },
        Scope::Tree,
    );
    bind(
        &mut keymap,
        KeyCode::Esc,
        Act { desc: "exit tree", handler: tree_exit },
        Scope::Tree,
    );
    bind(
        &mut keymap,
        KeyCode::Char('q'),
        Act { desc: "exit tree", handler: tree_exit },
        Scope::Tree,
    );

    // OrderedEntries
    bind(
        &mut keymap,
        KeyCode::Char('n'),
        Act { desc: "next page", handler: |_| Some(Action::LoadNextOrderedEntriesPage) },
        Scope::OrderedEntries,
    );
    bind(
        &mut keymap,
        KeyCode::Char('p'),
        Act { desc: "prev page", handler: |_| Some(Action::LoadPrevOrderedEntriesPage) },
        Scope::OrderedEntries,
    );
    bind(
        &mut keymap,
        KeyCode::Char('r'),
        Act { desc: "refresh", handler: |_| Some(Action::RefreshOrderedEntries) },
        Scope::OrderedEntries,
    );
    bind(
        &mut keymap,
        KeyCode::Char('/'),
        Act {
            desc: "search",
            handler: |app| {
                app.ordered_entries_search_active = true;
                app.status = "loading all entries for search...".to_string();
                Some(Action::LoadAllOrderedEntriesForSearch)
            },
        },
        Scope::OrderedEntries,
    );
    bind(
        &mut keymap,
        KeyCode::Char('c'),
        Act {
            desc: "create",
            handler: |app| {
                app.ordered_create_choosing = true;
                None
            },
        },
        Scope::OrderedEntries,
    );
    bind(
        &mut keymap,
        KeyCode::Char(' '),
        Act {
            desc: "select",
            handler: |app| {
                app.toggle_ordered_entry_mark();
                None
            },
        },
        Scope::OrderedEntries,
    );
    bind(
        &mut keymap,
        KeyCode::Char('a'),
        Act {
            desc: "select all",
            handler: |app| {
                app.toggle_select_all_ordered_visible();
                None
            },
        },
        Scope::OrderedEntries,
    );
    bind(
        &mut keymap,
        KeyCode::Char('d'),
        Act { desc: "delete", handler: ordered_entries_delete },
        Scope::OrderedEntries,
    );
    bind(
        &mut keymap,
        KeyCode::Enter,
        Act { desc: "view", handler: ordered_entries_view },
        Scope::OrderedEntries,
    );
    bind(
        &mut keymap,
        KeyCode::Char('l'),
        Act { desc: "view", handler: ordered_entries_view },
        Scope::OrderedEntries,
    );

    // OrderedValue
    bind(
        &mut keymap,
        KeyCode::Char('r'),
        Act { desc: "refresh", handler: |_| Some(Action::LoadOrderedValue) },
        Scope::OrderedValue,
    );
    bind(
        &mut keymap,
        KeyCode::Enter,
        Act { desc: "edit", handler: ordered_value_edit },
        Scope::OrderedValue,
    );
    bind(
        &mut keymap,
        KeyCode::Char('e'),
        Act { desc: "edit", handler: ordered_value_edit },
        Scope::OrderedValue,
    );
    bind(
        &mut keymap,
        KeyCode::Char('i'),
        Act {
            desc: "increment",
            handler: |app| {
                app.ordered_increment_edit.clear();
                app.ordered_increment_editing = true;
                None
            },
        },
        Scope::OrderedValue,
    );
    bind(
        &mut keymap,
        KeyCode::Char('d'),
        Act {
            desc: "delete",
            handler: |app| {
                app.arm_confirm(PendingConfirm::DeleteOrderedEntry);
                None
            },
        },
        Scope::OrderedValue,
    );

    // MemoryEntries
    bind(
        &mut keymap,
        KeyCode::Char('n'),
        Act { desc: "next page", handler: |_| Some(Action::LoadNextMemoryItemsPage) },
        Scope::MemoryEntries,
    );
    bind(
        &mut keymap,
        KeyCode::Char('p'),
        Act { desc: "prev page", handler: |_| Some(Action::LoadPrevMemoryItemsPage) },
        Scope::MemoryEntries,
    );
    bind(
        &mut keymap,
        KeyCode::Char('r'),
        Act { desc: "refresh", handler: |_| Some(Action::RefreshMemoryItems) },
        Scope::MemoryEntries,
    );
    bind(
        &mut keymap,
        KeyCode::Char('/'),
        Act {
            desc: "search",
            handler: |app| {
                app.memory_items_search_active = true;
                app.status = "loading all items for search...".to_string();
                Some(Action::LoadAllMemoryItemsForSearch)
            },
        },
        Scope::MemoryEntries,
    );
    bind(
        &mut keymap,
        KeyCode::Char('c'),
        Act {
            desc: "create",
            handler: |app| {
                app.memory_create_choosing = true;
                None
            },
        },
        Scope::MemoryEntries,
    );
    bind(
        &mut keymap,
        KeyCode::Char(' '),
        Act {
            desc: "select",
            handler: |app| {
                app.toggle_memory_item_mark();
                None
            },
        },
        Scope::MemoryEntries,
    );
    bind(
        &mut keymap,
        KeyCode::Char('a'),
        Act {
            desc: "select all",
            handler: |app| {
                app.toggle_select_all_memory_visible();
                None
            },
        },
        Scope::MemoryEntries,
    );
    bind(
        &mut keymap,
        KeyCode::Char('t'),
        Act {
            desc: "edit ttl",
            handler: |app| {
                if app.visible_memory_item_indices().is_empty() {
                    return None;
                }
                app.memory_ttl_edit.set("3600");
                app.memory_ttl_editing = true;
                None
            },
        },
        Scope::MemoryEntries,
    );
    bind(
        &mut keymap,
        KeyCode::Char('d'),
        Act { desc: "delete", handler: memory_entries_delete },
        Scope::MemoryEntries,
    );
    bind(
        &mut keymap,
        KeyCode::Enter,
        Act { desc: "view", handler: memory_entries_view },
        Scope::MemoryEntries,
    );
    bind(
        &mut keymap,
        KeyCode::Char('l'),
        Act { desc: "view", handler: memory_entries_view },
        Scope::MemoryEntries,
    );

    keymap
}

fn menu_open(app: &mut App) -> Option<Action> {
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
}

fn universe_choice_select(app: &mut App) -> Option<Action> {
    match app.universe_choice_selected {
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
    }
}

fn universe_select_choose(app: &mut App) -> Option<Action> {
    let visible = app.visible_universe_indices();
    let &index = visible.get(app.universe_select_selected)?;
    let universe_id = app.available_universes[index];
    app.universe_id = universe_id;
    enter_service(app)
}

fn stores_open(app: &mut App) -> Option<Action> {
    let store = app.stores.get(app.stores_selected)?;
    if store.state.as_deref().is_some_and(|s| s != "ACTIVE") {
        return None;
    }
    app.data_store_id = store.id.clone();
    app.entries_next_page_token = None;
    app.screen = Screen::Entries;
    Some(Action::LoadEntries)
}

fn stores_delete(app: &mut App) -> Option<Action> {
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
}

fn stores_undelete(app: &mut App) -> Option<Action> {
    if !app.stores_marked.is_empty() {
        app.arm_confirm(PendingConfirm::BulkUndeleteStores);
        return None;
    }
    let store = app.stores.get(app.stores_selected)?;
    if store.state.as_deref() != Some("ACTIVE") {
        return Some(Action::UndeleteDataStore);
    }
    None
}

fn entries_delete(app: &mut App) -> Option<Action> {
    if !app.entries_marked.is_empty() {
        app.arm_confirm(PendingConfirm::BulkDeleteEntries);
        return None;
    }
    if app.visible_entry_indices().is_empty() {
        return None;
    }
    app.arm_confirm(PendingConfirm::DeleteEntry);
    None
}

fn entries_view(app: &mut App) -> Option<Action> {
    if app.visible_entry_indices().is_empty() {
        return None;
    }
    app.screen = Screen::Value;
    Some(Action::LoadValue)
}

fn value_scroll_up(app: &mut App) -> Option<Action> {
    app.value_scroll = app.value_scroll.saturating_sub(1);
    None
}

fn value_scroll_down(app: &mut App) -> Option<Action> {
    let max_scroll = app.max_value_scroll();
    app.value_scroll = (app.value_scroll + 1).min(max_scroll);
    None
}

fn tree_yank(app: &mut App) -> Option<Action> {
    let mut clipboard = app.clipboard.take();
    if let Some(status) = app.tree_editor.as_ref().unwrap().yank(&mut clipboard) {
        app.status = status;
    }
    app.clipboard = clipboard;
    None
}

fn tree_paste(app: &mut App) -> Option<Action> {
    let mut clipboard = app.clipboard.take();
    if let Some(status) = app.tree_editor.as_mut().unwrap().paste(&mut clipboard) {
        app.status = status;
    }
    app.clipboard = clipboard;
    None
}

fn tree_delete_entry(app: &mut App) -> Option<Action> {
    if app.tree_target != TreeTarget::Value {
        return None;
    }
    let pending = match app.value_source {
        ValueSource::DataStore => PendingConfirm::DeleteEntry,
        ValueSource::MemoryStoreSortedMap => PendingConfirm::DeleteMemoryItem,
    };
    app.arm_confirm(pending);
    None
}

fn tree_refresh(app: &mut App) -> Option<Action> {
    if app.tree_target != TreeTarget::Value {
        return None;
    }
    if app.tree_editor.as_ref().unwrap().dirty() {
        app.arm_confirm(PendingConfirm::TreeRefresh);
        None
    } else {
        Some(Action::RefreshTree)
    }
}

fn tree_exit(app: &mut App) -> Option<Action> {
    if app.tree_editor.as_ref().unwrap().dirty() {
        app.arm_confirm(PendingConfirm::TreeQuit);
    } else {
        app.exit_tree_mode();
    }
    None
}

fn ordered_entries_delete(app: &mut App) -> Option<Action> {
    if !app.ordered_entries_marked.is_empty() {
        app.arm_confirm(PendingConfirm::BulkDeleteOrderedEntries);
        return None;
    }
    if app.visible_ordered_entry_indices().is_empty() {
        return None;
    }
    app.arm_confirm(PendingConfirm::DeleteOrderedEntry);
    None
}

fn ordered_entries_view(app: &mut App) -> Option<Action> {
    if app.visible_ordered_entry_indices().is_empty() {
        return None;
    }
    app.screen = Screen::OrderedValue;
    Some(Action::LoadOrderedValue)
}

fn ordered_value_edit(app: &mut App) -> Option<Action> {
    app.ordered_value_edit = app.ordered_value.to_string();
    app.ordered_value_editing = true;
    None
}

fn memory_entries_delete(app: &mut App) -> Option<Action> {
    if !app.memory_items_marked.is_empty() {
        app.arm_confirm(PendingConfirm::BulkDeleteMemoryItems);
        return None;
    }
    if app.visible_memory_item_indices().is_empty() {
        return None;
    }
    app.arm_confirm(PendingConfirm::DeleteMemoryItem);
    None
}

fn memory_entries_view(app: &mut App) -> Option<Action> {
    if app.visible_memory_item_indices().is_empty() {
        return None;
    }
    Some(Action::LoadMemoryValue)
}

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

pub(crate) fn handle_menu_key(app: &mut App, code: KeyCode) -> Option<Action> {
    let len = app.menu_items.len();
    if let Some(result) = list_nav_key(code, &mut app.menu_selected, len) {
        return result;
    }
    if let Some(result) = quit_key(code, app) {
        return result;
    }
    dispatch(app, Scope::Menu, code, KeyModifiers::empty())
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
    dispatch(app, Scope::UniverseChoice, code, KeyModifiers::empty())
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

    dispatch(app, Scope::UniverseSelect, code, KeyModifiers::empty())
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
    hint: fn(&App) -> Option<&'static str>,
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

    dispatch(app, Scope::Stores, code, KeyModifiers::empty())
}

const ENTRIES_CREATE_KEYS: &[KeyAction] = &[
    KeyAction {
        hint: |_| Some("tab: switch field"),
    },
    KeyAction {
        hint: |_| Some("enter: create"),
    },
    KeyAction {
        hint: |app| {
            (app.entries_create_field == EntriesCreateField::Value)
                .then_some("ctrl+t: tree edit value")
        },
    },
    KeyAction {
        hint: |_| Some("esc: cancel"),
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

    dispatch(app, Scope::Entries, code, modifiers)
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

    dispatch(app, Scope::Value, code, modifiers)
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

    dispatch(app, Scope::OrderedEntries, code, KeyModifiers::empty())
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

    dispatch(app, Scope::OrderedValue, code, KeyModifiers::empty())
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

    dispatch(app, Scope::Tree, code, modifiers)
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

pub(crate) fn memory_store_input_hints(_app: &App) -> String {
    "type a sorted map name   enter: confirm   esc: back".to_string()
}

const MEMORY_CREATE_KEYS: &[KeyAction] = &[
    KeyAction {
        hint: |_| Some("tab: switch field"),
    },
    KeyAction {
        hint: |_| Some("enter: create"),
    },
    KeyAction {
        hint: |app| {
            (app.memory_create_field == MemoryCreateField::Value)
                .then_some("ctrl+t: tree edit value")
        },
    },
    KeyAction {
        hint: |_| Some("esc: cancel"),
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

    dispatch(app, Scope::MemoryEntries, code, modifiers)
}
