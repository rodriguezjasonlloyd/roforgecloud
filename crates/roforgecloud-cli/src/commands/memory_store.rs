use anyhow::Result;
use clap::Subcommand;
use roforgecloud_core::opencloud::{ListQuery, OpenCloudClient};

use crate::output::{parse_value, print_json, print_ok};

#[derive(Subcommand)]
pub enum MemoryStoreCommand {
    #[command(subcommand)]
    SortedMap(SortedMapCommand),
    #[command(subcommand)]
    Queue(QueueCommand),
}

#[derive(Subcommand)]
pub enum SortedMapCommand {
    Get {
        universe_id: u64,
        sorted_map: String,
        item_id: String,
    },
    Set {
        universe_id: u64,
        sorted_map: String,
        item_id: String,
        value: String,
        #[arg(long, default_value = "3600")]
        ttl: u64,
    },
    Create {
        universe_id: u64,
        sorted_map: String,
        item_id: String,
        value: String,
        #[arg(long, default_value = "3600")]
        ttl: u64,
    },
    Delete {
        universe_id: u64,
        sorted_map: String,
        item_id: String,
    },
    List {
        universe_id: u64,
        sorted_map: String,
        #[arg(long)]
        max_page_size: Option<u32>,
    },
}

#[derive(Subcommand)]
pub enum QueueCommand {
    Read {
        universe_id: u64,
        queue: String,
        #[arg(long, default_value = "10")]
        count: u32,
        #[arg(long, default_value = "30")]
        invisibility_window: u32,
        #[arg(long)]
        all_or_nothing: bool,
    },
    Add {
        universe_id: u64,
        queue: String,
        value: String,
        #[arg(long)]
        id: Option<String>,
        #[arg(long)]
        priority: Option<i64>,
        #[arg(long, default_value = "3600")]
        ttl: u64,
    },
    Delete {
        universe_id: u64,
        queue: String,
        item_id: String,
    },
}

pub async fn run(client: &OpenCloudClient, cmd: MemoryStoreCommand) -> Result<()> {
    match cmd {
        MemoryStoreCommand::SortedMap(cmd) => run_sorted_map(client, cmd).await,
        MemoryStoreCommand::Queue(cmd) => run_queue(client, cmd).await,
    }
}

async fn run_sorted_map(client: &OpenCloudClient, cmd: SortedMapCommand) -> Result<()> {
    match cmd {
        SortedMapCommand::Get {
            universe_id,
            sorted_map,
            item_id,
        } => {
            let item = client
                .get_sorted_map_item(universe_id, &sorted_map, &item_id)
                .await?;
            print_json(&item)?;
        }
        SortedMapCommand::Set {
            universe_id,
            sorted_map,
            item_id,
            value,
            ttl,
        } => {
            let value = parse_value(&value)?;
            let item = client
                .update_sorted_map_item(universe_id, &sorted_map, &item_id, &value, ttl, None)
                .await?;
            print_json(&item)?;
        }
        SortedMapCommand::Create {
            universe_id,
            sorted_map,
            item_id,
            value,
            ttl,
        } => {
            let value = parse_value(&value)?;
            let item = client
                .create_sorted_map_item(universe_id, &sorted_map, &item_id, &value, ttl)
                .await?;
            print_json(&item)?;
        }
        SortedMapCommand::Delete {
            universe_id,
            sorted_map,
            item_id,
        } => {
            client
                .delete_sorted_map_item(universe_id, &sorted_map, &item_id)
                .await?;
            print_ok();
        }
        SortedMapCommand::List {
            universe_id,
            sorted_map,
            max_page_size,
        } => {
            let query = ListQuery {
                max_page_size,
                ..Default::default()
            };
            let result = client
                .list_sorted_map_items(universe_id, &sorted_map, &query)
                .await?;
            print_json(&result)?;
        }
    }

    Ok(())
}

async fn run_queue(client: &OpenCloudClient, cmd: QueueCommand) -> Result<()> {
    match cmd {
        QueueCommand::Read {
            universe_id,
            queue,
            count,
            invisibility_window,
            all_or_nothing,
        } => {
            let result = client
                .read_queue_items(
                    universe_id,
                    &queue,
                    count,
                    invisibility_window,
                    all_or_nothing,
                )
                .await?;
            print_json(&result)?;
        }
        QueueCommand::Add {
            universe_id,
            queue,
            value,
            id,
            priority,
            ttl,
        } => {
            let value = parse_value(&value)?;
            let item = client
                .add_queue_item(universe_id, &queue, id.as_deref(), &value, priority, ttl)
                .await?;
            print_json(&item)?;
        }
        QueueCommand::Delete {
            universe_id,
            queue,
            item_id,
        } => {
            client
                .delete_queue_item(universe_id, &queue, &item_id)
                .await?;
            print_ok();
        }
    }

    Ok(())
}
