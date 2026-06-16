use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui_which_key::{Key, Keymap};
pub(crate) use ratatui_which_key::WhichKeyState;

use crate::app::{
    Action, App, EntriesCreateField, MemoryCreateField, MessagingField, OrderedCreateField,
    OrderedInputField, PendingConfirm, Screen, TextField, TreeTarget, ValueSource,
    SERVICE_DATA_STORES, SERVICE_MEMORY_STORES, SERVICE_MESSAGING, SERVICE_ORDERED_DATA_STORES,
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

/// A single `key: description` hint, the same shape as a which-key binding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct HintEntry {
    pub key: std::borrow::Cow<'static, str>,
    pub desc: std::borrow::Cow<'static, str>,
}

impl HintEntry {
    pub(crate) const fn new(key: &'static str, desc: &'static str) -> Self {
        Self { key: std::borrow::Cow::Borrowed(key), desc: std::borrow::Cow::Borrowed(desc) }
    }
}

/// Join hints the same way `hint_bar` joins which-key bindings.
pub(crate) fn render_hints(hints: &[HintEntry]) -> String {
    hints
        .iter()
        .map(|h| format!("{}: {}", h.key, h.desc))
        .collect::<Vec<_>>()
        .join("   ")
}

/// Join several groups of hints into one hint line.
pub(crate) fn join_hints(groups: &[&[HintEntry]]) -> String {
    render_hints(&groups.iter().flat_map(|g| g.iter().cloned()).collect::<Vec<_>>())
}

pub(crate) const ENTER_CONFIRM: HintEntry = HintEntry::new("enter", "confirm");
const ENTER_ESC_CONFIRM: HintEntry = HintEntry::new("enter/esc", "confirm");
pub(crate) const ESC_CANCEL: HintEntry = HintEntry::new("esc", "cancel");
const ESC_BACK: HintEntry = HintEntry::new("esc", "back");
const TAB_SWITCH_FIELD: HintEntry = HintEntry::new("tab", "switch field");
pub(crate) const ANY_OTHER_KEY_CANCEL: HintEntry = HintEntry::new("any other key", "cancel");

/// Movement/scroll/quit hints for list-driven screens that handle these keys
/// as pre-dispatch shortcuts rather than going through the which-key keymap.
pub(crate) const MOVE: &[HintEntry] = &[HintEntry::new("↑/↓ or j/k", "move")];
pub(crate) const SCROLL: &[HintEntry] = &[
    HintEntry::new("↑/↓ or j/k", "scroll"),
    HintEntry::new("pgup/pgdn", "scroll x10"),
];
pub(crate) const QUIT: &[HintEntry] = &[HintEntry::new("q", "quit")];
pub(crate) const BACK_QUIT: &[HintEntry] =
    &[HintEntry::new("esc/h", "back"), HintEntry::new("q", "quit")];

/// Hints for screens that are in a free-text-input mode: these consume every
/// key via `handle_text_field_key`, so they're not which-key bindings, just
/// static instructions for enter/esc.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum InputHint {
    EditText,
    SearchById,
    SearchByIdOrName,
    SearchByIdOrUsername,
    UniverseInput,
    StoreInput,
    CreateChoosing,
    TtlEdit,
    Messaging,
    OrderedStoreInput,
    OrderedCreateActive,
    AmountEdit,
    MemoryStoreInput,
    TreeLeaderMenu,
}

const TYPE_TO_EDIT: HintEntry = HintEntry::new("type", "edit value");
const TYPE_SEARCH_ID: HintEntry = HintEntry::new("type", "search by id");
const TYPE_SEARCH_ID_OR_NAME: HintEntry = HintEntry::new("type", "search by id or name");
const TYPE_SEARCH_ID_OR_USERNAME: HintEntry =
    HintEntry::new("type", "search by id or username");
