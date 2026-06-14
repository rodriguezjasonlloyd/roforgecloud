use reqwest::Method;
use serde::Serialize;

use crate::error::Result;
use crate::opencloud::client::OpenCloudClient;

#[derive(Debug, Serialize)]
struct PublishMessageRequest<'a> {
    topic: &'a str,
    message: &'a str,
}

impl OpenCloudClient {
    pub async fn publish_message(
        &self,
        universe_id: u64,
        topic: &str,
        message: &str,
    ) -> Result<()> {
        let path = format!("/cloud/v2/universes/{universe_id}:publishMessage");
        let builder = self
            .request(Method::POST, &path)
            .json(&PublishMessageRequest { topic, message });
        self.send(builder).await?;
        Ok(())
    }
}
