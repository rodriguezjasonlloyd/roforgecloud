use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use roforgecloud_core::oauth::{OAuthClient, TokenResponseExt, TokenResponseType};
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;

const SCOPES: &[&str] = &["universe:read", "universe-messaging-service:publish"];

#[derive(Debug, Serialize, Deserialize)]
struct StoredToken {
    access_token: String,
    refresh_token: Option<String>,
    expires_at: Option<u64>,
}

pub async fn access_token(oauth: &OAuthClient, redirect_uri: &str) -> anyhow::Result<String> {
    if let Some(cached) = load_cached_token() {
        if !is_expired(&cached) {
            return Ok(cached.access_token);
        }

        if let Some(refresh_token) = &cached.refresh_token {
            if let Ok(response) = oauth.refresh(refresh_token).await {
                let mut stored = stored_token_from_response(&response);
                if stored.refresh_token.is_none() {
                    stored.refresh_token = Some(refresh_token.clone());
                }
                save_token(&stored)?;
                return Ok(stored.access_token);
            }
        }
    }

    let response = login(oauth, redirect_uri).await?;
    let stored = stored_token_from_response(&response);
    save_token(&stored)?;
    Ok(stored.access_token)
}

async fn login(oauth: &OAuthClient, redirect_uri: &str) -> anyhow::Result<TokenResponseType> {
    let (auth_url, state) = oauth.authorize_url(SCOPES.iter().map(|s| s.to_string()));

    println!("Open this URL in your browser to authorize roforgecloud:\n");
    println!("\x1b]8;;{auth_url}\x1b\\{auth_url}\x1b]8;;\x1b\\\n");

    if open::that(auth_url.as_str()).is_err() {
        println!("(couldn't open a browser automatically, open the link above manually)\n");
    }

    println!("Waiting for authorization...");

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
        anyhow::bail!("OAuth state mismatch");
    }

    let token = oauth.exchange_code(code, state.pkce_verifier).await?;
    println!("Authorization successful.\n");
    Ok(token)
}

async fn handle_callback(
    stream: &mut tokio::net::TcpStream,
) -> anyhow::Result<Option<(String, String)>> {
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

/// Revokes the cached refresh token (if any) and removes the local cache,
/// forcing a fresh login on the next `access_token` call.
pub async fn logout(oauth: &OAuthClient) -> anyhow::Result<()> {
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

fn save_token(token: &StoredToken) -> anyhow::Result<()> {
    let path = token_cache_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, serde_json::to_string_pretty(token)?)?;
    Ok(())
}
