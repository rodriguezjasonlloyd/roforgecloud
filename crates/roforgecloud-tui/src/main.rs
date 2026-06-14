mod app;
mod json_highlight;
mod json_tree;
mod ui;
mod userlookup;

use std::io;
use std::time::Duration;

use clap::Parser;
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

use app::{
    Action, App, MessagingField, PendingConfirm, Screen, SERVICE_ACCOUNT, SERVICE_DATA_STORES,
    SERVICE_MESSAGING, UNIVERSE_CHOICE_ENTER_ID, UNIVERSE_CHOICE_ITEMS, UNIVERSE_CHOICE_LIST_ALL,
};
use roforgecloud_core::auth;
use roforgecloud_core::oauth::{self, OAuthClient};
use roforgecloud_core::opencloud::{Credentials, OpenCloudClient};

#[derive(Parser)]
#[command(
    name = "roforgecloud-tui",
    about = "Browse Roblox Open Cloud data stores"
)]
struct Cli {
    #[arg(long, env = "ROFORGE_API_KEY")]
    api_key: Option<String>,

    #[arg(long, env = "ROFORGE_OAUTH_CLIENT_ID", default_value = oauth::DEFAULT_CLIENT_ID)]
    client_id: String,

    /// Only needed to talk to Roblox's OAuth endpoints directly, bypassing
    /// the relay (e.g. if you registered your own OAuth app).
    #[arg(long, env = "ROFORGE_OAUTH_CLIENT_SECRET")]
    client_secret: Option<String>,

    /// OAuth relay that holds the client secret (see `worker/`). Ignored if
    /// --client-secret is set.
    #[arg(long, env = "ROFORGE_OAUTH_RELAY_URL", default_value = oauth::DEFAULT_RELAY_URL)]
    relay_url: String,

    #[arg(
        long,
        env = "ROFORGE_OAUTH_REDIRECT_URI",
        default_value = "http://localhost:8675/callback"
    )]
    redirect_uri: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _ = dotenvy::dotenv();
    let cli = Cli::parse();

    let (client, oauth, available_universes, logged_in) = build_client(&cli).await?;

    let mut stdout = io::stdout();
    enable_raw_mode()?;
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let has_api_key = cli.api_key.is_some();
    let mut app = App::new(
        client,
        has_api_key,
        oauth,
        cli.redirect_uri.clone(),
        available_universes,
        logged_in,
    );

    let result = run(&mut terminal, &mut app).await;

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

async fn build_client(
    cli: &Cli,
) -> anyhow::Result<(OpenCloudClient, Option<OAuthClient>, Vec<u64>, bool)> {
    let oauth = auth::build_oauth_client(
        cli.client_id.clone(),
        cli.client_secret.clone(),
        &cli.relay_url,
        &cli.redirect_uri,
    )?;

    if let Some(api_key) = &cli.api_key {
        return Ok((
            OpenCloudClient::new(Credentials::ApiKey(api_key.clone())),
            Some(oauth),
            Vec::new(),
            auth::is_logged_in(),
        ));
    }

    let token = auth::access_token(&oauth, &cli.redirect_uri).await?;

    let resources = oauth.token_resources(&token).await?;
    let available_universes = oauth::authorized_universe_ids(&resources);

    Ok((
        OpenCloudClient::new(Credentials::OAuthToken(token)),
        Some(oauth),
        available_universes,
        true,
    ))
}

