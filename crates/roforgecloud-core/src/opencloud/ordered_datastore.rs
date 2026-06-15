use reqwest::Method;
use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::opencloud::client::{encode_path_segment, OpenCloudClient};

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
        let ordered_data_store_id = encode_path_segment(ordered_data_store_id);
        let scope = encode_path_segment(scope);
        format!(
            "/cloud/v2/universes/{universe_id}/ordered-data-stores/{ordered_data_store_id}/scopes/{scope}/entries"
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn list_ordered_entries(
        &self,
        universe_id: u64,
        ordered_data_store_id: &str,
        scope: &str,
        order_by: Option<&str>,
        filter: Option<&str>,
        page_token: Option<&str>,
        max_page_size: Option<u32>,
    ) -> Result<ListOrderedEntriesResponse> {
        let path = self.ordered_entries_path(universe_id, ordered_data_store_id, scope);

        let mut query = Vec::new();
        if let Some(order_by) = order_by {
            query.push(("orderBy".to_string(), order_by.to_string()));
        }
        if let Some(filter) = filter {
            query.push(("filter".to_string(), filter.to_string()));
        }
        if let Some(page_token) = page_token {
            query.push(("pageToken".to_string(), page_token.to_string()));
        }
        if let Some(max_page_size) = max_page_size {
            query.push(("maxPageSize".to_string(), max_page_size.to_string()));
        }

        let builder = self.request(Method::GET, &path).query(&query);
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
        let path = format!(
            "{}/{}",
            self.ordered_entries_path(universe_id, ordered_data_store_id, scope),
            encode_path_segment(entry_id)
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
        let path = format!(
            "{}/{}",
            self.ordered_entries_path(universe_id, ordered_data_store_id, scope),
            encode_path_segment(entry_id)
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
        let path = format!(
            "{}/{}",
            self.ordered_entries_path(universe_id, ordered_data_store_id, scope),
            encode_path_segment(entry_id)
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
            "{}/{}:increment",
            self.ordered_entries_path(universe_id, ordered_data_store_id, scope),
            encode_path_segment(entry_id)
        );
        let builder = self
            .request(Method::POST, &path)
            .json(&IncrementOrderedDataStoreEntryRequest { amount });
        self.send_json(builder).await
    }
}
