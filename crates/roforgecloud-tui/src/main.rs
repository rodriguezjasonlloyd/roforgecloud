mod app;
pub(crate) mod auth;
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
    Action, App, MessagingField, Screen, SERVICE_DATA_STORES, SERVICE_MESSAGING,
    UNIVERSE_CHOICE_ENTER_ID, UNIVERSE_CHOICE_ITEMS, UNIVERSE_CHOICE_LIST_ALL,
};
use roforgecloud_core::oauth::{self, OAuthClient};
use roforgecloud_core::opencloud::{Credentials, OpenCloudClient};

const DEFAULT_CLIENT_ID: &str = "1394680015364443315";

#[derive(Parser)]
#[command(
    name = "roforgecloud-tui",
    about = "Browse Roblox Open Cloud data stores"
)]
struct Cli {
    #[arg(long, env = "ROFORGE_API_KEY")]
    api_key: Option<String>,

    #[arg(long, env = "ROFORGE_OAUTH_CLIENT_ID", default_value = DEFAULT_CLIENT_ID)]
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

    /// Revoke the cached OAuth session and exit (forces a fresh login next run).
    #[arg(long)]
    logout: bool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _ = dotenvy::dotenv();
    let cli = Cli::parse();

    if cli.logout {
        let oauth = OAuthClient::new(
            cli.client_id.clone(),
            cli.client_secret.clone().unwrap_or_default(),
            &cli.redirect_uri,
        )?;
        let oauth = if cli.client_secret.is_none() && !cli.relay_url.is_empty() {
            oauth.with_relay(&cli.relay_url)?
        } else {
            oauth
        };
        auth::logout(&oauth).await?;
        println!("logged out");
        return Ok(());
    }

    let (client, oauth, available_universes) = build_client(&cli).await?;

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
    );

    let result = run(&mut terminal, &mut app).await;

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