async fn run<B: ratatui::backend::Backend + io::Write>(
    terminal: &mut Terminal<B>,
    app: &mut App,
) -> anyhow::Result<()> {
    loop {
        app.check_confirm_timeout();

        while let Ok(resolved) = app.username_rx.try_recv() {
            app.usernames.extend(resolved);
        }
        while let Ok((universe_id, name)) = app.universe_name_rx.try_recv() {
            app.universe_names.insert(universe_id, name);
        }

        let area = terminal.size()?;
        app.value_viewport_height = area.height.saturating_sub(6);

        terminal.draw(|frame| ui::draw(frame, app))?;

        if !event::poll(Duration::from_millis(200))? {
            continue;
        }

        let Event::Key(key) = event::read()? else {
            continue;
        };
        if key.kind != KeyEventKind::Press {
            continue;
        }

        if app.show_help {
            if matches!(key.code, KeyCode::Char('?') | KeyCode::Esc | KeyCode::Char('q')) {
                app.show_help = false;
            }
            continue;
        }
        if key.code == KeyCode::Char('?') && !app.text_input_active() {
            app.show_help = true;
            continue;
        }

        let action = match app.screen {
            Screen::Menu => handle_menu_key(app, key.code),
            Screen::UniverseChoice => handle_universe_choice_key(app, key.code),
            Screen::UniverseSelect => handle_universe_select_key(app, key.code),
            Screen::UniverseInput => handle_universe_input_key(app, key.code),
            Screen::Stores => handle_stores_key(app, key.code),
            Screen::Entries => handle_entries_key(app, key.code),
            Screen::Value => handle_value_key(app, key.code, key.modifiers),
            Screen::Messaging => handle_messaging_key(app, key.code),
        };

        if let Some(Action::EditValueExternal) = action {
            edit_value_external(terminal, app).await?;
        } else if let Some(Action::LoadUniverses) = action {
            let needs_login = match &app.oauth {
                Some(oauth) => auth::cached_access_token(oauth).await.is_none(),
                None => false,
            };
            if needs_login {
                perform_with_terminal_suspended(terminal, app, Action::LoadUniverses).await?;
            } else {
                app.loading = true;
                terminal.draw(|frame| ui::draw(frame, app))?;
                app.perform(Action::LoadUniverses).await;
                app.loading = false;
            }
        } else if let Some(Action::Login) = action {
            perform_with_terminal_suspended(terminal, app, Action::Login).await?;
        } else if let Some(action) = action {
            app.loading = true;
            terminal.draw(|frame| ui::draw(frame, app))?;
            app.perform(action).await;
            app.loading = false;
        }

        if app.should_quit {
            break;
        }
    }
    Ok(())
}

async fn edit_value_external<B: ratatui::backend::Backend + io::Write>(
    terminal: &mut Terminal<B>,
    app: &mut App,
) -> anyhow::Result<()> {
    let path = std::env::temp_dir().join(format!("roforgecloud-{}.json", std::process::id()));
    std::fs::write(&path, &app.value_text)?;

    let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vi".to_string());

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;

    let status = std::process::Command::new(&editor).arg(&path).status();

    enable_raw_mode()?;
    execute!(terminal.backend_mut(), EnterAlternateScreen)?;
    terminal.clear()?;

    let status = match status {
        Ok(status) => status,
        Err(err) => {
            let _ = std::fs::remove_file(&path);
            app.status = format!("error: failed to launch '{editor}': {err}");
            return Ok(());
        }
    };

    if !status.success() {
        let _ = std::fs::remove_file(&path);
        app.status = format!("error: '{editor}' exited with {status}");
        return Ok(());
    }

    let edited = std::fs::read_to_string(&path).unwrap_or_default();
    let _ = std::fs::remove_file(&path);

    if edited == app.value_text {
        app.status = "no changes".to_string();
        return Ok(());
    }

    app.value_edit_text = edited;
    app.loading = true;
    terminal.draw(|frame| ui::draw(frame, app))?;
    app.perform(Action::SaveValue).await;
    app.loading = false;

    Ok(())
}

