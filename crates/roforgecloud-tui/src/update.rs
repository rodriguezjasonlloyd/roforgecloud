use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui_which_key::Keymap;
pub(crate) use ratatui_which_key::WhichKeyState;

use crate::screens;

use crate::app::{
    Action, App, PendingConfirm, Screen, TextField, TreeTarget, ValueSource, SERVICE_DATA_STORES,
    SERVICE_MEMORY_STORES, SERVICE_MESSAGING, SERVICE_ORDERED_DATA_STORES,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Scope {
    Menu,
    UniverseChoice,
    UniverseSelect,
    Stores,
    Entries,
    DataStoreValue,
    MemoryStoreValue,
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

pub(crate) fn bind(
    keymap: &mut Keymap<KeyEvent, Scope, Act, Category>,
    code: KeyCode,
    act: Act,
    scope: Scope,
) {
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

pub(crate) fn bind_list_nav(km: &mut Keymap<KeyEvent, Scope, Act, Category>, scope: Scope) {
    bind(
        km,
        KeyCode::Char('j'),
        Act {
            desc: "move down",
            handler: |_| None,
        },
        scope,
    );
    bind(
        km,
        KeyCode::Down,
        Act {
            desc: "move down",
            handler: |_| None,
        },
        scope,
    );
    bind(
        km,
        KeyCode::Char('k'),
        Act {
            desc: "move up",
            handler: |_| None,
        },
        scope,
    );
    bind(
        km,
        KeyCode::Up,
        Act {
            desc: "move up",
            handler: |_| None,
        },
        scope,
    );
}

pub(crate) fn bind_quit(km: &mut Keymap<KeyEvent, Scope, Act, Category>, scope: Scope) {
    bind(
        km,
        KeyCode::Char('q'),
        Act {
            desc: "quit",
            handler: do_quit,
        },
        scope,
    );
    bind(
        km,
        KeyCode::Char('?'),
        Act {
            desc: "help",
            handler: |app| {
                app.which_key.toggle();
                None
            },
        },
        scope,
    );
}

fn do_quit(app: &mut App) -> Option<Action> {
    if app.needs_quit_confirm() {
        app.arm_confirm(PendingConfirm::Quit);
    } else {
        app.should_quit = true;
    }
    None
}

pub(crate) fn dispatch(
    app: &mut App,
    scope: Scope,
    code: KeyCode,
    modifiers: KeyModifiers,
) -> Option<Action> {
    app.which_key.set_scope(scope);
    let act = app.which_key.handle_key(KeyEvent::new(code, modifiers))?;
    (act.handler)(app)
}

pub(crate) fn build_keymap() -> Keymap<KeyEvent, Scope, Act, Category> {
    let mut keymap = Keymap::new();

    for screen_def in &screens::SCREENS {
        if let Some(f) = screen_def.bind_keys {
            f(&mut keymap);
        }
    }

    // Tree
    keymap.bind(
        "<C-s>",
        Act {
            desc: "save",
            handler: |_| Some(Action::SaveTree),
        },
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
        Act {
            desc: "yank",
            handler: tree_yank,
        },
        Scope::Tree,
    );
    bind(
        &mut keymap,
        KeyCode::Char('p'),
        Act {
            desc: "paste",
            handler: tree_paste,
        },
        Scope::Tree,
    );
    bind(
        &mut keymap,
        KeyCode::Char('d'),
        Act {
            desc: "delete entry",
            handler: tree_delete_entry,
        },
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
        Act {
            desc: "refresh",
            handler: tree_refresh,
        },
        Scope::Tree,
    );
    bind(
        &mut keymap,
        KeyCode::Esc,
        Act {
            desc: "exit tree",
            handler: tree_exit,
        },
        Scope::Tree,
    );
    bind(
        &mut keymap,
        KeyCode::Char('q'),
        Act {
            desc: "exit tree",
            handler: tree_exit,
        },
        Scope::Tree,
    );

    keymap
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
    let pending = match app.value.source {
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
    use crossterm::event::{KeyEventKind, KeyEventState};
    match code {
        KeyCode::Char(c) if !accept(c) => false,
        KeyCode::Enter => false,
        code => field.input(KeyEvent {
            code,
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        }),
    }
}

pub(crate) fn list_nav_key(
    code: KeyCode,
    selected: &mut usize,
    len: usize,
) -> Option<Option<Action>> {
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

pub(crate) fn is_numeric_input_char(c: char) -> bool {
    c.is_ascii_digit() || c == '.' || c == '-'
}

pub(crate) fn handle_tree_key(
    app: &mut App,
    code: KeyCode,
    modifiers: KeyModifiers,
) -> Option<Action> {
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
