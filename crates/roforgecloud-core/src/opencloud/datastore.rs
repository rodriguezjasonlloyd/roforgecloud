use reqwest::Method;
use serde::Deserialize;

use crate::error::Result;
use crate::opencloud::client::{encode_path_segment, OpenCloudClient};

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
    fn entries_path(&self, universe_id: u64, data_store_id: &str, scope: Option<&str>) -> String {
        let data_store_id = encode_path_segment(data_store_id);
        match scope {
            Some(scope) => {
                let scope = encode_path_segment(scope);
                format!(
                    "/cloud/v2/universes/{universe_id}/data-stores/{data_store_id}/scopes/{scope}/entries"
                )
            }
            None => {
                format!("/cloud/v2/universes/{universe_id}/data-stores/{data_store_id}/entries")
            }
        }
    }

    pub async fn list_data_stores(
        &self,
        universe_id: u64,
        page_token: Option<&str>,
        max_page_size: Option<u32>,
        show_deleted: bool,
    ) -> Result<ListDataStoresResponse> {
        let path = format!("/cloud/v2/universes/{universe_id}/data-stores");

        let mut query = Vec::new();
        if let Some(page_token) = page_token {
            query.push(("pageToken".to_string(), page_token.to_string()));
        }
        if let Some(max_page_size) = max_page_size {
            query.push(("maxPageSize".to_string(), max_page_size.to_string()));
        }
        if show_deleted {
            query.push(("showDeleted".to_string(), "true".to_string()));
        }

        let builder = self.request(Method::GET, &path).query(&query);
        self.send_json(builder).await
    }

    pub async fn delete_data_store(
        &self,
        universe_id: u64,
        data_store_id: &str,
    ) -> Result<DataStoreInfo> {
        let data_store_id = encode_path_segment(data_store_id);
        let path = format!("/cloud/v2/universes/{universe_id}/data-stores/{data_store_id}");

        let builder = self.request(Method::DELETE, &path);
        self.send_json(builder).await
    }

    pub async fn undelete_data_store(
        &self,
        universe_id: u64,
        data_store_id: &str,
    ) -> Result<DataStoreInfo> {
        let data_store_id = encode_path_segment(data_store_id);
        let path =
            format!("/cloud/v2/universes/{universe_id}/data-stores/{data_store_id}:undelete");

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
        filter: Option<&str>,
        page_token: Option<&str>,
        max_page_size: Option<u32>,
    ) -> Result<ListEntriesResponse> {
        let path = self.entries_path(universe_id, data_store_id, scope);

        let mut query = Vec::new();
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
        let path = format!(
            "{}/{}",
            self.entries_path(universe_id, data_store_id, scope),
            encode_path_segment(entry_id)
        );
        let builder = self.request(Method::GET, &path);
        let entry: serde_json::Value = self.send_json(builder).await?;
        Ok(match entry {
            serde_json::Value::Object(mut obj) => {
                let revision = obj
                    .get("revisionId")
                    .or_else(|| obj.get("etag"))
                    .and_then(|v| v.as_str())
                    .map(String::from);
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
        let path = format!(
            "{}/{}",
            self.entries_path(universe_id, data_store_id, scope),
            encode_path_segment(entry_id)
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
        Ok(response
            .get("revisionId")
            .or_else(|| response.get("etag"))
            .and_then(|v| v.as_str())
            .map(String::from))
    }

    pub async fn delete_entry(
        &self,
        universe_id: u64,
        data_store_id: &str,
        entry_id: &str,
        scope: Option<&str>,
    ) -> Result<()> {
        let path = format!(
            "{}/{}",
            self.entries_path(universe_id, data_store_id, scope),
            encode_path_segment(entry_id)
        );
        let builder = self.request(Method::DELETE, &path);
        self.send(builder).await?;
        Ok(())
    }
}
