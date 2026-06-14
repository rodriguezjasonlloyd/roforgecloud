use roforgecloud_core::auth;
use roforgecloud_core::oauth::{self, OAuthClient};
use roforgecloud_core::opencloud::datastore::{DataStoreEntryInfo, DataStoreInfo};
use roforgecloud_core::opencloud::OpenCloudClient;

use crate::json_tree::{flatten, JsonNode, JsonNodeValue};
use crate::userlookup;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Menu,
    UniverseChoice,
    UniverseSelect,
    UniverseInput,
    Stores,
    Entries,
    Value,
    Messaging,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    LoadStores,
    LoadEntries,
    LoadNextEntriesPage,
    LoadPrevEntriesPage,
    RefreshEntries,
    LoadValue,
    SaveValue,
    SaveTree,
    EditValueExternal,
    DeleteEntry,
    BulkDeleteEntries,
    DeleteDataStore,
    UndeleteDataStore,
    BulkDeleteDataStores,
    BulkUndeleteDataStores,
    PublishMessage,
    LoadUniverses,
    Login,
    Logout,
}

pub const UNIVERSE_CHOICE_LIST_ALL: usize = 0;
pub const UNIVERSE_CHOICE_ENTER_ID: usize = 1;
pub const UNIVERSE_CHOICE_ITEMS: &[&str] = &["List my universes (OAuth)", "Enter universe ID"];

pub const SERVICE_DATA_STORES: usize = 0;
pub const SERVICE_MESSAGING: usize = 1;
pub const SERVICE_ACCOUNT: usize = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessagingField {
    Topic,
    Message,
}

pub struct App {
    pub client: OpenCloudClient,
    pub has_api_key: bool,
    pub oauth: Option<OAuthClient>,
    pub redirect_uri: String,
    pub logged_in: bool,
    pub universe_id: u64,
    pub universe_input: String,
    pub available_universes: Vec<u64>,
    pub universe_names: std::collections::HashMap<u64, String>,
    pub universe_name_rx: tokio::sync::mpsc::UnboundedReceiver<(u64, String)>,
    pub universe_name_tx: tokio::sync::mpsc::UnboundedSender<(u64, String)>,
    pub universe_choice_selected: usize,
    pub universe_select_selected: usize,
    pub universe_search: String,
    pub universe_search_active: bool,
    pub pending_service: usize,
    pub screen: Screen,
    pub should_quit: bool,
    pub loading: bool,
    pub status: String,

