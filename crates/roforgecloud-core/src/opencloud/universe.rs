use reqwest::Method;
use serde::Deserialize;

use crate::error::Result;
use crate::opencloud::client::{universe_path, OpenCloudClient};

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UniverseInfo {
    pub display_name: String,
}

impl OpenCloudClient {
    pub async fn get_universe(&self, universe_id: u64) -> Result<UniverseInfo> {
        let path = universe_path(universe_id, "");
        let builder = self.request(Method::GET, &path);
        self.send_json(builder).await
    }
}
