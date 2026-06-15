use reqwest::Method;
use serde::Serialize;

use crate::error::{Error, Result};
use crate::opencloud::client::{universe_path, Credentials, OpenCloudClient};

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
        if matches!(self.credentials(), Credentials::OAuthToken(_)) {
            return Err(Error::OAuth(
                "publish_message requires an API key, OAuth is not supported".to_string(),
            ));
        }

        let path = universe_path(universe_id, ":publishMessage");
        let builder = self
            .request(Method::POST, &path)
            .json(&PublishMessageRequest { topic, message });
        self.send(builder).await?;
        Ok(())
    }
}