    pub menu_items: Vec<(&'static str, usize)>,
    pub menu_selected: usize,

    pub stores: Vec<DataStoreInfo>,
    pub stores_selected: usize,
    pub stores_delete_pending: bool,
    pub stores_undelete_pending: bool,
    pub stores_show_deleted: bool,
    pub stores_marked: std::collections::HashSet<usize>,

    pub data_store_id: String,
    pub http: reqwest::Client,
    pub entries: Vec<DataStoreEntryInfo>,
    pub usernames: std::collections::HashMap<u64, String>,
    pub username_rx: tokio::sync::mpsc::UnboundedReceiver<std::collections::HashMap<u64, String>>,
    pub username_tx: tokio::sync::mpsc::UnboundedSender<std::collections::HashMap<u64, String>>,
    pub entries_selected: usize,
    pub entries_next_page_token: Option<String>,
    pub entries_page_tokens: Vec<Option<String>>,
    pub entries_delete_pending: bool,
    pub entries_bulk_delete_pending: bool,
    pub entries_marked: std::collections::HashSet<usize>,
    pub entries_search: String,
    pub entries_search_active: bool,
    pub confirm_deadline: Option<std::time::Instant>,

    pub value_title: String,
    pub value_text: String,
    pub value_scroll: u16,
    pub value_viewport_height: u16,
    pub value_edit_text: String,

    pub tree_mode: bool,
    pub value_tree: Option<JsonNode>,
    pub tree_cursor: usize,
    pub tree_editing: bool,
    pub tree_edit_text: String,
    pub tree_dirty: bool,
    pub tree_quit_pending: bool,

    pub messaging_topic: String,
    pub messaging_message: String,
    pub messaging_field: MessagingField,
}

impl App {
    pub fn new(
        client: OpenCloudClient,
        has_api_key: bool,
        oauth: Option<OAuthClient>,
        redirect_uri: String,
        available_universes: Vec<u64>,
        logged_in: bool,
    ) -> Self {
        let menu_items = vec![
            ("Data Stores", SERVICE_DATA_STORES),
            ("Messaging", SERVICE_MESSAGING),
            ("Account", SERVICE_ACCOUNT),
        ];

        let username_channel = tokio::sync::mpsc::unbounded_channel();
        let universe_name_channel = tokio::sync::mpsc::unbounded_channel();

        Self {
            client,
            has_api_key,
            oauth,
            redirect_uri,
            logged_in,
            universe_id: 0,
            universe_input: String::new(),
            available_universes,
            universe_names: std::collections::HashMap::new(),
            universe_name_rx: universe_name_channel.1,
            universe_name_tx: universe_name_channel.0,
            universe_choice_selected: 0,
            universe_search: String::new(),
            universe_search_active: false,
            universe_select_selected: 0,
            pending_service: SERVICE_MESSAGING,
            screen: Screen::Menu,
            should_quit: false,
            loading: false,
            status: String::new(),
            menu_items,
            menu_selected: 0,
            stores: Vec::new(),
            stores_selected: 0,
            stores_delete_pending: false,
            stores_undelete_pending: false,
            stores_show_deleted: false,
            stores_marked: std::collections::HashSet::new(),
            data_store_id: String::new(),
            http: reqwest::Client::new(),
            entries: Vec::new(),
            usernames: std::collections::HashMap::new(),
            username_rx: username_channel.1,
            username_tx: username_channel.0,
            entries_selected: 0,
            entries_next_page_token: None,
            entries_page_tokens: vec![None],
            entries_delete_pending: false,
            entries_bulk_delete_pending: false,
            entries_marked: std::collections::HashSet::new(),
            entries_search: String::new(),
            entries_search_active: false,
            confirm_deadline: None,
            value_title: String::new(),
            value_text: String::new(),
            value_scroll: 0,
            value_viewport_height: 0,
            value_edit_text: String::new(),
            tree_mode: false,
            value_tree: None,
            tree_cursor: 0,
            tree_editing: false,
            tree_edit_text: String::new(),
            tree_dirty: false,
            tree_quit_pending: false,
            messaging_topic: String::new(),
            messaging_message: String::new(),
            messaging_field: MessagingField::Topic,
        }
    }

    pub async fn perform(&mut self, action: Action) {
        match action {
            Action::LoadStores => self.load_stores().await,
            Action::LoadEntries => self.load_entries().await,
            Action::LoadNextEntriesPage => self.load_next_entries_page().await,
            Action::LoadPrevEntriesPage => self.load_prev_entries_page().await,
            Action::RefreshEntries => self.load_entries_page().await,
            Action::LoadValue => self.load_value().await,
            Action::SaveValue => self.save_value().await,
            Action::SaveTree => self.save_tree().await,
            Action::EditValueExternal => {}
            Action::DeleteEntry => self.delete_entry().await,
            Action::BulkDeleteEntries => self.bulk_delete_entries().await,
            Action::DeleteDataStore => self.delete_data_store().await,
            Action::UndeleteDataStore => self.undelete_data_store().await,
            Action::BulkDeleteDataStores => self.bulk_delete_data_stores().await,
            Action::BulkUndeleteDataStores => self.bulk_undelete_data_stores().await,
            Action::PublishMessage => self.publish_message().await,
            Action::LoadUniverses => self.load_universes().await,
            Action::Login => self.login().await,
            Action::Logout => self.logout().await,
        }
    }

