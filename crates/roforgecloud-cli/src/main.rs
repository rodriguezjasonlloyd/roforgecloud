mod commands;
mod output;

use clap::builder::styling::{AnsiColor, Styles};
use clap::{Parser, Subcommand};
use colored::Colorize;
use roforgecloud_core::auth;
use roforgecloud_core::oauth;
use roforgecloud_core::opencloud::{Credentials, OpenCloudClient};

use commands::auth::CliLoginPrompt;
use commands::datastore::DatastoreCommand;
use commands::memory_store::MemoryStoreCommand;
use commands::messaging::MessagingCommand;
use commands::ordered_datastore::OrderedDatastoreCommand;

const STYLES: Styles = Styles::styled()
    .header(AnsiColor::Yellow.on_default().bold())
    .usage(AnsiColor::Yellow.on_default().bold())
    .literal(AnsiColor::Green.on_default().bold())
    .placeholder(AnsiColor::Cyan.on_default());

#[derive(Parser)]
#[command(name = "roforgecloud", about = "Roblox Open Cloud companion CLI", styles = STYLES)]
struct Cli {
    #[arg(long, env = "ROFORGE_API_KEY", hide_env_values = true, global = true)]
    api_key: Option<String>,

    #[arg(long, env = "ROFORGE_OAUTH_CLIENT_ID", hide_env_values = true, hide_default_value = true, default_value = oauth::DEFAULT_CLIENT_ID, global = true)]
    client_id: String,

    /// For self-registered OAuth apps; bypasses the relay.
    #[arg(
        long,
        env = "ROFORGE_OAUTH_CLIENT_SECRET",
        hide_env_values = true,
        hide_default_value = true,
        global = true
    )]
    client_secret: Option<String>,

    /// Relay holding the client secret. Ignored if --client-secret is set.
    #[arg(long, env = "ROFORGE_OAUTH_RELAY_URL",hide_env_values = true, hide_default_value = true, default_value = oauth::DEFAULT_RELAY_URL, global = true)]
    relay_url: String,

    #[arg(
        long,
        env = "ROFORGE_OAUTH_REDIRECT_URI",
        hide_env_values = true,
        hide_default_value = true,
        default_value = "http://localhost:8675/callback",
        global = true
    )]
    redirect_uri: String,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    #[command(subcommand)]
    Datastore(DatastoreCommand),
    #[command(subcommand)]
    OrderedDatastore(OrderedDatastoreCommand),
    #[command(subcommand)]
    MemoryStore(MemoryStoreCommand),
    #[command(subcommand)]
    Messaging(MessagingCommand),
    Login,
    Logout,
}

#[tokio::main]
async fn main() {
    if let Err(err) = run().await {
        output::print_error(&err);
        std::process::exit(1);
    }
}

async fn run() -> anyhow::Result<()> {
    let _ = dotenvy::dotenv();
    let cli = Cli::parse();

    match cli.command {
        Command::Login => {
            let oauth = auth::build_oauth_client(
                cli.client_id,
                cli.client_secret,
                &cli.relay_url,
                &cli.redirect_uri,
            )?;
            auth::force_login(&oauth, &cli.redirect_uri, &CliLoginPrompt::default()).await?;
            return Ok(());
        }
        Command::Logout => {
            let oauth = auth::build_oauth_client(
                cli.client_id,
                cli.client_secret,
                &cli.relay_url,
                &cli.redirect_uri,
            )?;
            auth::logout(&oauth).await?;
            println!("{}", "logged out".green());
            return Ok(());
        }
        _ => {}
    }

    let api_key = cli
        .api_key
        .ok_or_else(|| anyhow::anyhow!("missing API key: pass --api-key or set ROFORGE_API_KEY"))?;
    let client = OpenCloudClient::new(Credentials::ApiKey(api_key));

    match cli.command {
        Command::Datastore(cmd) => commands::datastore::run(&client, cmd).await,
        Command::OrderedDatastore(cmd) => commands::ordered_datastore::run(&client, cmd).await,
        Command::MemoryStore(cmd) => commands::memory_store::run(&client, cmd).await,
        Command::Messaging(cmd) => commands::messaging::run(&client, cmd).await,
        Command::Login | Command::Logout => unreachable!(),
    }
}
