mod commands;
mod output;

use clap::builder::styling::{AnsiColor, Styles};
use clap::{Parser, Subcommand};
use colored::Colorize;
use roforgecloud_core::auth;
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
    #[command(flatten)]
    oauth: auth::OAuthArgs,

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
            let oauth = cli.oauth.build_oauth_client()?;
            auth::force_login(&oauth, &cli.oauth.redirect_uri, &CliLoginPrompt::default()).await?;
            return Ok(());
        }
        Command::Logout => {
            let oauth = cli.oauth.build_oauth_client()?;
            auth::logout(&oauth).await?;
            println!("{}", "logged out".green());
            return Ok(());
        }
        _ => {}
    }

    let api_key = cli
        .oauth
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
