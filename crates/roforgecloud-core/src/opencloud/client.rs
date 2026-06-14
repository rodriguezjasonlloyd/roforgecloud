use percent_encoding::{AsciiSet, NON_ALPHANUMERIC};
use reqwest::{Client, Method, RequestBuilder};
use serde::de::DeserializeOwned;

use crate::error::{Error, Result};

const DEFAULT_BASE_URL: &str = "https://apis.roblox.com";

const PATH_SEGMENT: &AsciiSet = &NON_ALPHANUMERIC
    .remove(b'-')
    .remove(b'_')
    .remove(b'.')
    .remove(b'~');

pub(crate) fn encode_path_segment(value: &str) -> String {
    percent_encoding::utf8_percent_encode(value, PATH_SEGMENT).to_string()
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

    pub(crate) fn request(&self, method: Method, path: &str) -> RequestBuilder {
        let url = format!("{}{}", self.base_url, path);
        self.authorize(self.http.request(method, url))
    }

    pub(crate) async fn send_json<T: DeserializeOwned>(
        &self,
        builder: RequestBuilder,
    ) -> Result<T> {
        let response = builder.send().await?;
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(Error::Api { status, body });
        }
        Ok(response.json().await?)
    }

    pub(crate) async fn send(&self, builder: RequestBuilder) -> Result<reqwest::Response> {
        let response = builder.send().await?;
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(Error::Api { status, body });
        }
        Ok(response)
    }
}
