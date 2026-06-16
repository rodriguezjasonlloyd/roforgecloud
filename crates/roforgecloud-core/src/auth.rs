use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use clap::Args;
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;

use crate::error::{Error, Result};
use crate::oauth::{OAuthClient, TokenResponseExt, TokenResponseType};

const SCOPES: &[&str] = &["universe:read"];

pub trait LoginPrompt {
    fn auth_url(&self, _url: &str) {}
    fn browser_open_failed(&self) {}
    fn waiting(&self) {}
    fn success(&self) {}
}

pub struct NoopLoginPrompt;

impl LoginPrompt for NoopLoginPrompt {}

#[derive(Debug, Serialize, Deserialize)]
struct StoredToken {
    access_token: String,
    refresh_token: Option<String>,
    expires_at: Option<u64>,
}

/// Shared OAuth/API-key CLI args for the `roforgecloud` and `roforgecloud-tui` binaries.
#[derive(Args, Clone)]
pub struct OAuthArgs {
    #[arg(long, env = "ROFORGE_API_KEY", hide_env_values = true, global = true)]
    pub api_key: Option<String>,

    #[arg(long, env = "ROFORGE_OAUTH_CLIENT_ID", hide_env_values = true, hide_default_value = true, default_value = crate::oauth::DEFAULT_CLIENT_ID, global = true)]
    pub client_id: String,

    /// For self-registered OAuth apps; bypasses the relay.
    #[arg(
        long,
        env = "ROFORGE_OAUTH_CLIENT_SECRET",
        hide_env_values = true,
        hide_default_value = true,
        global = true
    )]
    pub client_secret: Option<String>,

    /// Relay holding the client secret. Ignored if --client-secret is set.
    #[arg(long, env = "ROFORGE_OAUTH_RELAY_URL", hide_env_values = true, hide_default_value = true, default_value = crate::oauth::DEFAULT_RELAY_URL, global = true)]
    pub relay_url: String,

    #[arg(
        long,
        env = "ROFORGE_OAUTH_REDIRECT_URI",
        hide_env_values = true,
        hide_default_value = true,
        default_value = "http://localhost:8675/callback",
        global = true
    )]
    pub redirect_uri: String,
}

impl OAuthArgs {
    pub fn build_oauth_client(&self) -> Result<OAuthClient> {
        build_oauth_client(
            self.client_id.clone(),
            self.client_secret.clone(),
            &self.relay_url,
            &self.redirect_uri,
        )
    }
}

pub fn build_oauth_client(
    client_id: impl Into<String>,
    client_secret: Option<String>,
    relay_url: &str,
    redirect_uri: &str,
) -> Result<OAuthClient> {
    let has_secret = client_secret.is_some();
    let oauth = OAuthClient::new(client_id, client_secret.unwrap_or_default(), redirect_uri)?;
    let oauth = if !has_secret && !relay_url.is_empty() {
        oauth.with_relay(relay_url)?
    } else {
        oauth
    };
    Ok(oauth)
}

pub async fn access_token(
    oauth: &OAuthClient,
    redirect_uri: &str,
    prompt: &dyn LoginPrompt,
) -> Result<String> {
    if let Some(token) = cached_access_token(oauth).await {
        return Ok(token);
    }

    force_login(oauth, redirect_uri, prompt).await
}

pub async fn cached_access_token(oauth: &OAuthClient) -> Option<String> {
    let cached = load_cached_token()?;
    if !is_expired(&cached) {
        return Some(cached.access_token);
    }

    let refresh_token = cached.refresh_token?;
    let response = oauth.refresh(&refresh_token).await.ok()?;
    let mut stored = stored_token_from_response(&response);
    if stored.refresh_token.is_none() {
        stored.refresh_token = Some(refresh_token);
    }
    save_token(&stored).ok()?;
    Some(stored.access_token)
}

pub fn is_logged_in() -> bool {
    load_cached_token().is_some()
}

pub async fn force_login(
    oauth: &OAuthClient,
    redirect_uri: &str,
    prompt: &dyn LoginPrompt,
) -> Result<String> {
    let response = login(oauth, redirect_uri, prompt).await?;
    let stored = stored_token_from_response(&response);
    save_token(&stored)?;
    Ok(stored.access_token)
}

async fn login(
    oauth: &OAuthClient,
    redirect_uri: &str,
    prompt: &dyn LoginPrompt,
) -> Result<TokenResponseType> {
    let (auth_url, state) = oauth.authorize_url(SCOPES.iter().map(|s| s.to_string()));

    prompt.auth_url(auth_url.as_str());

    if open::that(auth_url.as_str()).is_err() {
        prompt.browser_open_failed();
    }

    prompt.waiting();

    let redirect = url::Url::parse(redirect_uri)?;
    let host = redirect.host_str().unwrap_or("localhost");
    let port = redirect.port().unwrap_or(80);
    let listener = TcpListener::bind((host, port)).await?;

    let (code, returned_state) = loop {
        let (mut stream, _) = listener.accept().await?;
        if let Some(pair) = handle_callback(&mut stream).await? {
            break pair;
        }
    };

    if returned_state != *state.csrf_token.secret() {
        return Err(Error::OAuth("OAuth state mismatch".to_string()));
    }

    let token = oauth.exchange_code(code, state.pkce_verifier).await?;
    prompt.success();
    Ok(token)
}

async fn handle_callback(stream: &mut tokio::net::TcpStream) -> Result<Option<(String, String)>> {
    let mut reader = BufReader::new(&mut *stream);
    let mut request_line = String::new();
    reader.read_line(&mut request_line).await?;

    let path = request_line
        .split_whitespace()
        .nth(1)
        .unwrap_or("/")
        .to_string();

    let body =
        "<html><body>roforgecloud: authorization complete, you can close this tab.</body></html>";
    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
    );
    stream.write_all(response.as_bytes()).await?;
    stream.flush().await?;

    let url = url::Url::parse(&format!("http://localhost{path}"))?;
    let mut code = None;
    let mut state = None;
    for (key, value) in url.query_pairs() {
        match key.as_ref() {
            "code" => code = Some(value.into_owned()),
            "state" => state = Some(value.into_owned()),
            _ => {}
        }
    }

    Ok(match (code, state) {
        (Some(code), Some(state)) => Some((code, state)),
        _ => None,
    })
}

fn stored_token_from_response(token: &TokenResponseType) -> StoredToken {
    let expires_at = token.expires_in().map(|duration| {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
            + duration.as_secs()
    });

    StoredToken {
        access_token: token.access_token().secret().clone(),
        refresh_token: token.refresh_token().map(|t| t.secret().clone()),
        expires_at,
    }
}

fn is_expired(token: &StoredToken) -> bool {
    match token.expires_at {
        Some(expires_at) => {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            now + 60 >= expires_at
        }
        None => false,
    }
}

pub async fn logout(oauth: &OAuthClient) -> Result<()> {
    if let Some(cached) = load_cached_token() {
        if let Some(refresh_token) = &cached.refresh_token {
            let _ = oauth.revoke(refresh_token).await;
        }
    }

    let path = token_cache_path();
    if path.exists() {
        std::fs::remove_file(path)?;
    }

    Ok(())
}

fn token_cache_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".config/roforgecloud/token.json")
}

fn load_cached_token() -> Option<StoredToken> {
    let bytes = std::fs::read(token_cache_path()).ok()?;
    serde_json::from_slice(&bytes).ok()
}

fn save_token(token: &StoredToken) -> Result<()> {
    let path = token_cache_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, serde_json::to_string_pretty(token)?)?;
    Ok(())
}
