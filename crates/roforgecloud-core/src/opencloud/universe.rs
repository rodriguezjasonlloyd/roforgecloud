use reqwest::Method;
use serde::Deserialize;

use crate::error::Result;
use crate::opencloud::client::OpenCloudClient;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UniverseInfo {
    pub display_name: String,
}

impl OpenCloudClient {
    pub async fn get_universe(&self, universe_id: u64) -> Result<UniverseInfo> {
        let path = format!("/cloud/v2/universes/{universe_id}");
        let builder = self.request(Method::GET, &path);
        self.send_json(builder).await
    }
}
