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
    Action, App, EntriesCreateField, MemoryCreateField, MessagingField, OrderedCreateField,
    OrderedInputField, PendingConfirm, Screen, TextField, TreeTarget, ValueSource, SERVICE_ACCOUNT,
    SERVICE_DATA_STORES, SERVICE_MEMORY_STORES, SERVICE_MESSAGING, SERVICE_ORDERED_DATA_STORES,
    UNIVERSE_CHOICE_ENTER_ID, UNIVERSE_CHOICE_ITEMS, UNIVERSE_CHOICE_LIST_ALL,
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
            if matches!(
                key.code,
                KeyCode::Char('?') | KeyCode::Esc | KeyCode::Char('q')
            ) {
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
            Screen::Entries => handle_entries_key(app, key.code, key.modifiers),
            Screen::Value => handle_value_key(app, key.code, key.modifiers),
            Screen::Messaging => handle_messaging_key(app, key.code),
            Screen::OrderedStoreInput => handle_ordered_store_input_key(app, key.code),
            Screen::OrderedEntries => handle_ordered_entries_key(app, key.code),
            Screen::OrderedValue => handle_ordered_value_key(app, key.code),
            Screen::MemoryStoreInput => handle_memory_store_input_key(app, key.code),
            Screen::MemoryStoreEntries => handle_memory_entries_key(app, key.code, key.modifiers),
        };

        if let Some(Action::EditValueExternal) = action {
            edit_value_external(terminal, app).await?;
        } else if let Some(Action::EditTreeValueExternal) = action {
            edit_tree_value_external(terminal, app).await?;
        } else if let Some(Action::CreateEntryExternal) = action {
            create_entry_external(terminal, app).await?;
        } else if let Some(Action::CreateOrderedEntryExternal) = action {
            create_ordered_entry_external(terminal, app).await?;
        } else if let Some(Action::CreateMemoryItemExternal) = action {
            create_memory_item_external(terminal, app).await?;
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

async fn run_editor<B: ratatui::backend::Backend + io::Write>(
    terminal: &mut Terminal<B>,
    app: &mut App,
    initial: &str,
    suffix: &str,
) -> anyhow::Result<Option<String>> {
    let path = std::env::temp_dir().join(format!("roforgecloud-{}{suffix}", std::process::id()));
    std::fs::write(&path, initial)?;

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
            return Ok(None);
        }
    };

    if !status.success() {
        let _ = std::fs::remove_file(&path);
        app.status = format!("error: '{editor}' exited with {status}");
        return Ok(None);
    }

    let edited = std::fs::read_to_string(&path).unwrap_or_default();
    let _ = std::fs::remove_file(&path);
    Ok(Some(edited))
}

