use percent_encoding::{AsciiSet, NON_ALPHANUMERIC};
use reqwest::{Client, Method, RequestBuilder};
use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::error::{check_status, Result};

const DEFAULT_BASE_URL: &str = "https://apis.roblox.com";

const PATH_SEGMENT: &AsciiSet = &NON_ALPHANUMERIC
    .remove(b'-')
    .remove(b'_')
    .remove(b'.')
    .remove(b'~');

pub(crate) fn encode_path_segment(value: &str) -> String {
    percent_encoding::utf8_percent_encode(value, PATH_SEGMENT).to_string()
}

pub(crate) fn item_path(collection_path: &str, item_id: &str) -> String {
    format!("{collection_path}/{}", encode_path_segment(item_id))
}

pub(crate) fn universe_path(universe_id: u64, suffix: &str) -> String {
    format!("/cloud/v2/universes/{universe_id}{suffix}")
}

pub(crate) fn extract_revision(obj: &serde_json::Map<String, serde_json::Value>) -> Option<String> {
    obj.get("revisionId")
        .or_else(|| obj.get("etag"))
        .and_then(|v| v.as_str())
        .map(String::from)
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ListQuery<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filter: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub order_by: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_token: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_page_size: Option<u32>,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub show_deleted: bool,
}

#[derive(Debug, Clone)]
pub enum Credentials {
    ApiKey(String),
    OAuthToken(String),
}

#[derive(Debug, Clone)]
pub struct OpenCloudClient {
    http: Client,
    base_url: String,
    credentials: Credentials,
}

impl OpenCloudClient {
    pub fn new(credentials: Credentials) -> Self {
        Self {
            http: Client::new(),
            base_url: DEFAULT_BASE_URL.to_string(),
            credentials,
        }
    }

    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = base_url.into();
        self
    }

    fn authorize(&self, builder: RequestBuilder) -> RequestBuilder {
        match &self.credentials {
            Credentials::ApiKey(key) => builder.header("x-api-key", key),
            Credentials::OAuthToken(token) => builder.bearer_auth(token),
        }
    }

    pub(crate) fn credentials(&self) -> &Credentials {
        &self.credentials
    }

    pub(crate) fn request(&self, method: Method, path: &str) -> RequestBuilder {
        let url = format!("{}{}", self.base_url, path);
        self.authorize(self.http.request(method, url))
    }

    pub(crate) async fn send_json<T: DeserializeOwned>(
        &self,
        builder: RequestBuilder,
    ) -> Result<T> {
        let response = check_status(builder.send().await?).await?;
        Ok(response.json().await?)
    }

    pub(crate) async fn send(&self, builder: RequestBuilder) -> Result<reqwest::Response> {
        check_status(builder.send().await?).await
    }
}