async fn perform_with_terminal_suspended<B: ratatui::backend::Backend + io::Write>(
    terminal: &mut Terminal<B>,
    app: &mut App,
    action: Action,
) -> anyhow::Result<()> {
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;

    app.loading = true;
    app.perform(action).await;
    app.loading = false;

    enable_raw_mode()?;
    execute!(terminal.backend_mut(), EnterAlternateScreen)?;
    terminal.clear()?;

    Ok(())
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
        (PendingConfirm::Quit, KeyCode::Char('q')) => {
            app.should_quit = true;
            Some(None)
        }
        (PendingConfirm::TreeQuit, KeyCode::Esc) | (PendingConfirm::TreeQuit, KeyCode::Char('q')) => {
            app.exit_tree_mode();
            Some(None)
        }
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

fn handle_menu_key(app: &mut App, code: KeyCode) -> Option<Action> {
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

fn handle_universe_choice_key(app: &mut App, code: KeyCode) -> Option<Action> {
    if let Some(result) =
        list_nav_key(code, &mut app.universe_choice_selected, UNIVERSE_CHOICE_ITEMS.len())
    {
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
            app.status = "search: type to filter by id or name, enter/esc to confirm".to_string();
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

fn handle_universe_select_key(app: &mut App, code: KeyCode) -> Option<Action> {
    if app.universe_search_active {
        match code {
            KeyCode::Enter | KeyCode::Esc => {
                app.universe_search_active = false;
                app.status.clear();
            }
            KeyCode::Backspace => {
                app.universe_search.pop();
                app.universe_select_selected = 0;
            }
            KeyCode::Char(c) => {
                app.universe_search.push(c);
                app.universe_select_selected = 0;
            }
            _ => {}
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
        if !app.universe_search.is_empty() {
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

fn handle_universe_input_key(app: &mut App, code: KeyCode) -> Option<Action> {
    match code {
        KeyCode::Char(c) if c.is_ascii_digit() => {
            app.universe_input.push(c);
            None
        }
        KeyCode::Backspace => {
            if app.universe_input.is_empty() {
                app.screen = Screen::UniverseChoice;
                app.status.clear();
            } else {
                app.universe_input.pop();
            }
            None
        }
        KeyCode::Esc => {
            app.screen = Screen::UniverseChoice;
            app.status.clear();
            None
        }
        KeyCode::Enter => {
            let Ok(id) = app.universe_input.parse::<u64>() else {
                return None;
            };
            app.universe_id = id;
            enter_service(app)
        }
        _ => None,
    }
}

fn handle_messaging_key(app: &mut App, code: KeyCode) -> Option<Action> {
    let field = match app.messaging_field {
        MessagingField::Topic => &mut app.messaging_topic,
        MessagingField::Message => &mut app.messaging_message,
    };
    match code {
        KeyCode::Char(c) => {
            field.push(c);
            None
        }
        KeyCode::Backspace => {
            field.pop();
            None
        }
        KeyCode::Tab => {
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
        _ => None,
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
        hint: |_| Some("enter/l: open"),
        handler: |app| {
            let store = app.stores.get(app.stores_selected)?;
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

fn handle_stores_key(app: &mut App, code: KeyCode) -> Option<Action> {
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
        hint: |app| app.entries_next_page_token.is_some().then_some("n: next page"),
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
            app.status =
                "search: type to filter by id or username, enter/esc to confirm".to_string();
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

fn handle_entries_key(app: &mut App, code: KeyCode) -> Option<Action> {
    if app.entries_search_active {
        match code {
            KeyCode::Enter | KeyCode::Esc => {
                app.entries_search_active = false;
                app.status.clear();
            }
            KeyCode::Backspace => {
                app.entries_search.pop();
                app.entries_selected = 0;
            }
            KeyCode::Char(c) => {
                app.entries_search.push(c);
                app.entries_selected = 0;
            }
            _ => {}
        }
        return None;
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
        if !app.entries_search.is_empty() {
            app.entries_search.clear();
            app.entries_selected = 0;
            app.status.clear();
            return None;
        }
        app.screen = Screen::Stores;
        app.status.clear();
        return None;
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
            app.arm_confirm(PendingConfirm::DeleteEntry);
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

fn handle_value_key(app: &mut App, code: KeyCode, modifiers: KeyModifiers) -> Option<Action> {
    if app.tree_mode {
        return handle_tree_key(app, code, modifiers);
    }

    if let Some(result) = handle_pending_confirm(app, code) {
        return result;
    }
    if let Some(result) = quit_key(code, app) {
        return result;
    }
    if let Some(result) = back_key(code, app, Screen::Entries) {
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
            app.tree_move(-1);
            None
        },
    },
    KeyAction {
        keys: &[KeyCode::Down, KeyCode::Char('j')],
        hint: |_| None,
        handler: |app| {
            app.tree_move(1);
            None
        },
    },
    KeyAction {
        keys: &[KeyCode::Char(' ')],
        hint: |_| Some("space: fold/unfold"),
        handler: |app| {
            app.tree_toggle();
            None
        },
    },
    KeyAction {
        keys: &[KeyCode::Enter],
        hint: |_| Some("enter: edit value"),
        handler: |app| {
            app.tree_edit_leaf();
            None
        },
    },
    KeyAction {
        keys: &[KeyCode::Char('d')],
        hint: |_| Some("d: delete entry"),
        handler: |app| {
            app.arm_confirm(PendingConfirm::DeleteEntry);
            None
        },
    },
    KeyAction {
        keys: &[KeyCode::Esc, KeyCode::Char('q')],
        hint: |_| Some("esc/q: exit tree"),
        handler: |app| {
            if app.tree_dirty {
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

fn handle_tree_key(app: &mut App, code: KeyCode, modifiers: KeyModifiers) -> Option<Action> {
    if app.tree_editing {
        match code {
            KeyCode::Esc => app.tree_cancel_edit(),
            KeyCode::Enter => app.tree_confirm_edit(),
            KeyCode::Backspace => {
                app.tree_edit_text.pop();
            }
            KeyCode::Char(c) => app.tree_edit_text.push(c),
            _ => {}
        }
        return None;
    }

    if let Some(result) = handle_pending_confirm(app, code) {
        return result;
    }

    if code == KeyCode::Char('s') && modifiers.contains(KeyModifiers::CONTROL) {
        return Some(Action::SaveTree);
    }

    for action in TREE_KEYS {
        if action.keys.contains(&code) {
            return (action.handler)(app);
        }
    }
    None
}