const TYPE_UNIVERSE_ID: HintEntry = HintEntry::new("type", "universe id");
const TYPE_STORE_ID: HintEntry = HintEntry::new("type", "data store id");
const TYPE_TTL: HintEntry = HintEntry::new("type", "ttl seconds");
const TYPE_AMOUNT: HintEntry = HintEntry::new("type", "amount");
const N_FORM: HintEntry = HintEntry::new("n", "form");
const E_EDITOR: HintEntry = HintEntry::new("e", "$EDITOR");
const ENTER_CONTINUE: HintEntry = HintEntry::new("enter", "continue");
const ENTER_CREATE: HintEntry = HintEntry::new("enter", "create");
const ENTER_PUBLISH: HintEntry = HintEntry::new("enter", "publish");
const TYPE_SORTED_MAP_NAME: HintEntry = HintEntry::new("type", "sorted map name");
const V_EDIT_VALUE: HintEntry = HintEntry::new("v", "edit value");
const K_EDIT_KEY: HintEntry = HintEntry::new("k", "edit key");
const E_EDIT_EDITOR: HintEntry = HintEntry::new("e", "edit in $EDITOR");

const EDIT_TEXT_HINTS: &[HintEntry] = &[TYPE_TO_EDIT, ENTER_CONFIRM, ESC_CANCEL];
const SEARCH_ID_HINTS: &[HintEntry] = &[TYPE_SEARCH_ID, ENTER_ESC_CONFIRM];
const SEARCH_ID_OR_NAME_HINTS: &[HintEntry] = &[TYPE_SEARCH_ID_OR_NAME, ENTER_ESC_CONFIRM];
const SEARCH_ID_OR_USERNAME_HINTS: &[HintEntry] =
    &[TYPE_SEARCH_ID_OR_USERNAME, ENTER_ESC_CONFIRM];
const UNIVERSE_INPUT_HINTS: &[HintEntry] = &[TYPE_UNIVERSE_ID, ENTER_CONFIRM, ESC_BACK];
const STORE_INPUT_HINTS: &[HintEntry] = &[TYPE_STORE_ID, ENTER_CONTINUE, ESC_CANCEL];
const CREATE_CHOOSING_HINTS: &[HintEntry] = &[N_FORM, E_EDITOR, ESC_CANCEL];
const TTL_EDIT_HINTS: &[HintEntry] = &[TYPE_TTL, ENTER_CONFIRM, ESC_CANCEL];
const MESSAGING_HINTS: &[HintEntry] = &[TAB_SWITCH_FIELD, ENTER_PUBLISH, ESC_BACK];
const ORDERED_STORE_INPUT_HINTS: &[HintEntry] = &[TAB_SWITCH_FIELD, ENTER_CONFIRM, ESC_BACK];
const ORDERED_CREATE_ACTIVE_HINTS: &[HintEntry] = &[TAB_SWITCH_FIELD, ENTER_CREATE, ESC_CANCEL];
const AMOUNT_EDIT_HINTS: &[HintEntry] = &[TYPE_AMOUNT, ENTER_CONFIRM, ESC_CANCEL];
const MEMORY_STORE_INPUT_HINTS: &[HintEntry] = &[TYPE_SORTED_MAP_NAME, ENTER_CONFIRM, ESC_BACK];
const TREE_LEADER_MENU_HINTS: &[HintEntry] =
    &[V_EDIT_VALUE, K_EDIT_KEY, E_EDIT_EDITOR, ANY_OTHER_KEY_CANCEL];

impl InputHint {
    pub(crate) fn entries(self) -> &'static [HintEntry] {
        match self {
            InputHint::EditText => EDIT_TEXT_HINTS,
            InputHint::SearchById => SEARCH_ID_HINTS,
            InputHint::SearchByIdOrName => SEARCH_ID_OR_NAME_HINTS,
            InputHint::SearchByIdOrUsername => SEARCH_ID_OR_USERNAME_HINTS,
            InputHint::UniverseInput => UNIVERSE_INPUT_HINTS,
            InputHint::StoreInput => STORE_INPUT_HINTS,
            InputHint::CreateChoosing => CREATE_CHOOSING_HINTS,
            InputHint::TtlEdit => TTL_EDIT_HINTS,
            InputHint::Messaging => MESSAGING_HINTS,
            InputHint::OrderedStoreInput => ORDERED_STORE_INPUT_HINTS,
            InputHint::OrderedCreateActive => ORDERED_CREATE_ACTIVE_HINTS,
            InputHint::AmountEdit => AMOUNT_EDIT_HINTS,
            InputHint::MemoryStoreInput => MEMORY_STORE_INPUT_HINTS,
            InputHint::TreeLeaderMenu => TREE_LEADER_MENU_HINTS,
        }
    }
}

