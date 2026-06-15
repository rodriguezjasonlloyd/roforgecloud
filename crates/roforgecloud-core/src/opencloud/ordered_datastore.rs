use reqwest::Method;
use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::opencloud::client::{
    encode_path_segment, item_path, universe_path, ListQuery, OpenCloudClient,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderedDataStoreEntry {
    #[serde(default)]
    pub path: String,
    pub value: f64,
    #[serde(default)]
    pub id: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListOrderedEntriesResponse {
    #[serde(default)]
    pub ordered_data_store_entries: Vec<OrderedDataStoreEntry>,
    #[serde(default)]
    pub next_page_token: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct CreateOrderedDataStoreEntryRequest {
    value: f64,
}

#[derive(Debug, Clone, Serialize)]
struct UpdateOrderedDataStoreEntryRequest {
    value: f64,
}

#[derive(Debug, Clone, Serialize)]
struct IncrementOrderedDataStoreEntryRequest {
    amount: f64,
}

impl OpenCloudClient {
    fn ordered_entries_path(
        &self,
        universe_id: u64,
        ordered_data_store_id: &str,
        scope: &str,
    ) -> String {
        let ordered_data_store_path = item_path(
            &universe_path(universe_id, "/ordered-data-stores"),
            ordered_data_store_id,
        );
        format!(
            "{ordered_data_store_path}/scopes/{}/entries",
            encode_path_segment(scope)
        )
    }

    pub async fn list_ordered_entries(
        &self,
        universe_id: u64,
        ordered_data_store_id: &str,
        scope: &str,
        query: &ListQuery<'_>,
    ) -> Result<ListOrderedEntriesResponse> {
        let path = self.ordered_entries_path(universe_id, ordered_data_store_id, scope);
        let builder = self.request(Method::GET, &path).query(query);
        self.send_json(builder).await
    }

    pub async fn create_ordered_entry(
        &self,
        universe_id: u64,
        ordered_data_store_id: &str,
        scope: &str,
        entry_id: &str,
        value: f64,
    ) -> Result<OrderedDataStoreEntry> {
        let path = self.ordered_entries_path(universe_id, ordered_data_store_id, scope);
        let builder = self
            .request(Method::POST, &path)
            .query(&[("id", entry_id)])
            .json(&CreateOrderedDataStoreEntryRequest { value });
        self.send_json(builder).await
    }

    pub async fn get_ordered_entry(
        &self,
        universe_id: u64,
        ordered_data_store_id: &str,
        scope: &str,
        entry_id: &str,
    ) -> Result<OrderedDataStoreEntry> {
        let path = item_path(
            &self.ordered_entries_path(universe_id, ordered_data_store_id, scope),
            entry_id,
        );
        let builder = self.request(Method::GET, &path);
        self.send_json(builder).await
    }

    pub async fn update_ordered_entry(
        &self,
        universe_id: u64,
        ordered_data_store_id: &str,
        scope: &str,
        entry_id: &str,
        value: f64,
    ) -> Result<OrderedDataStoreEntry> {
        let path = item_path(
            &self.ordered_entries_path(universe_id, ordered_data_store_id, scope),
            entry_id,
        );
        let builder = self
            .request(Method::PATCH, &path)
            .json(&UpdateOrderedDataStoreEntryRequest { value });
        self.send_json(builder).await
    }

    pub async fn delete_ordered_entry(
        &self,
        universe_id: u64,
        ordered_data_store_id: &str,
        scope: &str,
        entry_id: &str,
    ) -> Result<()> {
        let path = item_path(
            &self.ordered_entries_path(universe_id, ordered_data_store_id, scope),
            entry_id,
        );
        let builder = self.request(Method::DELETE, &path);
        self.send(builder).await?;
        Ok(())
    }

    pub async fn increment_ordered_entry(
        &self,
        universe_id: u64,
        ordered_data_store_id: &str,
        scope: &str,
        entry_id: &str,
        amount: f64,
    ) -> Result<OrderedDataStoreEntry> {
        let path = format!(
            "{}:increment",
            item_path(
                &self.ordered_entries_path(universe_id, ordered_data_store_id, scope),
                entry_id,
            )
        );
        let builder = self
            .request(Method::POST, &path)
            .json(&IncrementOrderedDataStoreEntryRequest { amount });
        self.send_json(builder).await
    }
}
