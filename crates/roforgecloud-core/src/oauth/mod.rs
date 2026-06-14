use oauth2::basic::{BasicClient, BasicTokenResponse};
use oauth2::{
    AuthType, AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken, EndpointNotSet,
    EndpointSet, PkceCodeChallenge, PkceCodeVerifier, RedirectUrl, RefreshToken, Scope, TokenUrl,
};
use serde::Deserialize;
use url::Url;

use crate::error::{Error, Result};

const AUTH_URL: &str = "https://apis.roblox.com/oauth/v1/authorize";
const TOKEN_URL: &str = "https://apis.roblox.com/oauth/v1/token";
const RESOURCES_URL: &str = "https://apis.roblox.com/oauth/v1/token/resources";
const REVOKE_URL: &str = "https://apis.roblox.com/oauth/v1/token/revoke";

/// Default base URL for the roforgecloud OAuth relay (a Cloudflare Worker
/// that injects the client_id/client_secret, see `worker/`). Overridable so
/// users can point at their own relay deployment, or set to empty to talk to
/// Roblox directly (requires a client secret).
pub const DEFAULT_RELAY_URL: &str = "https://roforgecloud-oauth-relay.amaterxsu.workers.dev";

type RobloxOAuthClient =
    BasicClient<EndpointSet, EndpointNotSet, EndpointNotSet, EndpointNotSet, EndpointSet>;

pub struct AuthorizationState {
    pub pkce_verifier: PkceCodeVerifier,
    pub csrf_token: CsrfToken,
}

#[derive(Debug, Deserialize)]
pub struct TokenResources {
    pub resource_infos: Vec<ResourceInfo>,
}

#[derive(Debug, Deserialize)]
pub struct ResourceInfo {
    pub owner: ResourceOwner,
    pub resources: Resources,
}

#[derive(Debug, Deserialize)]
pub struct ResourceOwner {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: String,
}

#[derive(Debug, Deserialize, Default)]
pub struct Resources {
    pub universe: Option<UniverseIds>,
}

#[derive(Debug, Deserialize)]
pub struct UniverseIds {
    pub ids: Vec<String>,
}

pub fn authorized_universe_ids(resources: &TokenResources) -> Vec<u64> {
    let mut ids: Vec<u64> = resources
        .resource_infos
        .iter()
        .filter_map(|info| info.resources.universe.as_ref())
        .flat_map(|universe| universe.ids.iter())
        .filter_map(|id| id.parse::<u64>().ok())
        .collect();
    ids.sort_unstable();
    ids.dedup();
    ids
}

pub struct OAuthClient {
    client: RobloxOAuthClient,
    http: reqwest::Client,
    client_id: String,
    client_secret: String,
    resources_url: String,
    revoke_url: String,
}

impl OAuthClient {
    pub fn new(
        client_id: impl Into<String>,
        client_secret: impl Into<String>,
        redirect_url: impl AsRef<str>,
    ) -> Result<Self> {
        let client_id = client_id.into();
        let client_secret = client_secret.into();

        let client = BasicClient::new(ClientId::new(client_id.clone()))
            .set_client_secret(ClientSecret::new(client_secret.clone()))
            .set_auth_uri(
                AuthUrl::new(AUTH_URL.to_string()).map_err(|e| Error::OAuth(e.to_string()))?,
            )
            .set_token_uri(
                TokenUrl::new(TOKEN_URL.to_string()).map_err(|e| Error::OAuth(e.to_string()))?,
            )
            .set_redirect_uri(
                RedirectUrl::new(redirect_url.as_ref().to_string())
                    .map_err(|e| Error::OAuth(e.to_string()))?,
            )
            .set_auth_type(AuthType::RequestBody);

        Ok(Self {
            client,
            http: reqwest::Client::new(),
            client_id,
            client_secret,
            resources_url: RESOURCES_URL.to_string(),
            revoke_url: REVOKE_URL.to_string(),
        })
    }

    /// Route token exchange/refresh and the `token/resources` lookup through
    /// a relay (see `worker/`) that injects its own client_id/client_secret.
    /// This lets the CLI operate without holding a real client secret.
    pub fn with_relay(mut self, relay_url: &str) -> Result<Self> {
        let relay_url = relay_url.trim_end_matches('/');
        self.client = self.client.set_token_uri(
            TokenUrl::new(format!("{relay_url}/oauth/v1/token"))
                .map_err(|e| Error::OAuth(e.to_string()))?,
        );
        self.resources_url = format!("{relay_url}/oauth/v1/token/resources");
        self.revoke_url = format!("{relay_url}/oauth/v1/token/revoke");
        Ok(self)
    }

    pub fn authorize_url(
        &self,
        scopes: impl IntoIterator<Item = String>,
    ) -> (Url, AuthorizationState) {
        let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();

        let mut request = self
            .client
            .authorize_url(CsrfToken::new_random)
            .set_pkce_challenge(pkce_challenge);
        for scope in scopes {
            request = request.add_scope(Scope::new(scope));
        }
        let (url, csrf_token) = request.url();

        (
            url,
            AuthorizationState {
                pkce_verifier,
                csrf_token,
            },
        )
    }

    pub async fn exchange_code(
        &self,
        code: String,
        pkce_verifier: PkceCodeVerifier,
    ) -> Result<BasicTokenResponse> {
        self.client
            .exchange_code(AuthorizationCode::new(code))
            .set_pkce_verifier(pkce_verifier)
            .request_async(&self.http)
            .await
            .map_err(|e| Error::OAuth(e.to_string()))
    }

    pub async fn refresh(&self, refresh_token: &str) -> Result<BasicTokenResponse> {
        self.client
            .exchange_refresh_token(&RefreshToken::new(refresh_token.to_string()))
            .request_async(&self.http)
            .await
            .map_err(|e| Error::OAuth(e.to_string()))
    }

    pub async fn token_resources(&self, access_token: &str) -> Result<TokenResources> {
        let response = self
            .http
            .post(&self.resources_url)
            .form(&[
                ("token", access_token),
                ("client_id", self.client_id.as_str()),
                ("client_secret", self.client_secret.as_str()),
            ])
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(Error::Api { status, body });
        }

        Ok(response.json().await?)
    }

    /// Revokes a refresh token, invalidating the whole authorization session
    /// (the paired access token and the refresh token itself).
    pub async fn revoke(&self, refresh_token: &str) -> Result<()> {
        let response = self
            .http
            .post(&self.revoke_url)
            .form(&[
                ("token", refresh_token),
                ("client_id", self.client_id.as_str()),
                ("client_secret", self.client_secret.as_str()),
            ])
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(Error::Api { status, body });
        }

        Ok(())
    }
}

pub use oauth2::basic::BasicTokenResponse as TokenResponseType;
pub use oauth2::TokenResponse as TokenResponseExt;