impl std::fmt::Display for InputHint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", render_hints(self.entries()))
    }
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

pub(crate) type Keys = WhichKeyState<KeyEvent, Scope, Act, Category>;

pub(crate) fn bind(keymap: &mut Keymap<KeyEvent, Scope, Act, Category>, code: KeyCode, act: Act, scope: Scope) {
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
pub(crate) fn dispatch(app: &mut App, scope: Scope, code: KeyCode, modifiers: KeyModifiers) -> Option<Action> {
    app.which_key.set_scope(scope);
    let act = app.which_key.handle_key(KeyEvent::new(code, modifiers))?;
    (act.handler)(app)
}

/// Hint entries for `scope` from the which-key keymap, deduped by description.
pub(crate) fn hint_bar_entries(app: &App, scope: Scope) -> Vec<HintEntry> {
    let mut keys = app.which_key.clone();
    keys.set_scope(scope);

    let mut hints: Vec<HintEntry> = Vec::new();
    for binding in keys
        .current_bindings()
        .into_iter()
        .flat_map(|group| group.bindings.into_iter())
    {
        match hints.iter_mut().find(|h| h.desc == binding.description) {
            Some(hint) => {
                let key = hint.key.to_mut();
                key.push('/');
                key.push_str(&binding.key.display());
            }
            None => hints.push(HintEntry {
                key: std::borrow::Cow::Owned(binding.key.display()),
                desc: std::borrow::Cow::Owned(binding.description),
            }),
        }
    }

    hints
}

pub(crate) fn build_keymap() -> Keymap<KeyEvent, Scope, Act, Category> {
    let mut keymap = Keymap::new();

    // Menu (bindings delegated to screens::menu)
    crate::screens::menu::bind_keys(&mut keymap);

    // UniverseChoice / UniverseSelect (delegated to screen modules)
    crate::screens::universe_choice::bind_keys(&mut keymap);
    crate::screens::universe_select::bind_keys(&mut keymap);

    // Stores
    crate::screens::stores::bind_keys(&mut keymap);

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
    crate::screens::ordered_entries::bind_keys(&mut keymap);

    // OrderedValue
    crate::screens::ordered_value::bind_keys(&mut keymap);

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

pub(crate) fn enter_service(app: &mut App) -> Option<Action> {
    match app.menu.pending_service {
        SERVICE_DATA_STORES => {
            app.screen = Screen::Stores;
            Some(Action::LoadStores)
        }
        SERVICE_MESSAGING => {
            app.messaging.reset();
            app.status.clear();
            app.screen = Screen::Messaging;
            app.resolve_current_universe_name();
            None
        }
        SERVICE_ORDERED_DATA_STORES => {
            app.ordered_store_input.reset();
            app.status.clear();
            app.screen = Screen::OrderedStoreInput;
            None
        }
        SERVICE_MEMORY_STORES => {
            app.memory_store_input.reset();
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

pub(crate) fn handle_text_field_key(
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

pub(crate) fn list_nav_key(code: KeyCode, selected: &mut usize, len: usize) -> Option<Option<Action>> {
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

pub(crate) fn quit_key(code: KeyCode, app: &mut App) -> Option<Option<Action>> {
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

pub(crate) fn handle_pending_confirm(app: &mut App, code: KeyCode) -> Option<Option<Action>> {
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

pub(crate) fn back_key(code: KeyCode, app: &mut App, screen: Screen) -> Option<Option<Action>> {
    match code {
        KeyCode::Esc | KeyCode::Backspace | KeyCode::Char('h') => {
            app.screen = screen;
            app.status.clear();
            Some(None)
        }
        _ => None,
    }
}

pub(crate) fn handle_menu_key(app: &mut App, code: KeyCode, mods: KeyModifiers) -> Option<Action> {
    crate::screens::menu::handle_key(app, code, mods)
}

pub(crate) fn handle_universe_choice_key(app: &mut App, code: KeyCode, mods: KeyModifiers) -> Option<Action> {
    crate::screens::universe_choice::handle_key(app, code, mods)
}

pub(crate) fn handle_universe_select_key(app: &mut App, code: KeyCode, mods: KeyModifiers) -> Option<Action> {
    crate::screens::universe_select::handle_key(app, code, mods)
}

pub(crate) fn handle_universe_input_key(app: &mut App, code: KeyCode, mods: KeyModifiers) -> Option<Action> {
    crate::screens::universe_input::handle_key(app, code, mods)
}

pub(crate) fn handle_messaging_key(app: &mut App, code: KeyCode, mods: KeyModifiers) -> Option<Action> {
    crate::screens::messaging::handle_key(app, code, mods)
}

struct KeyAction {
    hint: fn(&App) -> Option<HintEntry>,
}

pub(crate) fn handle_stores_key(app: &mut App, code: KeyCode, mods: KeyModifiers) -> Option<Action> {
    crate::screens::stores::handle_key(app, code, mods)
}

const ENTRIES_CREATE_KEYS: &[KeyAction] = &[
    KeyAction { hint: |_| Some(TAB_SWITCH_FIELD) },
    KeyAction { hint: |_| Some(HintEntry::new("enter", "create")) },
    KeyAction {
        hint: |app| {
            (app.entries_create_field == EntriesCreateField::Value)
                .then_some(HintEntry::new("ctrl+t", "tree edit value"))
        },
    },
    KeyAction { hint: |_| Some(ESC_CANCEL) },
];

pub(crate) fn entries_create_hints(app: &App) -> String {
    render_hints(
        &ENTRIES_CREATE_KEYS
            .iter()
            .filter_map(|action| (action.hint)(app))
            .collect::<Vec<_>>(),
    )
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

pub(crate) fn handle_ordered_store_input_key(app: &mut App, code: KeyCode, mods: KeyModifiers) -> Option<Action> {
    crate::screens::ordered_store_input::handle_key(app, code, mods)
}

pub(crate) fn is_numeric_input_char(c: char) -> bool {
    c.is_ascii_digit() || c == '.' || c == '-'
}

pub(crate) fn handle_ordered_entries_key(app: &mut App, code: KeyCode, mods: KeyModifiers) -> Option<Action> {
    crate::screens::ordered_entries::handle_key(app, code, mods)
}

pub(crate) fn handle_ordered_value_key(app: &mut App, code: KeyCode, mods: KeyModifiers) -> Option<Action> {
    crate::screens::ordered_value::handle_key(app, code, mods)
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

pub(crate) fn handle_memory_store_input_key(app: &mut App, code: KeyCode, mods: KeyModifiers) -> Option<Action> {
    crate::screens::memory_store_input::handle_key(app, code, mods)
}

const MEMORY_CREATE_KEYS: &[KeyAction] = &[
    KeyAction { hint: |_| Some(TAB_SWITCH_FIELD) },
    KeyAction { hint: |_| Some(HintEntry::new("enter", "create")) },
    KeyAction {
        hint: |app| {
            (app.memory_create_field == MemoryCreateField::Value)
                .then_some(HintEntry::new("ctrl+t", "tree edit value"))
        },
    },
    KeyAction { hint: |_| Some(ESC_CANCEL) },
];

pub(crate) fn memory_create_hints(app: &App) -> String {
    render_hints(
        &MEMORY_CREATE_KEYS
            .iter()
            .filter_map(|action| (action.hint)(app))
            .collect::<Vec<_>>(),
    )
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
