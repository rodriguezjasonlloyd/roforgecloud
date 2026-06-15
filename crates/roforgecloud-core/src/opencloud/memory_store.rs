use reqwest::Method;
use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::opencloud::client::{item_path, universe_path, ListQuery, OpenCloudClient};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SortedMapItem {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub value: serde_json::Value,
    #[serde(default)]
    pub etag: Option<String>,
    #[serde(default)]
    pub expire_time: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListSortedMapItemsResponse {
    #[serde(default)]
    pub items: Vec<SortedMapItem>,
    #[serde(default)]
    pub next_page_token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QueueItem {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub value: serde_json::Value,
    #[serde(default)]
    pub priority: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReadQueueItemsResponse {
    #[serde(default)]
    pub items: Vec<QueueItem>,
}

#[derive(Debug, Clone, Serialize)]
struct SortedMapItemRequest<'a> {
    value: &'a serde_json::Value,
    ttl: String,
}

#[derive(Debug, Clone, Serialize)]
struct AddQueueItemRequest<'a> {
    value: &'a serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    priority: Option<i64>,
    ttl: String,
}

impl OpenCloudClient {
    fn sorted_map_items_path(&self, universe_id: u64, sorted_map: &str) -> String {
        format!(
            "{}/items",
            item_path(
                &universe_path(universe_id, "/memory-store/sorted-maps"),
                sorted_map
            )
        )
    }

    fn queue_items_path(&self, universe_id: u64, queue: &str) -> String {
        format!(
            "{}/items",
            item_path(&universe_path(universe_id, "/memory-store/queues"), queue)
        )
    }

    pub async fn list_sorted_map_items(
        &self,
        universe_id: u64,
        sorted_map: &str,
        query: &ListQuery<'_>,
    ) -> Result<ListSortedMapItemsResponse> {
        let path = self.sorted_map_items_path(universe_id, sorted_map);
        let builder = self.request(Method::GET, &path).query(query);
        self.send_json(builder).await
    }

    pub async fn get_sorted_map_item(
        &self,
        universe_id: u64,
        sorted_map: &str,
        item_id: &str,
    ) -> Result<SortedMapItem> {
        let path = item_path(
            &self.sorted_map_items_path(universe_id, sorted_map),
            item_id,
        );
        let builder = self.request(Method::GET, &path);
        self.send_json(builder).await
    }

    pub async fn create_sorted_map_item(
        &self,
        universe_id: u64,
        sorted_map: &str,
        item_id: &str,
        value: &serde_json::Value,
        ttl_seconds: u64,
    ) -> Result<SortedMapItem> {
        let path = self.sorted_map_items_path(universe_id, sorted_map);
        let builder = self
            .request(Method::POST, &path)
            .query(&[("id", item_id)])
            .json(&SortedMapItemRequest {
                value,
                ttl: format!("{ttl_seconds}s"),
            });
        self.send_json(builder).await
    }

    pub async fn update_sorted_map_item(
        &self,
        universe_id: u64,
        sorted_map: &str,
        item_id: &str,
        value: &serde_json::Value,
        ttl_seconds: u64,
        etag: Option<&str>,
    ) -> Result<SortedMapItem> {
        let path = item_path(
            &self.sorted_map_items_path(universe_id, sorted_map),
            item_id,
        );
        let mut query = Vec::new();
        if let Some(etag) = etag {
            query.push(("etag".to_string(), etag.to_string()));
        }
        let builder =
            self.request(Method::PATCH, &path)
                .query(&query)
                .json(&SortedMapItemRequest {
                    value,
                    ttl: format!("{ttl_seconds}s"),
                });
        self.send_json(builder).await
    }

    pub async fn delete_sorted_map_item(
        &self,
        universe_id: u64,
        sorted_map: &str,
        item_id: &str,
    ) -> Result<()> {
        let path = item_path(
            &self.sorted_map_items_path(universe_id, sorted_map),
            item_id,
        );
        let builder = self.request(Method::DELETE, &path);
        self.send(builder).await?;
        Ok(())
    }

    pub async fn add_queue_item(
        &self,
        universe_id: u64,
        queue: &str,
        item_id: Option<&str>,
        value: &serde_json::Value,
        priority: Option<i64>,
        ttl_seconds: u64,
    ) -> Result<QueueItem> {
        let path = self.queue_items_path(universe_id, queue);
        let mut builder = self.request(Method::POST, &path);
        if let Some(item_id) = item_id {
            builder = builder.query(&[("id", item_id)]);
        }
        let builder = builder.json(&AddQueueItemRequest {
            value,
            priority,
            ttl: format!("{ttl_seconds}s"),
        });
        self.send_json(builder).await
    }

    pub async fn read_queue_items(
        &self,
        universe_id: u64,
        queue: &str,
        count: u32,
        invisibility_window_seconds: u32,
        all_or_nothing: bool,
    ) -> Result<ReadQueueItemsResponse> {
        let path = self.queue_items_path(universe_id, queue);
        let query = [
            ("count".to_string(), count.to_string()),
            (
                "invisibilityWindowSeconds".to_string(),
                invisibility_window_seconds.to_string(),
            ),
            ("allOrNothing".to_string(), all_or_nothing.to_string()),
        ];
        let builder = self.request(Method::GET, &path).query(&query);
        self.send_json(builder).await
    }

    pub async fn delete_queue_item(
        &self,
        universe_id: u64,
        queue: &str,
        item_id: &str,
    ) -> Result<()> {
        let path = item_path(&self.queue_items_path(universe_id, queue), item_id);
        let builder = self.request(Method::DELETE, &path);
        self.send(builder).await?;
        Ok(())
    }
}
