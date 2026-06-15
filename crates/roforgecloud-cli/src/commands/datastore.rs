use anyhow::Result;
use clap::Subcommand;
use roforgecloud_core::opencloud::{ListQuery, OpenCloudClient};

use crate::output::{parse_value, print_json, print_ok, print_table};

#[derive(Subcommand)]
pub enum DatastoreCommand {
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
    Create {
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
    ListStores {
        universe_id: u64,
    },
    ListScopes {
        universe_id: u64,
        data_store_id: String,
    },
    List {
        universe_id: u64,
        data_store_id: String,
        #[arg(long)]
        scope: Option<String>,
        #[arg(long)]
        filter: Option<String>,
    },
}

pub async fn run(client: &OpenCloudClient, cmd: DatastoreCommand) -> Result<()> {
    match cmd {
        DatastoreCommand::Get {
            universe_id,
            data_store_id,
            entry_id,
            scope,
        } => {
            let value = client
                .get_entry(universe_id, &data_store_id, &entry_id, scope.as_deref())
                .await?;
            print_json(&value)?;
        }
        DatastoreCommand::Set {
            universe_id,
            data_store_id,
            entry_id,
            value,
            scope,
        } => {
            let value = parse_value(&value)?;
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
            print_ok();
        }
        DatastoreCommand::Create {
            universe_id,
            data_store_id,
            entry_id,
            value,
            scope,
        } => {
            let value = parse_value(&value)?;
            client
                .create_entry(
                    universe_id,
                    &data_store_id,
                    &entry_id,
                    scope.as_deref(),
                    &value,
                )
                .await?;
            print_ok();
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
            print_ok();
        }
        DatastoreCommand::ListStores { universe_id } => {
            let result = client
                .list_data_stores(universe_id, &ListQuery::default())
                .await?;
            let rows = result.data_stores.into_iter().map(|s| vec![s.id]).collect();
            print_table(&["ID"], rows);
        }
        DatastoreCommand::ListScopes {
            universe_id,
            data_store_id,
        } => {
            let mut scopes = std::collections::BTreeSet::new();
            let mut page_token = None;
            loop {
                let query = ListQuery {
                    page_token: page_token.as_deref(),
                    max_page_size: Some(256),
                    ..Default::default()
                };
                let result = client
                    .list_entries(universe_id, &data_store_id, None, &query)
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
            let rows = scopes.into_iter().map(|s| vec![s]).collect();
            print_table(&["Scope"], rows);
        }
        DatastoreCommand::List {
            universe_id,
            data_store_id,
            scope,
            filter,
        } => {
            let query = ListQuery {
                filter: filter.as_deref(),
                ..Default::default()
            };
            let result = client
                .list_entries(universe_id, &data_store_id, scope.as_deref(), &query)
                .await?;
            let rows = result
                .data_store_entries
                .into_iter()
                .map(|e| vec![e.id])
                .collect();
            print_table(&["ID"], rows);
        }
    }

    Ok(())
}
