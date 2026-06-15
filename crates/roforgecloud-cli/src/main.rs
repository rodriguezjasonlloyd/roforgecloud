use clap::{Parser, Subcommand};
use roforgecloud_core::auth;
use roforgecloud_core::oauth;
use roforgecloud_core::opencloud::{Credentials, OpenCloudClient};

#[derive(Parser)]
#[command(name = "roforgecloud", about = "Roblox Open Cloud companion CLI")]
struct Cli {
    #[arg(long, env = "ROFORGE_API_KEY", global = true)]
    api_key: Option<String>,

    #[arg(long, env = "ROFORGE_OAUTH_CLIENT_ID", default_value = oauth::DEFAULT_CLIENT_ID, global = true)]
    client_id: String,

    /// Only needed to talk to Roblox's OAuth endpoints directly, bypassing
    /// the relay (e.g. if you registered your own OAuth app).
    #[arg(long, env = "ROFORGE_OAUTH_CLIENT_SECRET", global = true)]
    client_secret: Option<String>,

    /// OAuth relay that holds the client secret (see `worker/`). Ignored if
    /// --client-secret is set.
    #[arg(long, env = "ROFORGE_OAUTH_RELAY_URL", default_value = oauth::DEFAULT_RELAY_URL, global = true)]
    relay_url: String,

    #[arg(
        long,
        env = "ROFORGE_OAUTH_REDIRECT_URI",
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
    Messaging(MessagingCommand),
    Login,
    Logout,
}

#[derive(Subcommand)]
enum DatastoreCommand {
    ListStores {
        universe_id: u64,
    },
    Get {
        universe_id: u64,
        data_store_id: String,
        entry_id: String,
        #[arg(long)]
        scope: Option<String>,
    },
    Create {
        universe_id: u64,
        data_store_id: String,
        entry_id: String,
        value: String,
        #[arg(long)]
        scope: Option<String>,
    },
    Set {
        universe_id: u64,
        data_store_id: String,
        entry_id: String,
        value: String,
        #[arg(long)]
        scope: Option<String>,
    },
    Delete {
        universe_id: u64,
        data_store_id: String,
        entry_id: String,
        #[arg(long)]
        scope: Option<String>,
    },
    List {
        universe_id: u64,
        data_store_id: String,
        #[arg(long)]
        scope: Option<String>,
        #[arg(long)]
        filter: Option<String>,
    },
    ListScopes {
        universe_id: u64,
        data_store_id: String,
    },
}

#[derive(Subcommand)]
enum OrderedDatastoreCommand {
    List {
        universe_id: u64,
        ordered_data_store_id: String,
        #[arg(long, default_value = "global")]
        scope: String,
        #[arg(long)]
        order_by: Option<String>,
        #[arg(long)]
        filter: Option<String>,
        #[arg(long)]
        max_page_size: Option<u32>,
    },
    Create {
        universe_id: u64,
        ordered_data_store_id: String,
        entry_id: String,
        value: f64,
        #[arg(long, default_value = "global")]
        scope: String,
    },
    Get {
        universe_id: u64,
        ordered_data_store_id: String,
        entry_id: String,
        #[arg(long, default_value = "global")]
        scope: String,
    },
    Update {
        universe_id: u64,
        ordered_data_store_id: String,
        entry_id: String,
        value: f64,
        #[arg(long, default_value = "global")]
        scope: String,
    },
    Delete {
        universe_id: u64,
        ordered_data_store_id: String,
        entry_id: String,
        #[arg(long, default_value = "global")]
        scope: String,
    },
    Increment {
        universe_id: u64,
        ordered_data_store_id: String,
        entry_id: String,
        amount: f64,
        #[arg(long, default_value = "global")]
        scope: String,
    },
}

