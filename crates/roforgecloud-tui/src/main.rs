mod api;
mod app;
mod json_highlight;
mod json_tree;
mod screens;
mod status;
mod tree_editor;
mod ui;
mod update;
mod user_lookup;

use std::io;
use std::time::Duration;

use clap::Parser;
use crossterm::event::{self, Event, KeyEventKind};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

use app::{Action, App, TextFieldExt};
use roforgecloud_core::auth;
use roforgecloud_core::oauth::{self, OAuthClient};
use roforgecloud_core::opencloud::{Credentials, OpenCloudClient};

#[derive(Parser)]
#[command(name = "roforgecloud-tui", about = "Roblox developer toolkit")]
struct Cli {
    #[command(flatten)]
    oauth: auth::OAuthArgs,
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

    let has_api_key = cli.oauth.api_key.is_some();
    let mut app = App::new(
        client,
        has_api_key,
        oauth,
        cli.oauth.redirect_uri.clone(),
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
    let oauth = cli.oauth.build_oauth_client()?;

    if let Some(api_key) = &cli.oauth.api_key {
        return Ok((
            OpenCloudClient::new(Credentials::ApiKey(api_key.clone())),
            Some(oauth),
            Vec::new(),
            auth::is_logged_in(),
        ));
    }

    let token = auth::access_token(&oauth, &cli.oauth.redirect_uri, &auth::NoopLoginPrompt).await?;

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
) -> anyhow::Result<()>
where
    B::Error: Send + Sync + 'static,
{
    loop {
        app.check_confirm_timeout();

        while let Ok(resolved) = app.entries.username_rx.try_recv() {
            app.entries.usernames.extend(resolved);
        }
        while let Ok((universe_id, name)) = app.universe_name_rx.try_recv() {
            app.universe_names.insert(universe_id, name);
        }

        let area = terminal.size()?;
        app.value.viewport_height = area.height.saturating_sub(6);

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

        let action = (screens::def(app.screen).handle_key)(app, key.code, key.modifiers);

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
) -> anyhow::Result<Option<String>>
where
    B::Error: Send + Sync + 'static,
{
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
            app.status = status::editor_launch_error(&editor, err);
            return Ok(None);
        }
    };

    if !status.success() {
        let _ = std::fs::remove_file(&path);
        app.status = status::editor_exit_error(&editor, status);
        return Ok(None);
    }

    let edited = std::fs::read_to_string(&path).unwrap_or_default();
    let _ = std::fs::remove_file(&path);
    Ok(Some(edited))
}

async fn edit_value_external<B: ratatui::backend::Backend + io::Write>(
    terminal: &mut Terminal<B>,
    app: &mut App,
) -> anyhow::Result<()>
where
    B::Error: Send + Sync + 'static,
{
    let initial = app.value.text.clone();
    let Some(edited) = run_editor(terminal, app, &initial, ".json").await? else {
        return Ok(());
    };

    if edited == initial {
        app.status = status::no_changes();
        return Ok(());
    }

    app.value.edit_text = edited;
    app.loading = true;
    terminal.draw(|frame| ui::draw(frame, app))?;
    app.perform(Action::SaveValue).await;
    app.loading = false;

    Ok(())
}

async fn edit_tree_value_external<B: ratatui::backend::Backend + io::Write>(
    terminal: &mut Terminal<B>,
    app: &mut App,
) -> anyhow::Result<()>
where
    B::Error: Send + Sync + 'static,
{
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
        app.status = status::no_changes();
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
                app.status = status::expect_single_key_obj();
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
) -> anyhow::Result<()>
where
    B::Error: Send + Sync + 'static,
{
    let template = "{\n  \"id\": \"\",\n  \"value\": null\n}\n";
    let Some(edited) = run_editor(terminal, app, template, "-create.json").await? else {
        return Ok(());
    };

    let parsed: serde_json::Value = match serde_json::from_str(&edited) {
        Ok(value) => value,
        Err(err) => {
            app.status = status::json_error(err);
            return Ok(());
        }
    };
    let Some(id) = parsed
        .get("id")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
    else {
        app.status = status::id_field_required();
        return Ok(());
    };
    let value = parsed
        .get("value")
        .cloned()
        .unwrap_or(serde_json::Value::Null);

    app.entries.create_id.set_value(id);
    app.entries
        .create_value
        .set_value(serde_json::to_string(&value)?);
    app.entries.create_active = false;

    app.loading = true;
    terminal.draw(|frame| ui::draw(frame, app))?;
    app.perform(Action::CreateEntry).await;
    app.loading = false;

    Ok(())
}

async fn create_ordered_entry_external<B: ratatui::backend::Backend + io::Write>(
    terminal: &mut Terminal<B>,
    app: &mut App,
) -> anyhow::Result<()>
where
    B::Error: Send + Sync + 'static,
{
    let template = "{\n  \"id\": \"\",\n  \"value\": 0\n}\n";
    let Some(edited) = run_editor(terminal, app, template, "-create-ordered.json").await? else {
        return Ok(());
    };

    let parsed: serde_json::Value = match serde_json::from_str(&edited) {
        Ok(value) => value,
        Err(err) => {
            app.status = status::json_error(err);
            return Ok(());
        }
    };
    let Some(id) = parsed
        .get("id")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
    else {
        app.status = status::id_field_required();
        return Ok(());
    };
    let Some(value) = parsed.get("value").and_then(|v| v.as_f64()) else {
        app.status = status::value_field_number();
        return Ok(());
    };

    app.ordered_entries.create_id.set_value(id);
    app.ordered_entries
        .create_value
        .set_value(value.to_string());
    app.ordered_entries.create_active = false;

    app.loading = true;
    terminal.draw(|frame| ui::draw(frame, app))?;
    app.perform(Action::CreateOrderedEntry).await;
    app.loading = false;

    Ok(())
}

async fn create_memory_item_external<B: ratatui::backend::Backend + io::Write>(
    terminal: &mut Terminal<B>,
    app: &mut App,
) -> anyhow::Result<()>
where
    B::Error: Send + Sync + 'static,
{
    let template = "{\n  \"id\": \"\",\n  \"value\": null,\n  \"ttl\": 3600\n}\n";
    let Some(edited) = run_editor(terminal, app, template, "-create-memory.json").await? else {
        return Ok(());
    };

    let parsed: serde_json::Value = match serde_json::from_str(&edited) {
        Ok(value) => value,
        Err(err) => {
            app.status = status::json_error(err);
            return Ok(());
        }
    };
    let Some(id) = parsed
        .get("id")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
    else {
        app.status = status::id_field_required();
        return Ok(());
    };
    let value = parsed
        .get("value")
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    let ttl = parsed.get("ttl").and_then(|v| v.as_u64()).unwrap_or(3600);

    app.memory_entries.create_id.set_value(id);
    app.memory_entries
        .create_value
        .set_value(serde_json::to_string(&value)?);
    app.memory_entries.create_ttl.set_value(ttl.to_string());
    app.memory_entries.create_active = false;

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
) -> anyhow::Result<()>
where
    B::Error: Send + Sync + 'static,
{
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
