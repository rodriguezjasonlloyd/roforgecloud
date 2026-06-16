use roforgecloud_core::auth;
use roforgecloud_core::oauth;

use crate::app::{App, Screen};
use crate::status;

impl App {
    pub async fn login(&mut self) {
        let Some(oauth) = &self.oauth else {
            self.status = status::OAUTH_NOT_CONFIGURED.to_string();
            return;
        };

        self.status = status::OPENING_BROWSER.to_string();
        match auth::force_login(oauth, &self.redirect_uri, &auth::NoopLoginPrompt).await {
            Ok(_) => {
                self.logged_in = true;
                self.status = status::LOGGED_IN.to_string();
            }
            Err(err) => self.status = status::api_error(err),
        }
    }

    pub async fn logout(&mut self) {
        let Some(oauth) = &self.oauth else {
            self.status = status::OAUTH_NOT_CONFIGURED.to_string();
            return;
        };

        match auth::logout(oauth).await {
            Ok(()) => {
                self.logged_in = false;
                self.status = status::LOGGED_OUT.to_string();
            }
            Err(err) => self.status = status::api_error(err),
        }
    }

    pub async fn load_universes(&mut self) {
        let Some(oauth) = &self.oauth else {
            self.status = status::OAUTH_NOT_CONFIGURED.to_string();
            return;
        };

        self.status = status::FETCHING.to_string();
        let result = async {
            let token = auth::access_token(oauth, &self.redirect_uri, &auth::NoopLoginPrompt).await?;
            let resources = oauth.token_resources(&token).await?;
            anyhow::Ok(oauth::authorized_universe_ids(&resources))
        }
        .await;

        match result {
            Ok(universes) if universes.is_empty() => {
                self.status = status::NO_UNIVERSES.to_string();
            }
            Ok(universes) => {
                self.available_universes = universes;
                self.universe_select.selected = 0;
                self.status.clear();
                self.screen = Screen::UniverseSelect;
                self.resolve_universe_names();
            }
            Err(err) => {
                self.status = status::api_error(err);
            }
        }
    }

    pub fn resolve_current_universe_name(&mut self) {
        if self.universe_names.contains_key(&self.universe_id) {
            return;
        }

        let client = self.client.clone();
        let tx = self.universe_name_tx.clone();
        let universe_id = self.universe_id;
        tokio::spawn(async move {
            if let Ok(info) = client.get_universe(universe_id).await {
                let _ = tx.send((universe_id, info.display_name));
            }
        });
    }

    pub(crate) fn resolve_universe_names(&mut self) {
        for &universe_id in &self.available_universes {
            if self.universe_names.contains_key(&universe_id) {
                continue;
            }

            let client = self.client.clone();
            let tx = self.universe_name_tx.clone();
            tokio::spawn(async move {
                if let Ok(info) = client.get_universe(universe_id).await {
                    let _ = tx.send((universe_id, info.display_name));
                }
            });
        }
    }
}