    pub async fn login(&mut self) {
        let Some(oauth) = &self.oauth else {
            self.status =
                "OAuth not configured: set ROFORGE_OAUTH_CLIENT_ID/ROFORGE_OAUTH_CLIENT_SECRET"
                    .to_string();
            return;
        };

        self.status = "opening browser for login...".to_string();
        match auth::force_login(oauth, &self.redirect_uri).await {
            Ok(_) => {
                self.logged_in = true;
                self.status = "logged in".to_string();
            }
            Err(err) => self.status = format!("error: {err}"),
        }
    }

    pub async fn logout(&mut self) {
        let Some(oauth) = &self.oauth else {
            self.status =
                "OAuth not configured: set ROFORGE_OAUTH_CLIENT_ID/ROFORGE_OAUTH_CLIENT_SECRET"
                    .to_string();
            return;
        };

        match auth::logout(oauth).await {
            Ok(()) => {
                self.logged_in = false;
                self.status = "logged out".to_string();
            }
            Err(err) => self.status = format!("error: {err}"),
        }
    }

    pub async fn load_universes(&mut self) {
        let Some(oauth) = &self.oauth else {
            self.status =
                "OAuth not configured: set ROFORGE_OAUTH_CLIENT_ID/ROFORGE_OAUTH_CLIENT_SECRET"
                    .to_string();
            return;
        };

        self.status = "fetching authorized universes...".to_string();
        let result = async {
            let token = auth::access_token(oauth, &self.redirect_uri).await?;
            let resources = oauth.token_resources(&token).await?;
            anyhow::Ok(oauth::authorized_universe_ids(&resources))
        }
        .await;

        match result {
            Ok(universes) if universes.is_empty() => {
                self.status = "no authorized universes found for this token".to_string();
            }
            Ok(universes) => {
                self.available_universes = universes;
                self.universe_select_selected = 0;
                self.status.clear();
                self.screen = Screen::UniverseSelect;
                self.resolve_universe_names();
            }
            Err(err) => {
                self.status = format!("error: {err}");
            }
        }
    }

    fn datastore_error(&self, err: roforgecloud_core::error::Error) -> String {
        if !self.has_api_key
            && matches!(&err, roforgecloud_core::error::Error::Api { status, .. } if status.as_u16() == 401)
        {
            "error: Data Stores need an API key — OAuth tokens aren't accepted here. \
             Set ROFORGE_API_KEY and restart."
                .to_string()
        } else {
            format!("error: {err}")
        }
    }

    pub fn max_value_scroll(&self) -> u16 {
        let total_lines = self.value_text.lines().count() as u16;
        total_lines.saturating_sub(self.value_viewport_height)
    }

    pub async fn load_stores(&mut self) {
        self.resolve_current_universe_name();
        self.status = "loading data stores...".to_string();
        match self
            .client
            .list_data_stores(self.universe_id, None, None, self.stores_show_deleted)
            .await
        {
            Ok(result) => {
                self.stores = result.data_stores;
                self.stores_selected = 0;
                self.stores_marked.clear();
                self.status = format!("{} data stores", self.stores.len());
            }
            Err(err) => {
                self.status = self.datastore_error(err);
            }
        }
    }

    pub async fn delete_data_store(&mut self) {
        self.stores_delete_pending = false;
        let Some(store) = self.stores.get(self.stores_selected) else {
            return;
        };

        self.status = "deleting data store...".to_string();
        match self
            .client
            .delete_data_store(self.universe_id, &store.id)
            .await
        {
            Ok(info) => {
                self.status = "data store scheduled for deletion".to_string();
                self.stores[self.stores_selected] = info;
            }
            Err(err) => {
                self.status = self.datastore_error(err);
            }
        }
    }

