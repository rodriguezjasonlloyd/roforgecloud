use reqwest::Method;
use serde::Deserialize;

use crate::error::Result;
use crate::opencloud::client::{
    encode_path_segment, extract_revision, item_path, universe_path, ListQuery, OpenCloudClient,
};

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DataStoreInfo {
    pub path: String,
    pub id: String,
    pub create_time: String,
    #[serde(default)]
    pub expire_time: Option<String>,
    #[serde(default)]
    pub state: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListDataStoresResponse {
    #[serde(default)]
    pub data_stores: Vec<DataStoreInfo>,
    #[serde(default)]
    pub next_page_token: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DataStoreEntryInfo {
    pub path: String,
    pub id: String,
    #[serde(default)]
    pub create_time: Option<String>,
    #[serde(default)]
    pub revision_id: Option<String>,
    #[serde(default)]
    pub revision_create_time: Option<String>,
    #[serde(default)]
    pub state: Option<String>,
    #[serde(default)]
    pub etag: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListEntriesResponse {
    #[serde(default)]
    pub data_store_entries: Vec<DataStoreEntryInfo>,
    #[serde(default)]
    pub next_page_token: Option<String>,
}

impl OpenCloudClient {
    fn data_stores_path(&self, universe_id: u64) -> String {
        universe_path(universe_id, "/data-stores")
    }

    fn entries_path(&self, universe_id: u64, data_store_id: &str, scope: Option<&str>) -> String {
        let data_store_path = item_path(&self.data_stores_path(universe_id), data_store_id);
        match scope {
            Some(scope) => format!(
                "{data_store_path}/scopes/{}/entries",
                encode_path_segment(scope)
            ),
            None => format!("{data_store_path}/entries"),
        }
    }

    pub async fn list_data_stores(
        &self,
        universe_id: u64,
        query: &ListQuery<'_>,
    ) -> Result<ListDataStoresResponse> {
        let path = self.data_stores_path(universe_id);
        let builder = self.request(Method::GET, &path).query(query);
        self.send_json(builder).await
    }

    pub async fn delete_data_store(
        &self,
        universe_id: u64,
        data_store_id: &str,
    ) -> Result<DataStoreInfo> {
        let path = item_path(&self.data_stores_path(universe_id), data_store_id);

        let builder = self.request(Method::DELETE, &path);
        self.send_json(builder).await
    }

    pub async fn undelete_data_store(
        &self,
        universe_id: u64,
        data_store_id: &str,
    ) -> Result<DataStoreInfo> {
        let path = format!(
            "{}:undelete",
            item_path(&self.data_stores_path(universe_id), data_store_id)
        );

        let builder = self
            .request(Method::POST, &path)
            .json(&serde_json::json!({}));
        self.send_json(builder).await
    }

    pub async fn list_entries(
        &self,
        universe_id: u64,
        data_store_id: &str,
        scope: Option<&str>,
        query: &ListQuery<'_>,
    ) -> Result<ListEntriesResponse> {
        let path = self.entries_path(universe_id, data_store_id, scope);
        let builder = self.request(Method::GET, &path).query(query);
        self.send_json(builder).await
    }

    pub async fn get_entry(
        &self,
        universe_id: u64,
        data_store_id: &str,
        entry_id: &str,
        scope: Option<&str>,
    ) -> Result<serde_json::Value> {
        let (value, _) = self
            .get_entry_with_revision(universe_id, data_store_id, entry_id, scope)
            .await?;
        Ok(value)
    }

    pub async fn get_entry_with_revision(
        &self,
        universe_id: u64,
        data_store_id: &str,
        entry_id: &str,
        scope: Option<&str>,
    ) -> Result<(serde_json::Value, Option<String>)> {
        let path = item_path(
            &self.entries_path(universe_id, data_store_id, scope),
            entry_id,
        );
        let builder = self.request(Method::GET, &path);
        let entry: serde_json::Value = self.send_json(builder).await?;
        Ok(match entry {
            serde_json::Value::Object(mut obj) => {
                let revision = extract_revision(&obj);
                let value = obj
                    .remove("value")
                    .unwrap_or(serde_json::Value::Object(obj));
                (value, revision)
            }
            other => (other, None),
        })
    }

    pub async fn create_entry(
        &self,
        universe_id: u64,
        data_store_id: &str,
        entry_id: &str,
        scope: Option<&str>,
        value: &serde_json::Value,
    ) -> Result<()> {
        let path = self.entries_path(universe_id, data_store_id, scope);
        let builder = self
            .request(Method::POST, &path)
            .query(&[("id", entry_id)])
            .json(&serde_json::json!({ "value": value }));
        self.send(builder).await?;
        Ok(())
    }

    pub async fn set_entry(
        &self,
        universe_id: u64,
        data_store_id: &str,
        entry_id: &str,
        scope: Option<&str>,
        value: &serde_json::Value,
        match_version: Option<&str>,
    ) -> Result<Option<String>> {
        let path = item_path(
            &self.entries_path(universe_id, data_store_id, scope),
            entry_id,
        );
        let mut query = vec![("allowMissing".to_string(), "true".to_string())];
        if let Some(match_version) = match_version {
            query.push(("matchVersion".to_string(), match_version.to_string()));
        }
        let builder = self
            .request(Method::PATCH, &path)
            .query(&query)
            .json(&serde_json::json!({ "value": value }));
        let response: serde_json::Value = self.send_json(builder).await?;
        Ok(response.as_object().and_then(extract_revision))
    }

    pub async fn delete_entry(
        &self,
        universe_id: u64,
        data_store_id: &str,
        entry_id: &str,
        scope: Option<&str>,
    ) -> Result<()> {
        let path = item_path(
            &self.entries_path(universe_id, data_store_id, scope),
            entry_id,
        );
        let builder = self.request(Method::DELETE, &path);
        self.send(builder).await?;
        Ok(())
    }
}
