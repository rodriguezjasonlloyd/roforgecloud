use anyhow::Result;
use clap::Subcommand;
use roforgecloud_core::opencloud::{ListQuery, OpenCloudClient};

use crate::output::{print_json, print_ok, print_table};

#[derive(Subcommand)]
pub enum OrderedDatastoreCommand {
    Get {
        universe_id: u64,
        ordered_data_store_id: String,
        entry_id: String,
        #[arg(long, default_value = "global")]
        scope: String,
    },
    Create {
        universe_id: u64,
        ordered_data_store_id: String,
        entry_id: String,
        value: f64,
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
}

pub async fn run(client: &OpenCloudClient, cmd: OrderedDatastoreCommand) -> Result<()> {
    match cmd {
        OrderedDatastoreCommand::Get {
            universe_id,
            ordered_data_store_id,
            entry_id,
            scope,
        } => {
            let entry = client
                .get_ordered_entry(universe_id, &ordered_data_store_id, &scope, &entry_id)
                .await?;
            print_json(&entry)?;
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
            print_json(&entry)?;
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
            print_json(&entry)?;
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
            print_ok();
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
            print_json(&entry)?;
        }
        OrderedDatastoreCommand::List {
            universe_id,
            ordered_data_store_id,
            scope,
            order_by,
            filter,
            max_page_size,
        } => {
            let query = ListQuery {
                order_by: order_by.as_deref(),
                filter: filter.as_deref(),
                max_page_size,
                ..Default::default()
            };
            let result = client
                .list_ordered_entries(universe_id, &ordered_data_store_id, &scope, &query)
                .await?;
            let rows = result
                .ordered_data_store_entries
                .into_iter()
                .map(|e| vec![e.id, e.value.to_string()])
                .collect();
            print_table(&["ID", "Value"], rows);
        }
    }

    Ok(())
}