    pub async fn undelete_data_store(&mut self) {
        let Some(store) = self.stores.get(self.stores_selected) else {
            return;
        };

        self.status = "restoring data store...".to_string();
        match self
            .client
            .undelete_data_store(self.universe_id, &store.id)
            .await
        {
            Ok(info) => {
                self.status = "data store restored".to_string();
                self.stores[self.stores_selected] = info;
            }
            Err(err) => {
                self.status = self.datastore_error(err);
            }
        }
    }

    pub async fn bulk_delete_data_stores(&mut self) {
        self.stores_delete_pending = false;

        let mut indices: Vec<usize> = self.stores_marked.iter().copied().collect();
        indices.sort_unstable();

        let total = indices.len();
        let mut errors = 0;

        for (n, &i) in indices.iter().enumerate() {
            let id = self.stores[i].id.clone();
            self.status = format!("scheduling deletion {}/{total}...", n + 1);
            match self.client.delete_data_store(self.universe_id, &id).await {
                Ok(info) => self.stores[i] = info,
                Err(_) => errors += 1,
            }
        }

        self.stores_marked.clear();
        self.status = if errors == 0 {
            format!("scheduled {total} data stores for deletion")
        } else {
            format!(
                "scheduled {} data stores for deletion, {errors} failed",
                total - errors
            )
        };
    }

    pub async fn bulk_undelete_data_stores(&mut self) {
        self.stores_undelete_pending = false;

        let mut indices: Vec<usize> = self.stores_marked.iter().copied().collect();
        indices.sort_unstable();

        let total = indices.len();
        let mut errors = 0;

        for (n, &i) in indices.iter().enumerate() {
            let id = self.stores[i].id.clone();
            self.status = format!("restoring {}/{total}...", n + 1);
            match self.client.undelete_data_store(self.universe_id, &id).await {
                Ok(info) => self.stores[i] = info,
                Err(_) => errors += 1,
            }
        }

        self.stores_marked.clear();
        self.status = if errors == 0 {
            format!("restored {total} data stores")
        } else {
            format!("restored {} data stores, {errors} failed", total - errors)
        };
    }

    pub async fn load_entries(&mut self) {
        self.entries_page_tokens = vec![None];
        self.load_entries_page().await;
    }

    pub async fn load_next_entries_page(&mut self) {
        let Some(token) = self.entries_next_page_token.clone() else {
            return;
        };
        self.entries_page_tokens.push(Some(token));
        self.load_entries_page().await;
    }

    pub async fn load_prev_entries_page(&mut self) {
        if self.entries_page_tokens.len() <= 1 {
            return;
        }
        self.entries_page_tokens.pop();
        self.load_entries_page().await;
    }