async fn build_client(
    cli: &Cli,
) -> anyhow::Result<(OpenCloudClient, Option<OAuthClient>, Vec<u64>)> {
    let oauth = OAuthClient::new(
        cli.client_id.clone(),
        cli.client_secret.clone().unwrap_or_default(),
        &cli.redirect_uri,
    )?;
    let oauth = if cli.client_secret.is_none() && !cli.relay_url.is_empty() {
        oauth.with_relay(&cli.relay_url)?
    } else {
        oauth
    };

    if let Some(api_key) = &cli.api_key {
        return Ok((
            OpenCloudClient::new(Credentials::ApiKey(api_key.clone())),
            Some(oauth),
            Vec::new(),
        ));
    }

    let token = auth::access_token(&oauth, &cli.redirect_uri).await?;

    let resources = oauth.token_resources(&token).await?;
    let available_universes = oauth::authorized_universe_ids(&resources);

    Ok((
        OpenCloudClient::new(Credentials::OAuthToken(token)),
        Some(oauth),
        available_universes,
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
            perform_with_terminal_suspended(terminal, app, Action::LoadUniverses).await?;
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

fn handle_menu_key(app: &mut App, code: KeyCode) -> Option<Action> {
    match code {
        KeyCode::Up | KeyCode::Char('k') => {
            if app.menu_selected > 0 {
                app.menu_selected -= 1;
            }
            None
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if app.menu_selected + 1 < app.menu_items.len() {
                app.menu_selected += 1;
            }
            None
        }
        KeyCode::Enter | KeyCode::Char('l') => {
            app.pending_service = app.menu_items[app.menu_selected].1;
            app.status.clear();
            app.universe_choice_selected = 0;
            app.screen = Screen::UniverseChoice;
            None
        }
        KeyCode::Char('q') => {
            app.should_quit = true;
            None
        }
        _ => None,
    }
}

fn handle_universe_choice_key(app: &mut App, code: KeyCode) -> Option<Action> {
    match code {
        KeyCode::Up | KeyCode::Char('k') => {
            if app.universe_choice_selected > 0 {
                app.universe_choice_selected -= 1;
            }
            None
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if app.universe_choice_selected + 1 < UNIVERSE_CHOICE_ITEMS.len() {
                app.universe_choice_selected += 1;
            }
            None
        }
        KeyCode::Enter | KeyCode::Char('l') => match app.universe_choice_selected {
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
        KeyCode::Esc | KeyCode::Backspace | KeyCode::Char('h') => {
            app.screen = Screen::Menu;
            app.status.clear();
            None
        }
        KeyCode::Char('q') => {
            app.should_quit = true;
            None
        }
        _ => None,
    }
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

    let visible = app.visible_universe_indices();

    match code {
        KeyCode::Up | KeyCode::Char('k') => {
            if app.universe_select_selected > 0 {
                app.universe_select_selected -= 1;
            }
            None
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if app.universe_select_selected + 1 < visible.len() {
                app.universe_select_selected += 1;
            }
            None
        }
        KeyCode::Char('/') => {
            app.universe_search_active = true;
            app.status = "search: type to filter by id or name, enter/esc to confirm".to_string();
            None
        }
        KeyCode::Enter | KeyCode::Char('l') => {
            let &index = visible.get(app.universe_select_selected)?;
            let universe_id = app.available_universes[index];
            app.universe_id = universe_id;
            enter_service(app)
        }
        KeyCode::Esc | KeyCode::Backspace | KeyCode::Char('h') => {
            if !app.universe_search.is_empty() {
                app.universe_search.clear();
                app.universe_select_selected = 0;
                app.status.clear();
                return None;
            }
            app.screen = Screen::UniverseChoice;
            app.status.clear();
            None
        }
        KeyCode::Char('q') => {
            app.should_quit = true;
            None
        }
        _ => None,
    }
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

fn handle_stores_key(app: &mut App, code: KeyCode) -> Option<Action> {
    if app.stores_delete_pending {
        app.stores_delete_pending = false;
        app.confirm_deadline = None;
        if code == KeyCode::Char('d') {
            if !app.stores_marked.is_empty() {
                return Some(Action::BulkDeleteDataStores);
            }
            return Some(Action::DeleteDataStore);
        }
        app.status.clear();
        return None;
    }

    if app.stores_undelete_pending {
        app.stores_undelete_pending = false;
        app.confirm_deadline = None;
        if code == KeyCode::Char('u') {
            if !app.stores_marked.is_empty() {
                return Some(Action::BulkUndeleteDataStores);
            }
            return Some(Action::UndeleteDataStore);
        }
        app.status.clear();
        return None;
    }

    match code {
        KeyCode::Up | KeyCode::Char('k') => {
            if app.stores_selected > 0 {
                app.stores_selected -= 1;
            }
            None
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if app.stores_selected + 1 < app.stores.len() {
                app.stores_selected += 1;
            }
            None
        }
        KeyCode::Char('r') => Some(Action::LoadStores),
        KeyCode::Char('D') => {
            app.stores_show_deleted = !app.stores_show_deleted;
            Some(Action::LoadStores)
        }
        KeyCode::Char(' ') => {
            app.toggle_store_mark();
            None
        }
        KeyCode::Char('a') => {
            app.toggle_select_all_stores();
            None
        }
        KeyCode::Char('d') => {
            if !app.stores_marked.is_empty() {
                let count = app.stores_marked.len();
                app.stores_delete_pending = true;
                app.arm_confirm();
                app.status = format!(
                    "press d again to schedule {count} selected data stores for deletion, any other key to cancel"
                );
                return None;
            }
            if app.stores.is_empty() {
                return None;
            }
            app.stores_delete_pending = true;
            app.arm_confirm();
            app.status =
                "press d again to schedule this data store for deletion, any other key to cancel"
                    .to_string();
            None
        }
        KeyCode::Char('u') => {
            if !app.stores_marked.is_empty() {
                let count = app.stores_marked.len();
                app.stores_undelete_pending = true;
                app.arm_confirm();
                app.status = format!(
                    "press u again to restore {count} selected data stores, any other key to cancel"
                );
                return None;
            }
            let store = app.stores.get(app.stores_selected)?;
            if store.state.as_deref() != Some("ACTIVE") {
                return Some(Action::UndeleteDataStore);
            }
            None
        }
        KeyCode::Enter | KeyCode::Char('l') => {
            let store = app.stores.get(app.stores_selected)?;
            app.data_store_id = store.id.clone();
            app.entries_next_page_token = None;
            app.screen = Screen::Entries;
            Some(Action::LoadEntries)
        }
        KeyCode::Esc | KeyCode::Backspace | KeyCode::Char('h') => {
            app.screen = Screen::UniverseChoice;
            app.status.clear();
            None
        }
        KeyCode::Char('q') => {
            app.should_quit = true;
            None
        }
        _ => None,
    }
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

    if app.entries_delete_pending {
        app.entries_delete_pending = false;
        app.confirm_deadline = None;
        if code == KeyCode::Char('d') {
            return Some(Action::DeleteEntry);
        }
        app.status.clear();
        return None;
    }

    if app.entries_bulk_delete_pending {
        app.entries_bulk_delete_pending = false;
        app.confirm_deadline = None;
        if code == KeyCode::Char('d') {
            return Some(Action::BulkDeleteEntries);
        }
        app.status.clear();
        return None;
    }

    let visible = app.visible_entry_indices().len();

    match code {
        KeyCode::Up | KeyCode::Char('k') => {
            if app.entries_selected > 0 {
                app.entries_selected -= 1;
            }
            None
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if app.entries_selected + 1 < visible {
                app.entries_selected += 1;
            }
            None
        }
        KeyCode::Char('n') => Some(Action::LoadNextEntriesPage),
        KeyCode::Char('p') => Some(Action::LoadPrevEntriesPage),
        KeyCode::Char('r') => Some(Action::RefreshEntries),
        KeyCode::Char('/') => {
            app.entries_search_active = true;
            app.status =
                "search: type to filter by id or username, enter/esc to confirm".to_string();
            None
        }
        KeyCode::Char(' ') => {
            app.toggle_entry_mark();
            None
        }
        KeyCode::Char('a') => {
            app.toggle_select_all_visible();
            None
        }
        KeyCode::Char('d') => {
            if !app.entries_marked.is_empty() {
                let count = app.entries_marked.len();
                app.entries_bulk_delete_pending = true;
                app.arm_confirm();
                app.status = format!(
                    "press d again to delete {count} selected entries, any other key to cancel"
                );
                return None;
            }
            if visible == 0 {
                return None;
            }
            app.entries_delete_pending = true;
            app.arm_confirm();
            app.status = "press d again to delete this entry, any other key to cancel".to_string();
            None
        }
        KeyCode::Enter | KeyCode::Char('l') => {
            if visible == 0 {
                return None;
            }
            app.screen = Screen::Value;
            Some(Action::LoadValue)
        }
        KeyCode::Esc | KeyCode::Backspace | KeyCode::Char('h') => {
            if !app.entries_search.is_empty() {
                app.entries_search.clear();
                app.entries_selected = 0;
                app.status.clear();
                return None;
            }
            app.screen = Screen::Stores;
            app.status.clear();
            None
        }
        KeyCode::Char('q') => {
            app.should_quit = true;
            None
        }
        _ => None,
    }
}

fn handle_value_key(app: &mut App, code: KeyCode, modifiers: KeyModifiers) -> Option<Action> {
    if app.tree_mode {
        return handle_tree_key(app, code, modifiers);
    }

    if app.entries_delete_pending {
        app.entries_delete_pending = false;
        app.confirm_deadline = None;
        if code == KeyCode::Char('d') {
            return Some(Action::DeleteEntry);
        }
        app.status.clear();
        return None;
    }

    let max_scroll = app.max_value_scroll();
    match code {
        KeyCode::Esc | KeyCode::Backspace | KeyCode::Char('h') => {
            app.screen = Screen::Entries;
            app.status.clear();
        }
        KeyCode::Char('r') => return Some(Action::LoadValue),
        KeyCode::Char('e') => return Some(Action::EditValueExternal),
        KeyCode::Char('d') => {
            app.entries_delete_pending = true;
            app.arm_confirm();
            app.status = "press d again to delete this entry, any other key to cancel".to_string();
        }
        KeyCode::Enter | KeyCode::Char('l') => app.enter_tree_mode(),
        KeyCode::Up | KeyCode::Char('k') => {
            app.value_scroll = app.value_scroll.saturating_sub(1);
        }
        KeyCode::Down | KeyCode::Char('j') => {
            app.value_scroll = (app.value_scroll + 1).min(max_scroll);
        }
        KeyCode::PageUp => {
            app.value_scroll = app.value_scroll.saturating_sub(10);
        }
        KeyCode::PageDown => {
            app.value_scroll = (app.value_scroll + 10).min(max_scroll);
        }
        KeyCode::Char('q') => {
            app.should_quit = true;
        }
        _ => {}
    }
    None
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

    if app.tree_quit_pending {
        match code {
            KeyCode::Char('q') => {
                app.should_quit = true;
                return None;
            }
            KeyCode::Esc => {
                app.exit_tree_mode();
                return None;
            }
            _ => {
                app.tree_quit_pending = false;
                app.confirm_deadline = None;
                app.status.clear();
                return None;
            }
        }
    }

    if app.entries_delete_pending {
        app.entries_delete_pending = false;
        app.confirm_deadline = None;
        if code == KeyCode::Char('d') {
            return Some(Action::DeleteEntry);
        }
        app.status.clear();
        return None;
    }

    match code {
        KeyCode::Char('s') if modifiers.contains(KeyModifiers::CONTROL) => {
            return Some(Action::SaveTree);
        }
        KeyCode::Char('d') => {
            app.entries_delete_pending = true;
            app.arm_confirm();
            app.status = "press d again to delete this entry, any other key to cancel".to_string();
        }
        KeyCode::Esc => {
            if app.tree_dirty {
                app.tree_quit_pending = true;
                app.arm_confirm();
                app.status =
                    "unsaved changes — esc again discards, q quits, ctrl+s saves".to_string();
            } else {
                app.exit_tree_mode();
            }
        }
        KeyCode::Up | KeyCode::Char('k') => app.tree_move(-1),
        KeyCode::Down | KeyCode::Char('j') => app.tree_move(1),
        KeyCode::Char(' ') => app.tree_toggle(),
        KeyCode::Enter => app.tree_edit_leaf(),
        KeyCode::Char('q') => {
            if app.tree_dirty {
                app.tree_quit_pending = true;
                app.arm_confirm();
                app.status =
                    "unsaved changes — q again quits without saving, esc discards, ctrl+s saves"
                        .to_string();
            } else {
                app.should_quit = true;
            }
        }
        _ => {}
    }
    None
}