#[derive(Subcommand)]
enum MessagingCommand {
    Publish {
        universe_id: u64,
        topic: String,
        message: String,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _ = dotenvy::dotenv();
    let cli = Cli::parse();

    if matches!(cli.command, Command::Login | Command::Logout) {
        let oauth = auth::build_oauth_client(
            cli.client_id,
            cli.client_secret,
            &cli.relay_url,
            &cli.redirect_uri,
        )?;

        match cli.command {
            Command::Login => {
                auth::force_login(&oauth, &cli.redirect_uri).await?;
                println!("logged in");
            }
            Command::Logout => {
                auth::logout(&oauth).await?;
                println!("logged out");
            }
            _ => unreachable!(),
        }

        return Ok(());
    }

    let api_key = cli
        .api_key
        .ok_or_else(|| anyhow::anyhow!("missing API key: pass --api-key or set ROFORGE_API_KEY"))?;
    let client = OpenCloudClient::new(Credentials::ApiKey(api_key));

    match cli.command {
        Command::Datastore(cmd) => match cmd {
            DatastoreCommand::ListStores { universe_id } => {
                let result = client
                    .list_data_stores(universe_id, None, None, false)
                    .await?;
                for store in result.data_stores {
                    println!("{}", store.id);
                }
            }
            DatastoreCommand::Get {
                universe_id,
                data_store_id,
                entry_id,
                scope,
            } => {
                let value = client
                    .get_entry(universe_id, &data_store_id, &entry_id, scope.as_deref())
                    .await?;
                println!("{}", serde_json::to_string_pretty(&value)?);
            }
            DatastoreCommand::Create {
                universe_id,
                data_store_id,
                entry_id,
                value,
                scope,
            } => {
                let value: serde_json::Value = serde_json::from_str(&value)?;
                client
                    .create_entry(
                        universe_id,
                        &data_store_id,
                        &entry_id,
                        scope.as_deref(),
                        &value,
                    )
                    .await?;
                println!("ok");
            }
            DatastoreCommand::Set {
                universe_id,
                data_store_id,
                entry_id,
                value,
                scope,
            } => {
                let value: serde_json::Value = serde_json::from_str(&value)?;
                client
                    .set_entry(
                        universe_id,
                        &data_store_id,
                        &entry_id,
                        scope.as_deref(),
                        &value,
                        None,
                    )
                    .await?;
                println!("ok");
            }
            DatastoreCommand::Delete {
                universe_id,
                data_store_id,
                entry_id,
                scope,
            } => {
                client
                    .delete_entry(universe_id, &data_store_id, &entry_id, scope.as_deref())
                    .await?;
                println!("ok");
            }
            DatastoreCommand::List {
                universe_id,
                data_store_id,
                scope,
                filter,
            } => {
                let result = client
                    .list_entries(
                        universe_id,
                        &data_store_id,
                        scope.as_deref(),
                        filter.as_deref(),
                        None,
                        None,
                    )
                    .await?;
                for entry in result.data_store_entries {
                    println!("{}", entry.id);
                }
            }
            DatastoreCommand::ListScopes {
                universe_id,
                data_store_id,
            } => {
                let mut scopes = std::collections::BTreeSet::new();
                let mut page_token = None;
                loop {
                    let result = client
                        .list_entries(
                            universe_id,
                            &data_store_id,
                            None,
                            None,
                            page_token.as_deref(),
                            Some(256),
                        )
                        .await?;
                    for entry in result.data_store_entries {
                        if let Some((scope, _)) = entry.id.split_once('/') {
                            scopes.insert(scope.to_string());
                        }
                    }
                    page_token = result.next_page_token.filter(|t| !t.is_empty());
                    if page_token.is_none() {
                        break;
                    }
                }
                for scope in scopes {
                    println!("{scope}");
                }
            }
        },
        Command::OrderedDatastore(cmd) => match cmd {
            OrderedDatastoreCommand::List {
                universe_id,
                ordered_data_store_id,
                scope,
                order_by,
                filter,
                max_page_size,
            } => {
                let result = client
                    .list_ordered_entries(
                        universe_id,
                        &ordered_data_store_id,
                        &scope,
                        order_by.as_deref(),
                        filter.as_deref(),
                        None,
                        max_page_size,
                    )
                    .await?;
                for entry in result.ordered_data_store_entries {
                    println!("{}\t{}", entry.id, entry.value);
                }
            }
            OrderedDatastoreCommand::Create {
                universe_id,
                ordered_data_store_id,
                entry_id,
                value,
                scope,
            } => {
                let entry = client
                    .create_ordered_entry(
                        universe_id,
                        &ordered_data_store_id,
                        &scope,
                        &entry_id,
                        value,
                    )
                    .await?;
                println!("{}", serde_json::to_string_pretty(&entry)?);
            }
            OrderedDatastoreCommand::Get {
                universe_id,
                ordered_data_store_id,
                entry_id,
                scope,
            } => {
                let entry = client
                    .get_ordered_entry(universe_id, &ordered_data_store_id, &scope, &entry_id)
                    .await?;
                println!("{}", serde_json::to_string_pretty(&entry)?);
            }
            OrderedDatastoreCommand::Update {
                universe_id,
                ordered_data_store_id,
                entry_id,
                value,
                scope,
            } => {
                let entry = client
                    .update_ordered_entry(
                        universe_id,
                        &ordered_data_store_id,
                        &scope,
                        &entry_id,
                        value,
                    )
                    .await?;
                println!("{}", serde_json::to_string_pretty(&entry)?);
            }
            OrderedDatastoreCommand::Delete {
                universe_id,
                ordered_data_store_id,
                entry_id,
                scope,
            } => {
                client
                    .delete_ordered_entry(universe_id, &ordered_data_store_id, &scope, &entry_id)
                    .await?;
                println!("ok");
            }
            OrderedDatastoreCommand::Increment {
                universe_id,
                ordered_data_store_id,
                entry_id,
                amount,
                scope,
            } => {
                let entry = client
                    .increment_ordered_entry(
                        universe_id,
                        &ordered_data_store_id,
                        &scope,
                        &entry_id,
                        amount,
                    )
                    .await?;
                println!("{}", serde_json::to_string_pretty(&entry)?);
            }
        },
        Command::Messaging(cmd) => match cmd {
            MessagingCommand::Publish {
                universe_id,
                topic,
                message,
            } => {
                client
                    .publish_message(universe_id, &topic, &message)
                    .await?;
                println!("ok");
            }
        },
        Command::Login | Command::Logout => unreachable!(),
    }

    Ok(())
}