    pub async fn load_entries_page(&mut self) {
        let page_token = self.entries_page_tokens.last().cloned().flatten();

        self.status = "loading entries...".to_string();
        match self
            .client
            .list_entries(
                self.universe_id,
                &self.data_store_id,
                None,
                None,
                page_token.as_deref(),
                Some(256),
            )
            .await
        {
            Ok(result) => {
                self.entries = result.data_store_entries;
                self.entries_selected = 0;
                self.entries_marked.clear();
                self.entries_next_page_token = result.next_page_token;
                let page = self.entries_page_tokens.len();
                self.status = format!(
                    "{} entries (page {page}){}",
                    self.entries.len(),
                    match (page > 1, self.entries_next_page_token.is_some()) {
                        (true, true) => "  p: prev page, n: next page",
                        (true, false) => "  p: prev page",
                        (false, true) => "  n: next page",
                        (false, false) => "",
                    }
                );
                self.resolve_entry_usernames();
            }
            Err(err) => {
                self.status = self.datastore_error(err);
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

    fn resolve_universe_names(&mut self) {
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

    fn resolve_entry_usernames(&mut self) {
        let ids: Vec<u64> = self
            .entries
            .iter()
            .filter_map(|entry| userlookup::extract_id(&entry.id))
            .filter(|id| !self.usernames.contains_key(id))
            .collect();

        if ids.is_empty() {
            return;
        }

        let client = self.http.clone();
        let tx = self.username_tx.clone();
        tokio::spawn(async move {
            if let Ok(resolved) = userlookup::resolve_usernames(&client, &ids).await {
                let _ = tx.send(resolved);
            }
        });
    }

    pub fn visible_universe_indices(&self) -> Vec<usize> {
        if self.universe_search.is_empty() {
            return (0..self.available_universes.len()).collect();
        }

        let needle = self.universe_search.to_lowercase();
        self.available_universes
            .iter()
            .enumerate()
            .filter(|(_, id)| {
                if id.to_string().contains(&needle) {
                    return true;
                }
                self.universe_names
                    .get(id)
                    .is_some_and(|name| name.to_lowercase().contains(&needle))
            })
            .map(|(i, _)| i)
            .collect()
    }

    const CONFIRM_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(3);

    pub fn arm_confirm(&mut self) {
        self.confirm_deadline = Some(std::time::Instant::now() + Self::CONFIRM_TIMEOUT);
    }

    pub fn cancel_pending_confirms(&mut self) {
        self.entries_delete_pending = false;
        self.entries_bulk_delete_pending = false;
        self.stores_delete_pending = false;
        self.stores_undelete_pending = false;
        self.tree_quit_pending = false;
        self.confirm_deadline = None;
    }

    pub fn check_confirm_timeout(&mut self) {
        if self
            .confirm_deadline
            .is_some_and(|deadline| std::time::Instant::now() >= deadline)
        {
            self.cancel_pending_confirms();
            self.status.clear();
        }
    }

    pub fn toggle_entry_mark(&mut self) {
        if let Some(index) = self.current_entry_index() {
            if !self.entries_marked.remove(&index) {
                self.entries_marked.insert(index);
            }
        }
    }

    pub fn toggle_select_all_visible(&mut self) {
        let visible = self.visible_entry_indices();
        if visible.iter().all(|i| self.entries_marked.contains(i)) {
            for i in &visible {
                self.entries_marked.remove(i);
            }
        } else {
            self.entries_marked.extend(visible);
        }
    }

    pub fn toggle_store_mark(&mut self) {
        if !self.stores.is_empty() && !self.stores_marked.remove(&self.stores_selected) {
            self.stores_marked.insert(self.stores_selected);
        }
    }

    pub fn toggle_select_all_stores(&mut self) {
        if self.stores.is_empty() {
            return;
        }
        if self.stores_marked.len() == self.stores.len() {
            self.stores_marked.clear();
        } else {
            self.stores_marked = (0..self.stores.len()).collect();
        }
    }

    pub fn visible_entry_indices(&self) -> Vec<usize> {
        if self.entries_search.is_empty() {
            return (0..self.entries.len()).collect();
        }

        let needle = self.entries_search.to_lowercase();
        self.entries
            .iter()
            .enumerate()
            .filter(|(_, entry)| {
                if entry.id.to_lowercase().contains(&needle) {
                    return true;
                }
                userlookup::extract_id(&entry.id)
                    .and_then(|id| self.usernames.get(&id))
                    .is_some_and(|name| name.to_lowercase().contains(&needle))
            })
            .map(|(i, _)| i)
            .collect()
    }

    fn current_entry_index(&self) -> Option<usize> {
        self.visible_entry_indices()
            .get(self.entries_selected)
            .copied()
    }

    fn current_entry_scope_key(&self) -> Option<(String, String)> {
        let entry = self.entries.get(self.current_entry_index()?)?;
        Some(match entry.id.split_once('/') {
            Some((scope, key)) => (scope.to_string(), key.to_string()),
            None => ("global".to_string(), entry.id.clone()),
        })
    }

    pub async fn load_value(&mut self) {
        let Some((scope, key)) = self.current_entry_scope_key() else {
            return;
        };

        self.status = "loading value...".to_string();
        match self
            .client
            .get_entry(self.universe_id, &self.data_store_id, &key, Some(&scope))
            .await
        {
            Ok(value) => {
                self.value_title = format!("{}/{} (scope: {scope})", self.data_store_id, key);
                self.value_text = serde_json::to_string_pretty(&value).unwrap_or_default();
                self.value_scroll = 0;
                self.tree_mode = false;
                self.value_tree = None;
                self.status.clear();
            }
            Err(err) => {
                self.status = self.datastore_error(err);
            }
        }
    }

    pub fn enter_tree_mode(&mut self) {
        match serde_json::from_str::<serde_json::Value>(&self.value_text) {
            Ok(value) => {
                let mut tree = JsonNode::from_value(&value);
                tree.collapse_below_root();
                self.value_tree = Some(tree);
                self.tree_mode = true;
                self.tree_cursor = 0;
                self.tree_editing = false;
                self.tree_dirty = false;
                self.tree_quit_pending = false;
                self.status.clear();
            }
            Err(err) => {
                self.status = format!("invalid JSON: {err}");
            }
        }
    }

    pub fn exit_tree_mode(&mut self) {
        self.tree_mode = false;
        self.tree_editing = false;
        self.tree_quit_pending = false;
        self.value_tree = None;
        self.status.clear();
    }

    pub fn tree_move(&mut self, delta: isize) {
        let Some(tree) = &self.value_tree else {
            return;
        };
        let len = flatten(tree).len();
        if len == 0 {
            return;
        }
        let cursor = self.tree_cursor as isize + delta;
        self.tree_cursor = cursor.clamp(0, len as isize - 1) as usize;
    }

    pub fn tree_toggle(&mut self) {
        let Some(tree) = &mut self.value_tree else {
            return;
        };
        let rows = flatten(tree);
        let Some(current) = rows.get(self.tree_cursor) else {
            return;
        };

        if !current.is_closing && !current.is_container {
            return;
        }
        let path = current.path.clone();

        if let Some(node) = tree.get_mut(&path) {
            node.collapsed = !node.collapsed;
        }

        if current.is_closing {
            let rows = flatten(self.value_tree.as_ref().unwrap());
            if let Some(idx) = rows
                .iter()
                .position(|r| r.path == path && r.is_container && !r.is_closing)
            {
                self.tree_cursor = idx;
            }
        }
    }

    pub fn tree_edit_leaf(&mut self) {
        let Some(tree) = &self.value_tree else {
            return;
        };
        let rows = flatten(tree);
        let Some(current) = rows.get(self.tree_cursor) else {
            return;
        };

        if current.is_leaf {
            self.tree_edit_text = current.preview.clone();
            self.tree_editing = true;
            self.status = "editing value — enter: confirm, esc: cancel".to_string();
        }
    }

    pub fn tree_confirm_edit(&mut self) {
        let Some(tree) = &mut self.value_tree else {
            return;
        };
        let rows = flatten(tree);
        let Some(current) = rows.get(self.tree_cursor) else {
            self.tree_editing = false;
            return;
        };

        let value = serde_json::from_str::<serde_json::Value>(&self.tree_edit_text)
            .unwrap_or_else(|_| serde_json::Value::String(self.tree_edit_text.clone()));

        if let Some(node) = tree.get_mut(&current.path) {
            node.value = JsonNodeValue::Leaf(value);
        }
        self.tree_editing = false;
        self.tree_edit_text.clear();
        self.tree_dirty = true;
        self.status.clear();
    }

    pub fn tree_cancel_edit(&mut self) {
        self.tree_editing = false;
        self.tree_edit_text.clear();
        self.status.clear();
    }

    pub async fn save_tree(&mut self) {
        let Some(tree) = &self.value_tree else {
            return;
        };
        self.value_edit_text = serde_json::to_string_pretty(&tree.to_value()).unwrap_or_default();
        self.save_value().await;
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(&self.value_text) {
            self.value_tree = Some(JsonNode::from_value(&value));
            self.tree_dirty = false;
            self.tree_quit_pending = false;
        }
    }

    pub async fn save_value(&mut self) {
        let Some((scope, key)) = self.current_entry_scope_key() else {
            return;
        };

        let value: serde_json::Value = match serde_json::from_str(&self.value_edit_text) {
            Ok(value) => value,
            Err(err) => {
                self.status = format!("invalid JSON: {err}");
                return;
            }
        };

        self.status = "saving...".to_string();
        match self
            .client
            .set_entry(
                self.universe_id,
                &self.data_store_id,
                &key,
                Some(&scope),
                &value,
            )
            .await
        {
            Ok(()) => {
                self.value_text = serde_json::to_string_pretty(&value).unwrap_or_default();
                self.value_scroll = 0;
                self.status = "saved".to_string();
            }
            Err(err) => {
                self.status = self.datastore_error(err);
            }
        }
    }

    pub async fn delete_entry(&mut self) {
        self.entries_delete_pending = false;
        let Some(index) = self.current_entry_index() else {
            return;
        };
        let Some((scope, key)) = self.current_entry_scope_key() else {
            return;
        };

        self.status = "deleting...".to_string();
        match self
            .client
            .delete_entry(self.universe_id, &self.data_store_id, &key, Some(&scope))
            .await
        {
            Ok(()) => {
                self.entries.remove(index);
                let visible = self.visible_entry_indices().len();
                if self.entries_selected >= visible {
                    self.entries_selected = visible.saturating_sub(1);
                }
                if self.screen == Screen::Value {
                    self.exit_tree_mode();
                    self.screen = Screen::Entries;
                }
                self.status = "deleted".to_string();
            }
            Err(err) => {
                self.status = self.datastore_error(err);
            }
        }
    }

    pub async fn bulk_delete_entries(&mut self) {
        self.entries_bulk_delete_pending = false;

        let mut indices: Vec<usize> = self.entries_marked.iter().copied().collect();
        indices.sort_unstable();

        let targets: Vec<(usize, String, String)> = indices
            .into_iter()
            .map(|i| {
                let entry = &self.entries[i];
                let (scope, key) = match entry.id.split_once('/') {
                    Some((scope, key)) => (scope.to_string(), key.to_string()),
                    None => ("global".to_string(), entry.id.clone()),
                };
                (i, scope, key)
            })
            .collect();

        if targets.is_empty() {
            self.status = "no entries to delete".to_string();
            return;
        }

        let total = targets.len();
        let mut deleted_indices = Vec::new();
        let mut errors = 0;

        for (i, scope, key) in &targets {
            self.status = format!("deleting {}/{total}...", deleted_indices.len() + errors + 1);
            match self
                .client
                .delete_entry(self.universe_id, &self.data_store_id, key, Some(scope))
                .await
            {
                Ok(()) => deleted_indices.push(*i),
                Err(_) => errors += 1,
            }
        }

        for &i in deleted_indices.iter().rev() {
            self.entries.remove(i);
        }

        self.entries_marked.clear();

        let visible = self.visible_entry_indices().len();
        if self.entries_selected >= visible {
            self.entries_selected = visible.saturating_sub(1);
        }

        self.status = if errors == 0 {
            format!("deleted {} entries", deleted_indices.len())
        } else {
            format!("deleted {} entries, {errors} failed", deleted_indices.len())
        };
    }

    pub async fn publish_message(&mut self) {
        if self.messaging_topic.is_empty() {
            self.status = "topic cannot be empty".to_string();
            return;
        }

        self.status = "publishing...".to_string();
        match self
            .client
            .publish_message(
                self.universe_id,
                &self.messaging_topic,
                &self.messaging_message,
            )
            .await
        {
            Ok(()) => {
                self.status = format!("published to '{}'", self.messaging_topic);
            }
            Err(err) => {
                self.status = format!("error: {err}");
            }
        }
    }
}