async fn edit_value_external<B: ratatui::backend::Backend + io::Write>(
    terminal: &mut Terminal<B>,
    app: &mut App,
) -> anyhow::Result<()> {
    let initial = app.value_text.clone();
    let Some(edited) = run_editor(terminal, app, &initial, ".json").await? else {
        return Ok(());
    };

    if edited == initial {
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

async fn edit_tree_value_external<B: ratatui::backend::Backend + io::Write>(
    terminal: &mut Terminal<B>,
    app: &mut App,
) -> anyhow::Result<()> {
    let Some(tree) = &app.value_tree else {
        return Ok(());
    };
    let rows = json_tree::flatten(tree);
    let Some(current) = rows.get(app.tree_cursor) else {
        return Ok(());
    };
    let key = current.key.clone();
    let Some(value) = app.tree_current_value() else {
        return Ok(());
    };

    let initial = match &key {
        Some(key) => {
            let mut obj = serde_json::Map::new();
            obj.insert(key.clone(), value);
            serde_json::to_string_pretty(&serde_json::Value::Object(obj))?
        }
        None => serde_json::to_string_pretty(&value)?,
    };

    let Some(edited) = run_editor(terminal, app, &initial, ".json").await? else {
        return Ok(());
    };
    let edited = edited.trim();

    if edited == initial.trim() {
        app.status = "no changes".to_string();
        return Ok(());
    }

    if key.is_some() {
        match serde_json::from_str::<serde_json::Value>(edited) {
            Ok(serde_json::Value::Object(map)) if map.len() == 1 => {
                let (key, value) = map.into_iter().next().unwrap();
                app.tree_replace_value(Some(key), value);
            }
            _ => {
                app.status = "error: expected a single-key JSON object".to_string();
            }
        }
    } else {
        let value = serde_json::from_str::<serde_json::Value>(edited)
            .unwrap_or_else(|_| serde_json::Value::String(edited.to_string()));
        app.tree_replace_value(None, value);
    }
    Ok(())
}

async fn create_entry_external<B: ratatui::backend::Backend + io::Write>(
    terminal: &mut Terminal<B>,
    app: &mut App,
) -> anyhow::Result<()> {
    let template = "{\n  \"id\": \"\",\n  \"value\": null\n}\n";
    let Some(edited) = run_editor(terminal, app, template, "-create.json").await? else {
        return Ok(());
    };

    let parsed: serde_json::Value = match serde_json::from_str(&edited) {
        Ok(value) => value,
        Err(err) => {
            app.status = format!("error: invalid JSON: {err}");
            return Ok(());
        }
    };
    let Some(id) = parsed
        .get("id")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
    else {
        app.status = "error: \"id\" field must be a non-empty string".to_string();
        return Ok(());
    };
    let value = parsed
        .get("value")
        .cloned()
        .unwrap_or(serde_json::Value::Null);

    app.entries_create_id.set(id);
    app.entries_create_value.set(serde_json::to_string(&value)?);
    app.entries_create_active = false;

    app.loading = true;
    terminal.draw(|frame| ui::draw(frame, app))?;
    app.perform(Action::CreateEntry).await;
    app.loading = false;

    Ok(())
}

async fn create_ordered_entry_external<B: ratatui::backend::Backend + io::Write>(
    terminal: &mut Terminal<B>,
    app: &mut App,
) -> anyhow::Result<()> {
    let template = "{\n  \"id\": \"\",\n  \"value\": 0\n}\n";
    let Some(edited) = run_editor(terminal, app, template, "-create-ordered.json").await? else {
        return Ok(());
    };

    let parsed: serde_json::Value = match serde_json::from_str(&edited) {
        Ok(value) => value,
        Err(err) => {
            app.status = format!("error: invalid JSON: {err}");
            return Ok(());
        }
    };
    let Some(id) = parsed
        .get("id")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
    else {
        app.status = "error: \"id\" field must be a non-empty string".to_string();
        return Ok(());
    };
    let Some(value) = parsed.get("value").and_then(|v| v.as_f64()) else {
        app.status = "error: \"value\" field must be a number".to_string();
        return Ok(());
    };

    app.ordered_create_id.set(id);
    app.ordered_create_value.set(value.to_string());
    app.ordered_create_active = false;

    app.loading = true;
    terminal.draw(|frame| ui::draw(frame, app))?;
    app.perform(Action::CreateOrderedEntry).await;
    app.loading = false;

    Ok(())
}

async fn create_memory_item_external<B: ratatui::backend::Backend + io::Write>(
    terminal: &mut Terminal<B>,
    app: &mut App,
) -> anyhow::Result<()> {
    let template = "{\n  \"id\": \"\",\n  \"value\": null,\n  \"ttl\": 3600\n}\n";
    let Some(edited) = run_editor(terminal, app, template, "-create-memory.json").await? else {
        return Ok(());
    };

    let parsed: serde_json::Value = match serde_json::from_str(&edited) {
        Ok(value) => value,
        Err(err) => {
            app.status = format!("error: invalid JSON: {err}");
            return Ok(());
        }
    };
    let Some(id) = parsed
        .get("id")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
    else {
        app.status = "error: \"id\" field must be a non-empty string".to_string();
        return Ok(());
    };
    let value = parsed
        .get("value")
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    let ttl = parsed.get("ttl").and_then(|v| v.as_u64()).unwrap_or(3600);

    app.memory_create_id.set(id);
    app.memory_create_value.set(serde_json::to_string(&value)?);
    app.memory_create_ttl.set(ttl.to_string());
    app.memory_create_active = false;

    app.loading = true;
    terminal.draw(|frame| ui::draw(frame, app))?;
    app.perform(Action::CreateMemoryItem).await;
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

fn handle_universe_select_key(app: &mut App, code: KeyCode) -> Option<Action> {
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

fn handle_universe_input_key(app: &mut App, code: KeyCode) -> Option<Action> {
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

fn handle_messaging_key(app: &mut App, code: KeyCode) -> Option<Action> {
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

fn handle_stores_key(app: &mut App, code: KeyCode) -> Option<Action> {
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

fn handle_entries_key(app: &mut App, code: KeyCode, modifiers: KeyModifiers) -> Option<Action> {
    if app.tree_mode {
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

fn handle_value_key(app: &mut App, code: KeyCode, modifiers: KeyModifiers) -> Option<Action> {
    if app.tree_mode {
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
        hint: |_| None,
        handler: |app| {
            app.tree_edit_leaf();
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
            app.tree_add_entry();
            None
        },
    },
    KeyAction {
        keys: &[KeyCode::Char('x')],
        hint: |_| Some("x: delete node"),
        handler: |app| {
            app.tree_delete_current();
            None
        },
    },
    KeyAction {
        keys: &[KeyCode::Char('e')],
        hint: |_| Some("e: edit"),
        handler: |app| {
            app.tree_pending_leader = true;
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
            if app.tree_dirty {
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

fn handle_ordered_store_input_key(app: &mut App, code: KeyCode) -> Option<Action> {
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

fn handle_ordered_entries_key(app: &mut App, code: KeyCode) -> Option<Action> {
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

fn handle_ordered_value_key(app: &mut App, code: KeyCode) -> Option<Action> {
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
    if app.tree_editing_key {
        match code {
            KeyCode::Esc => app.tree_cancel_edit(),
            KeyCode::Enter => app.tree_confirm_key(),
            KeyCode::Tab | KeyCode::BackTab => {
                app.tree_confirm_key();
                app.tree_edit_leaf();
            }
            _ => {
                handle_text_field_key(&mut app.tree_edit_key, code, |_| true);
            }
        }
        return None;
    }

    if app.tree_editing {
        match code {
            KeyCode::Esc => app.tree_cancel_edit(),
            KeyCode::Enter => app.tree_confirm_edit(),
            KeyCode::Tab | KeyCode::BackTab => {
                app.tree_confirm_edit();
                app.tree_edit_key_start();
            }
            _ => {
                handle_text_field_key(&mut app.tree_edit_text, code, |_| true);
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

    if app.tree_pending_leader {
        app.tree_pending_leader = false;
        return match code {
            KeyCode::Char('v') => {
                app.tree_edit_leaf();
                None
            }
            KeyCode::Char('k') => {
                app.tree_edit_key_start();
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

fn handle_memory_store_input_key(app: &mut App, code: KeyCode) -> Option<Action> {
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

fn handle_memory_entries_key(
    app: &mut App,
    code: KeyCode,
    modifiers: KeyModifiers,
) -> Option<Action> {
    if app.tree_mode {
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
