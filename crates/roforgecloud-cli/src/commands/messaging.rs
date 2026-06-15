use anyhow::Result;
use clap::Subcommand;
use roforgecloud_core::opencloud::OpenCloudClient;

use crate::output::print_ok;

#[derive(Subcommand)]
pub enum MessagingCommand {
    Publish {
        universe_id: u64,
        topic: String,
        message: String,
    },
}

pub async fn run(client: &OpenCloudClient, cmd: MessagingCommand) -> Result<()> {
    match cmd {
        MessagingCommand::Publish {
            universe_id,
            topic,
            message,
        } => {
            client
                .publish_message(universe_id, &topic, &message)
                .await?;
            print_ok();
        }
    }

    Ok(())
}
