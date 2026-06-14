use clap::{Parser, Subcommand};
use roforgecloud_core::opencloud::{Credentials, OpenCloudClient};

#[derive(Parser)]
#[command(name = "roforgecloud", about = "Roblox Open Cloud companion CLI")]
struct Cli {
    #[arg(long, env = "ROFORGE_API_KEY", global = true)]
    api_key: Option<String>,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    #[command(subcommand)]
    Datastore(DatastoreCommand),
    #[command(subcommand)]
    Messaging(MessagingCommand),
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
    }

    Ok(())
}
