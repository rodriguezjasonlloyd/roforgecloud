use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("http request failed: {0}")]
    Http(#[from] reqwest::Error),

    #[error("api returned error status {status}: {body}")]
    Api {
        status: reqwest::StatusCode,
        body: String,
    },

    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("url error: {0}")]
    Url(#[from] url::ParseError),

    #[error("oauth error: {0}")]
    OAuth(String),
}

pub type Result<T> = std::result::Result<T, Error>;
