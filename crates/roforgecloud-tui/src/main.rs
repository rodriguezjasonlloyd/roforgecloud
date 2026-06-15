mod app;
mod json_highlight;
mod json_tree;
mod tree_editor;
mod ui;
mod update;
mod userlookup;

use std::io;
use std::time::Duration;

use clap::Parser;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

use app::{Action, App, Screen};
use roforgecloud_core::auth;
use roforgecloud_core::oauth::{self, OAuthClient};
use roforgecloud_core::opencloud::{Credentials, OpenCloudClient};
use update::{
    handle_entries_key, handle_memory_entries_key, handle_memory_store_input_key,
    handle_menu_key, handle_messaging_key, handle_ordered_entries_key,
    handle_ordered_store_input_key, handle_ordered_value_key, handle_stores_key,
    handle_universe_choice_key, handle_universe_input_key, handle_universe_select_key,
    handle_value_key,
};

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

    let token = auth::access_token(&oauth, &cli.redirect_uri, &auth::NoopLoginPrompt).await?;

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

        if app.which_key.active {
            if matches!(
                key.code,
                KeyCode::Char('?') | KeyCode::Esc | KeyCode::Char('q')
            ) {
                app.which_key.dismiss();
            }
            continue;
        }
        if key.code == KeyCode::Char('?') && !app.text_input_active() {
            app.which_key.toggle();
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
    let Some(editor) = &app.tree_editor else {
        return Ok(());
    };
    let rows = json_tree::flatten(editor.root());
    let Some(current) = rows.get(editor.cursor()) else {
        return Ok(());
    };
    let key = current.key.clone();
    let Some(value) = editor.current_value() else {
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
                app.tree_editor
                    .as_mut()
                    .unwrap()
                    .replace_value(Some(key), value);
            }
            _ => {
                app.status = "error: expected a single-key JSON object".to_string();
            }
        }
    } else {
        let value = serde_json::from_str::<serde_json::Value>(edited)
            .unwrap_or_else(|_| serde_json::Value::String(edited.to_string()));
        app.tree_editor.as_mut().unwrap().replace_value(None, value);
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

