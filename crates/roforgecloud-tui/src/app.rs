use roforgecloud_core::auth;
use roforgecloud_core::oauth::{self, OAuthClient};
use roforgecloud_core::opencloud::datastore::{DataStoreEntryInfo, DataStoreInfo};
use roforgecloud_core::opencloud::ordered_datastore::OrderedDataStoreEntry;
use roforgecloud_core::opencloud::OpenCloudClient;

use crate::json_tree::{flatten, JsonNode, JsonNodeValue};
use crate::userlookup;

#[derive(Debug, Clone, Default)]
pub struct TextField {
    pub value: String,
    pub cursor: usize,
}

impl TextField {
    pub fn clear(&mut self) {
        self.value.clear();
        self.cursor = 0;
    }

    pub fn set(&mut self, value: impl Into<String>) {
        self.value = value.into();
        self.cursor = self.value.len();
    }

    pub fn insert(&mut self, c: char) {
        self.value.insert(self.cursor, c);
        self.cursor += c.len_utf8();
    }

    pub fn backspace(&mut self) {
        if self.cursor == 0 {
            return;
        }
        let prev_len = self.value[..self.cursor].chars().last().map_or(0, char::len_utf8);
        let new_cursor = self.cursor - prev_len;
        self.value.remove(new_cursor);
        self.cursor = new_cursor;
    }

    pub fn delete(&mut self) {
        if self.cursor < self.value.len() {
            self.value.remove(self.cursor);
        }
    }

    pub fn left(&mut self) {
        if self.cursor > 0 {
            let prev_len = self.value[..self.cursor].chars().last().map_or(0, char::len_utf8);
            self.cursor -= prev_len;
        }
    }

    pub fn right(&mut self) {
        if self.cursor < self.value.len() {
            let next_len = self.value[self.cursor..].chars().next().map_or(0, char::len_utf8);
            self.cursor += next_len;
        }
    }

    pub fn home(&mut self) {
        self.cursor = 0;
    }

    pub fn end(&mut self) {
        self.cursor = self.value.len();
    }
}

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
    OrderedStoreInput,
    OrderedEntries,
    OrderedValue,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    LoadStores,
    LoadEntries,
    LoadNextEntriesPage,
    LoadPrevEntriesPage,
    RefreshEntries,
    LoadAllEntriesForSearch,
    LoadValue,
    RefreshTree,
    SaveValue,
    SaveTree,
    EditValueExternal,
    EditTreeValueExternal,
    CreateEntry,
    CreateEntryExternal,
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
    LoadOrderedEntries,
    LoadNextOrderedEntriesPage,
    LoadPrevOrderedEntriesPage,
    RefreshOrderedEntries,
    LoadAllOrderedEntriesForSearch,
    CreateOrderedEntry,
    CreateOrderedEntryExternal,
    LoadOrderedValue,
    SaveOrderedValue,
    DeleteOrderedEntry,
    BulkDeleteOrderedEntries,
    IncrementOrderedEntry,
}

pub const UNIVERSE_CHOICE_LIST_ALL: usize = 0;
pub const UNIVERSE_CHOICE_ENTER_ID: usize = 1;
pub const UNIVERSE_CHOICE_ITEMS: &[&str] = &["List my universes (OAuth)", "Enter universe ID"];

pub const SERVICE_DATA_STORES: usize = 0;
pub const SERVICE_ORDERED_DATA_STORES: usize = 1;
pub const SERVICE_MESSAGING: usize = 2;
pub const SERVICE_ACCOUNT: usize = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessagingField {
    Topic,
    Message,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntriesCreateField {
    Id,
    Value,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrderedInputField {
    StoreId,
    Scope,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrderedCreateField {
    Id,
    Value,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PendingConfirm {
    DeleteStore,
    BulkDeleteStores,
    BulkUndeleteStores,
    DeleteEntry,
    BulkDeleteEntries,
    DeleteOrderedEntry,
    BulkDeleteOrderedEntries,
    Quit,
    TreeQuit,
    TreeRefresh,
}

impl PendingConfirm {
    pub fn footer_hint(&self) -> &'static str {
        match self {
            PendingConfirm::DeleteStore
            | PendingConfirm::BulkDeleteStores
            | PendingConfirm::DeleteEntry
            | PendingConfirm::BulkDeleteEntries
            | PendingConfirm::DeleteOrderedEntry
            | PendingConfirm::BulkDeleteOrderedEntries => "d: confirm delete   any other key: cancel",
            PendingConfirm::BulkUndeleteStores => "u: confirm undelete   any other key: cancel",
            PendingConfirm::Quit => "q: confirm quit   any other key: cancel",
            PendingConfirm::TreeQuit => {
                "q/esc: discard changes and exit tree   any other key: cancel"
            }
            PendingConfirm::TreeRefresh => {
                "r: discard changes and refresh   any other key: cancel"
            }
        }
    }
}

pub struct App {
    pub client: OpenCloudClient,
    pub has_api_key: bool,
    pub oauth: Option<OAuthClient>,
    pub redirect_uri: String,
    pub logged_in: bool,
    pub universe_id: u64,
    pub universe_input: TextField,
    pub available_universes: Vec<u64>,
    pub universe_names: std::collections::HashMap<u64, String>,
    pub universe_name_rx: tokio::sync::mpsc::UnboundedReceiver<(u64, String)>,
    pub universe_name_tx: tokio::sync::mpsc::UnboundedSender<(u64, String)>,
    pub universe_choice_selected: usize,
    pub universe_select_selected: usize,
    pub universe_search: TextField,
    pub universe_search_active: bool,
    pub pending_service: usize,
    pub screen: Screen,
    pub should_quit: bool,
    pub loading: bool,
    pub status: String,
    pub show_help: bool,

    pub menu_items: Vec<(&'static str, usize)>,
    pub menu_selected: usize,

    pub stores: Vec<DataStoreInfo>,
    pub stores_selected: usize,
    pub stores_marked: std::collections::HashSet<usize>,
    pub stores_new_id: TextField,
    pub stores_new_active: bool,

    pub data_store_id: String,
    pub http: reqwest::Client,
    pub entries: Vec<DataStoreEntryInfo>,
    pub usernames: std::collections::HashMap<u64, String>,
    pub username_rx: tokio::sync::mpsc::UnboundedReceiver<std::collections::HashMap<u64, String>>,
    pub username_tx: tokio::sync::mpsc::UnboundedSender<std::collections::HashMap<u64, String>>,
    pub entries_selected: usize,
    pub entries_next_page_token: Option<String>,
    pub entries_page_tokens: Vec<Option<String>>,
    pub entries_marked: std::collections::HashSet<usize>,
    pub entries_search: TextField,
    pub entries_search_active: bool,
    pub entries_create_id: TextField,
    pub entries_create_value: TextField,
    pub entries_create_field: EntriesCreateField,
    pub entries_create_active: bool,
    pub entries_create_choosing: bool,
    pub pending_confirm: Option<PendingConfirm>,
    pub confirm_deadline: Option<std::time::Instant>,

    pub value_title: String,
    pub value_text: String,
    pub value_revision: Option<String>,
    pub value_scroll: u16,
    pub value_viewport_height: u16,
    pub value_edit_text: String,

    pub tree_mode: bool,
    pub value_tree: Option<JsonNode>,
    pub tree_cursor: usize,
    pub tree_editing: bool,
    pub tree_edit_text: TextField,
    pub tree_editing_key: bool,
    pub tree_edit_key: TextField,
    pub tree_adding: bool,
    pub tree_pending_leader: bool,
    pub tree_dirty: bool,

    pub messaging_topic: TextField,
    pub messaging_message: TextField,
    pub messaging_field: MessagingField,

    pub ordered_data_store_id: TextField,
    pub ordered_scope: TextField,
    pub ordered_input_field: OrderedInputField,

    pub ordered_entries: Vec<OrderedDataStoreEntry>,
    pub ordered_entries_selected: usize,
    pub ordered_entries_next_page_token: Option<String>,
    pub ordered_entries_page_tokens: Vec<Option<String>>,
    pub ordered_entries_marked: std::collections::HashSet<usize>,
    pub ordered_entries_search: TextField,
    pub ordered_entries_search_active: bool,

    pub ordered_value_title: String,
    pub ordered_value: f64,
    pub ordered_value_edit: String,
    pub ordered_value_editing: bool,
    pub ordered_increment_edit: String,
    pub ordered_increment_editing: bool,

    pub ordered_create_id: TextField,
    pub ordered_create_value: TextField,
    pub ordered_create_field: OrderedCreateField,
    pub ordered_create_active: bool,
    pub ordered_create_choosing: bool,
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
            ("Ordered Data Stores", SERVICE_ORDERED_DATA_STORES),
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
            universe_input: TextField::default(),
            available_universes,
            universe_names: std::collections::HashMap::new(),
            universe_name_rx: universe_name_channel.1,
            universe_name_tx: universe_name_channel.0,
            universe_choice_selected: 0,
            universe_search: TextField::default(),
            universe_search_active: false,
            universe_select_selected: 0,
            pending_service: SERVICE_MESSAGING,
            screen: Screen::Menu,
            should_quit: false,
            loading: false,
            status: String::new(),
            show_help: false,
            menu_items,
            menu_selected: 0,
            stores: Vec::new(),
            stores_selected: 0,
            stores_marked: std::collections::HashSet::new(),
            stores_new_id: TextField::default(),
            stores_new_active: false,
            data_store_id: String::new(),
            http: reqwest::Client::new(),
            entries: Vec::new(),
            usernames: std::collections::HashMap::new(),
            username_rx: username_channel.1,
            username_tx: username_channel.0,
            entries_selected: 0,
            entries_next_page_token: None,
            entries_page_tokens: vec![None],
            entries_marked: std::collections::HashSet::new(),
            entries_search: TextField::default(),
            entries_search_active: false,
            entries_create_id: TextField::default(),
            entries_create_value: TextField::default(),
            entries_create_field: EntriesCreateField::Id,
            entries_create_active: false,
            entries_create_choosing: false,
            pending_confirm: None,
            confirm_deadline: None,
            value_title: String::new(),
            value_text: String::new(),
            value_revision: None,
            value_scroll: 0,
            value_viewport_height: 0,
            value_edit_text: String::new(),
            tree_mode: false,
            value_tree: None,
            tree_cursor: 0,
            tree_editing: false,
            tree_edit_text: TextField::default(),
            tree_editing_key: false,
            tree_edit_key: TextField::default(),
            tree_adding: false,
            tree_pending_leader: false,
            tree_dirty: false,
            messaging_topic: TextField::default(),
            messaging_message: TextField::default(),
            messaging_field: MessagingField::Topic,
            ordered_data_store_id: TextField::default(),
            ordered_scope: TextField {
                value: "global".to_string(),
                cursor: "global".len(),
            },
            ordered_input_field: OrderedInputField::StoreId,
            ordered_entries: Vec::new(),
            ordered_entries_selected: 0,
            ordered_entries_next_page_token: None,
            ordered_entries_page_tokens: vec![None],
            ordered_entries_marked: std::collections::HashSet::new(),
            ordered_entries_search: TextField::default(),
            ordered_entries_search_active: false,
            ordered_value_title: String::new(),
            ordered_value: 0.0,
            ordered_value_edit: String::new(),
            ordered_value_editing: false,
            ordered_increment_edit: String::new(),
            ordered_increment_editing: false,
            ordered_create_id: TextField::default(),
            ordered_create_value: TextField::default(),
            ordered_create_field: OrderedCreateField::Id,
            ordered_create_active: false,
            ordered_create_choosing: false,
        }
    }

    pub async fn perform(&mut self, action: Action) {
        match action {
            Action::LoadStores => self.load_stores().await,
            Action::LoadEntries => self.load_entries().await,
            Action::LoadNextEntriesPage => self.load_next_entries_page().await,
            Action::LoadPrevEntriesPage => self.load_prev_entries_page().await,
            Action::RefreshEntries => self.load_entries_page().await,
            Action::LoadAllEntriesForSearch => self.load_all_entries_for_search().await,
            Action::LoadValue => self.load_value().await,
            Action::RefreshTree => self.refresh_tree().await,
            Action::SaveValue => self.save_value().await,
            Action::SaveTree => self.save_tree().await,
            Action::EditValueExternal => {}
            Action::EditTreeValueExternal => {}
            Action::CreateEntry => self.create_entry().await,
            Action::CreateEntryExternal => {}
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
            Action::LoadOrderedEntries => self.load_ordered_entries().await,
            Action::LoadNextOrderedEntriesPage => self.load_next_ordered_entries_page().await,
            Action::LoadPrevOrderedEntriesPage => self.load_prev_ordered_entries_page().await,
            Action::RefreshOrderedEntries => self.load_ordered_entries_page().await,
            Action::LoadAllOrderedEntriesForSearch => self.load_all_ordered_entries_for_search().await,
            Action::CreateOrderedEntry => self.create_ordered_entry().await,
            Action::CreateOrderedEntryExternal => {}
            Action::LoadOrderedValue => self.load_ordered_value().await,
            Action::SaveOrderedValue => self.save_ordered_value().await,
            Action::DeleteOrderedEntry => self.delete_ordered_entry().await,
            Action::BulkDeleteOrderedEntries => self.bulk_delete_ordered_entries().await,
            Action::IncrementOrderedEntry => self.increment_ordered_entry().await,
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
            .list_data_stores(self.universe_id, None, None, true)
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
                self.status = format!("{} entries (page {page})", self.entries.len());
                self.resolve_entry_usernames();
            }
            Err(err) => {
                self.status = self.datastore_error(err);
            }
        }
    }

    pub async fn load_all_entries_for_search(&mut self) {
        self.status = "loading all entries for search...".to_string();
        let mut all = Vec::new();
        let mut page_token: Option<String> = None;
        loop {
            match self
                .client
                .list_entries(self.universe_id, &self.data_store_id, None, None, page_token.as_deref(), Some(256))
                .await
            {
                Ok(result) => {
                    all.extend(result.data_store_entries);
                    page_token = result.next_page_token.filter(|t| !t.is_empty());
                    if page_token.is_none() {
                        break;
                    }
                }
                Err(err) => {
                    self.status = self.datastore_error(err);
                    return;
                }
            }
        }
        self.entries = all;
        self.entries_selected = 0;
        self.entries_marked.clear();
        self.entries_next_page_token = None;
        self.entries_page_tokens = vec![None];
        self.status = format!("{} entries (search across whole store)", self.entries.len());
        self.resolve_entry_usernames();
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
        if self.universe_search.value.is_empty() {
            return (0..self.available_universes.len()).collect();
        }

        let needle = self.universe_search.value.to_lowercase();
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

    pub fn arm_confirm(&mut self, pending: PendingConfirm) {
        self.pending_confirm = Some(pending);
        self.confirm_deadline = Some(std::time::Instant::now() + Self::CONFIRM_TIMEOUT);
        self.status.clear();
    }

    pub fn cancel_pending_confirms(&mut self) {
        self.pending_confirm = None;
        self.confirm_deadline = None;
    }

    pub fn needs_quit_confirm(&self) -> bool {
        !self.stores_marked.is_empty()
            || !self.entries_marked.is_empty()
            || !self.ordered_entries_marked.is_empty()
    }

    pub fn text_input_active(&self) -> bool {
        match self.screen {
            Screen::UniverseInput | Screen::Messaging | Screen::OrderedStoreInput => true,
            Screen::UniverseSelect => self.universe_search_active,
            Screen::Stores => self.stores_new_active,
            Screen::Entries => {
                self.entries_search_active || self.entries_create_active || self.entries_create_choosing
            }
            Screen::Value => self.tree_mode && (self.tree_editing || self.tree_editing_key),
            Screen::OrderedEntries => {
                self.ordered_entries_search_active
                    || self.ordered_create_active
                    || self.ordered_create_choosing
            }
            Screen::OrderedValue => self.ordered_value_editing || self.ordered_increment_editing,
            _ => false,
        }
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
        if self.entries_search.value.is_empty() {
            return (0..self.entries.len()).collect();
        }

        let needle = self.entries_search.value.to_lowercase();
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
            .get_entry_with_revision(self.universe_id, &self.data_store_id, &key, Some(&scope))
            .await
        {
            Ok((value, revision)) => {
                self.value_title = format!("{}/{} (scope: {scope})", self.data_store_id, key);
                self.value_text = serde_json::to_string_pretty(&value).unwrap_or_default();
                self.value_revision = revision;
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
                self.pending_confirm = None;
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
        self.pending_confirm = None;
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
            self.tree_edit_text.set(current.preview.clone());
            self.tree_editing = true;
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

        let value = serde_json::from_str::<serde_json::Value>(&self.tree_edit_text.value)
            .unwrap_or_else(|_| serde_json::Value::String(self.tree_edit_text.value.clone()));

        if let Some(node) = tree.get_mut(&current.path) {
            node.value = JsonNodeValue::Leaf(value);
        }
        self.tree_editing = false;
        self.tree_edit_text.clear();
        self.tree_dirty = true;
        self.tree_adding = false;
    }

    pub fn tree_current_value(&self) -> Option<serde_json::Value> {
        let tree = self.value_tree.as_ref()?;
        let rows = flatten(tree);
        let current = rows.get(self.tree_cursor)?;
        Some(tree.get(&current.path)?.to_value())
    }

    pub fn tree_replace_value(&mut self, key: Option<String>, value: serde_json::Value) {
        let Some(tree) = &mut self.value_tree else {
            return;
        };
        let rows = flatten(tree);
        let Some(current) = rows.get(self.tree_cursor) else {
            return;
        };
        let path = current.path.clone();
        if let Some(node) = tree.get_mut(&path) {
            let mut new_node = JsonNode::from_value(&value);
            new_node.collapse_below_root();
            new_node.key = key.or_else(|| node.key.clone());
            *node = new_node;
        }
        self.tree_dirty = true;
        let len = flatten(self.value_tree.as_ref().unwrap()).len();
        if self.tree_cursor >= len {
            self.tree_cursor = len.saturating_sub(1);
        }
    }

    pub fn tree_cancel_edit(&mut self) {
        self.tree_editing = false;
        self.tree_editing_key = false;
        self.tree_edit_text.clear();
        self.tree_edit_key.clear();
        if self.tree_adding {
            self.tree_remove_current();
            self.tree_adding = false;
        }
        self.status.clear();
    }

    pub fn tree_add_entry(&mut self) {
        let Some(tree) = &mut self.value_tree else {
            return;
        };
        let rows = flatten(tree);
        let Some(current) = rows.get(self.tree_cursor) else {
            return;
        };

        let expanded_container = current.is_container
            && !current.is_closing
            && !tree.get_mut(&current.path).is_some_and(|n| n.collapsed);

        let (parent_path, insert_index) = if expanded_container {
            (current.path.clone(), 0)
        } else if current.path.is_empty() {
            (Vec::new(), 0)
        } else {
            let mut parent_path = current.path.clone();
            let idx = parent_path.pop().unwrap();
            (parent_path, idx + 1)
        };

        let Some(parent) = tree.get_mut(&parent_path) else {
            return;
        };
        parent.collapsed = false;
        let is_object = matches!(parent.value, JsonNodeValue::Object(_));
        let items = match &mut parent.value {
            JsonNodeValue::Array(items) | JsonNodeValue::Object(items) => items,
            JsonNodeValue::Leaf(_) => return,
        };
        let insert_index = insert_index.min(items.len());

        if is_object {
            items.insert(
                insert_index,
                JsonNode {
                    key: Some(String::new()),
                    value: JsonNodeValue::Leaf(serde_json::Value::Null),
                    collapsed: false,
                },
            );
            self.tree_edit_key.clear();
            self.tree_editing_key = true;
        } else {
            items.insert(
                insert_index,
                JsonNode {
                    key: None,
                    value: JsonNodeValue::Leaf(serde_json::Value::String(String::new())),
                    collapsed: false,
                },
            );
            self.tree_edit_text.clear();
            self.tree_editing = true;
        }

        self.tree_adding = true;
        self.tree_dirty = true;

        let mut new_path = parent_path;
        new_path.push(insert_index);
        let rows = flatten(tree);
        if let Some(idx) = rows.iter().position(|r| r.path == new_path && !r.is_closing) {
            self.tree_cursor = idx;
        }
    }

    pub fn tree_edit_key_start(&mut self) {
        let Some(tree) = &self.value_tree else {
            return;
        };
        let rows = flatten(tree);
        let Some(current) = rows.get(self.tree_cursor) else {
            return;
        };
        let Some(key) = &current.key else {
            return;
        };
        self.tree_edit_key.set(key.clone());
        self.tree_editing_key = true;
    }

    pub fn tree_confirm_key(&mut self) {
        let Some(tree) = &mut self.value_tree else {
            return;
        };
        let rows = flatten(tree);
        let Some(current) = rows.get(self.tree_cursor) else {
            return;
        };
        let path = current.path.clone();
        if let Some(node) = tree.get_mut(&path) {
            node.key = Some(self.tree_edit_key.value.clone());
        }
        self.tree_editing_key = false;
        self.tree_edit_key.clear();
        self.tree_dirty = true;
        if self.tree_adding {
            self.tree_editing = true;
        }
    }

    fn tree_remove_current(&mut self) {
        let Some(tree) = &mut self.value_tree else {
            return;
        };
        let rows = flatten(tree);
        let Some(current) = rows.get(self.tree_cursor) else {
            return;
        };
        let mut path = current.path.clone();
        if path.is_empty() {
            return;
        }
        let idx = path.pop().unwrap();
        if let Some(parent) = tree.get_mut(&path) {
            match &mut parent.value {
                JsonNodeValue::Array(items) | JsonNodeValue::Object(items) => {
                    if idx < items.len() {
                        items.remove(idx);
                    }
                }
                JsonNodeValue::Leaf(_) => {}
            }
        }
        let len = flatten(tree).len();
        if self.tree_cursor >= len {
            self.tree_cursor = len.saturating_sub(1);
        }
    }

    pub fn tree_delete_current(&mut self) {
        let Some(tree) = &self.value_tree else {
            return;
        };
        let rows = flatten(tree);
        let Some(current) = rows.get(self.tree_cursor) else {
            return;
        };
        if current.path.is_empty() {
            return;
        }
        self.tree_remove_current();
        self.tree_dirty = true;
    }

    pub async fn refresh_tree(&mut self) {
        self.load_value().await;
        if self.status.is_empty() {
            self.enter_tree_mode();
            if let Some(tree) = &self.value_tree {
                let len = flatten(tree).len();
                self.tree_cursor = self.tree_cursor.min(len.saturating_sub(1));
            }
        }
    }

    pub async fn save_tree(&mut self) {
        let Some(tree) = &self.value_tree else {
            return;
        };
        self.value_edit_text = serde_json::to_string_pretty(&tree.to_value()).unwrap_or_default();
        self.save_value().await;
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(&self.value_text) {
            let mut tree = JsonNode::from_value(&value);
            tree.collapse_below_root();
            self.value_tree = Some(tree);
            self.tree_mode = true;
            self.tree_dirty = false;
            self.pending_confirm = None;
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
                self.value_revision.as_deref(),
            )
            .await
        {
            Ok(revision) => {
                self.value_text = serde_json::to_string_pretty(&value).unwrap_or_default();
                self.value_revision = revision;
                self.value_scroll = 0;
                self.status = "saved".to_string();
            }
            Err(err)
                if matches!(&err, roforgecloud_core::error::Error::Api { status, .. } if status.as_u16() == 409 || status.as_u16() == 412) =>
            {
                self.load_value().await;
                self.status = "conflict: entry changed on server — reloaded latest value, your edit was discarded".to_string();
            }
            Err(err) => {
                self.status = self.datastore_error(err);
            }
        }
    }

    pub async fn create_entry(&mut self) {
        let id = self.entries_create_id.value.trim();
        if id.is_empty() {
            self.status = "entry id cannot be empty".to_string();
            return;
        }
        let (scope, key) = match id.split_once('/') {
            Some((scope, key)) => (scope.to_string(), key.to_string()),
            None => ("global".to_string(), id.to_string()),
        };

        let value: serde_json::Value = match serde_json::from_str(&self.entries_create_value.value) {
            Ok(value) => value,
            Err(err) => {
                self.status = format!("invalid JSON: {err}");
                return;
            }
        };

        self.status = "creating...".to_string();
        match self
            .client
            .create_entry(self.universe_id, &self.data_store_id, &key, Some(&scope), &value)
            .await
        {
            Ok(()) => {
                self.entries_create_id.clear();
                self.entries_create_value.clear();
                self.entries_create_active = false;
                self.status = "created".to_string();
                self.load_entries_page().await;
            }
            Err(err) => {
                self.status = self.datastore_error(err);
            }
        }
    }

    pub async fn delete_entry(&mut self) {
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

    pub async fn load_ordered_entries(&mut self) {
        self.ordered_entries_page_tokens = vec![None];
        self.load_ordered_entries_page().await;
    }

    pub async fn load_next_ordered_entries_page(&mut self) {
        let Some(token) = self.ordered_entries_next_page_token.clone() else {
            return;
        };
        self.ordered_entries_page_tokens.push(Some(token));
        self.load_ordered_entries_page().await;
    }

    pub async fn load_prev_ordered_entries_page(&mut self) {
        if self.ordered_entries_page_tokens.len() <= 1 {
            return;
        }
        self.ordered_entries_page_tokens.pop();
        self.load_ordered_entries_page().await;
    }

    pub async fn load_ordered_entries_page(&mut self) {
        let page_token = self.ordered_entries_page_tokens.last().cloned().flatten();

        self.status = "loading entries...".to_string();
        match self
            .client
            .list_ordered_entries(
                self.universe_id,
                &self.ordered_data_store_id.value,
                &self.ordered_scope.value,
                None,
                None,
                page_token.as_deref(),
                Some(256),
            )
            .await
        {
            Ok(result) => {
                self.ordered_entries = result.ordered_data_store_entries;
                self.ordered_entries_selected = 0;
                self.ordered_entries_marked.clear();
                self.ordered_entries_next_page_token = result.next_page_token;
                let page = self.ordered_entries_page_tokens.len();
                self.status = format!("{} entries (page {page})", self.ordered_entries.len());
            }
            Err(err) => {
                self.status = self.datastore_error(err);
            }
        }
    }

    pub async fn load_all_ordered_entries_for_search(&mut self) {
        self.status = "loading all entries for search...".to_string();
        let mut all = Vec::new();
        let mut page_token: Option<String> = None;
        loop {
            match self
                .client
                .list_ordered_entries(
                    self.universe_id,
                    &self.ordered_data_store_id.value,
                    &self.ordered_scope.value,
                    None,
                    None,
                    page_token.as_deref(),
                    Some(256),
                )
                .await
            {
                Ok(result) => {
                    all.extend(result.ordered_data_store_entries);
                    page_token = result.next_page_token.filter(|t| !t.is_empty());
                    if page_token.is_none() {
                        break;
                    }
                }
                Err(err) => {
                    self.status = self.datastore_error(err);
                    return;
                }
            }
        }
        self.ordered_entries = all;
        self.ordered_entries_selected = 0;
        self.ordered_entries_marked.clear();
        self.ordered_entries_next_page_token = None;
        self.ordered_entries_page_tokens = vec![None];
        self.status = format!("{} entries (search across whole store)", self.ordered_entries.len());
    }

    pub fn visible_ordered_entry_indices(&self) -> Vec<usize> {
        if self.ordered_entries_search.value.is_empty() {
            return (0..self.ordered_entries.len()).collect();
        }

        let needle = self.ordered_entries_search.value.to_lowercase();
        self.ordered_entries
            .iter()
            .enumerate()
            .filter(|(_, entry)| entry.id.to_lowercase().contains(&needle))
            .map(|(i, _)| i)
            .collect()
    }

    fn current_ordered_entry_index(&self) -> Option<usize> {
        self.visible_ordered_entry_indices()
            .get(self.ordered_entries_selected)
            .copied()
    }

    pub fn toggle_ordered_entry_mark(&mut self) {
        if let Some(index) = self.current_ordered_entry_index() {
            if !self.ordered_entries_marked.remove(&index) {
                self.ordered_entries_marked.insert(index);
            }
        }
    }

    pub fn toggle_select_all_ordered_visible(&mut self) {
        let visible = self.visible_ordered_entry_indices();
        if visible.iter().all(|i| self.ordered_entries_marked.contains(i)) {
            for i in &visible {
                self.ordered_entries_marked.remove(i);
            }
        } else {
            self.ordered_entries_marked.extend(visible);
        }
    }

    pub async fn load_ordered_value(&mut self) {
        let Some(index) = self.current_ordered_entry_index() else {
            return;
        };
        let id = self.ordered_entries[index].id.clone();

        self.status = "loading value...".to_string();
        match self
            .client
            .get_ordered_entry(
                self.universe_id,
                &self.ordered_data_store_id.value,
                &self.ordered_scope.value,
                &id,
            )
            .await
        {
            Ok(entry) => {
                self.ordered_value_title =
                    format!("{}/{id} (scope: {})", self.ordered_data_store_id.value, self.ordered_scope.value);
                self.ordered_value = entry.value;
                self.ordered_value_editing = false;
                self.ordered_increment_editing = false;
                self.status.clear();
            }
            Err(err) => {
                self.status = self.datastore_error(err);
            }
        }
    }

    pub async fn save_ordered_value(&mut self) {
        let Some(index) = self.current_ordered_entry_index() else {
            return;
        };
        let id = self.ordered_entries[index].id.clone();

        let value: f64 = match self.ordered_value_edit.parse() {
            Ok(value) => value,
            Err(_) => {
                self.status = "invalid number".to_string();
                return;
            }
        };

        self.status = "saving...".to_string();
        match self
            .client
            .update_ordered_entry(
                self.universe_id,
                &self.ordered_data_store_id.value,
                &self.ordered_scope.value,
                &id,
                value,
            )
            .await
        {
            Ok(entry) => {
                self.ordered_value = entry.value;
                self.ordered_entries[index].value = entry.value;
                self.ordered_value_editing = false;
                self.status = "saved".to_string();
            }
            Err(err) => {
                self.status = self.datastore_error(err);
            }
        }
    }

    pub async fn increment_ordered_entry(&mut self) {
        let Some(index) = self.current_ordered_entry_index() else {
            return;
        };
        let id = self.ordered_entries[index].id.clone();

        let amount: f64 = match self.ordered_increment_edit.parse() {
            Ok(amount) => amount,
            Err(_) => {
                self.status = "invalid number".to_string();
                return;
            }
        };

        self.status = "incrementing...".to_string();
        match self
            .client
            .increment_ordered_entry(
                self.universe_id,
                &self.ordered_data_store_id.value,
                &self.ordered_scope.value,
                &id,
                amount,
            )
            .await
        {
            Ok(entry) => {
                self.ordered_value = entry.value;
                self.ordered_entries[index].value = entry.value;
                self.ordered_increment_editing = false;
                self.status = "incremented".to_string();
            }
            Err(err) => {
                self.status = self.datastore_error(err);
            }
        }
    }

    pub async fn delete_ordered_entry(&mut self) {
        let Some(index) = self.current_ordered_entry_index() else {
            return;
        };
        let id = self.ordered_entries[index].id.clone();

        self.status = "deleting...".to_string();
        match self
            .client
            .delete_ordered_entry(
                self.universe_id,
                &self.ordered_data_store_id.value,
                &self.ordered_scope.value,
                &id,
            )
            .await
        {
            Ok(()) => {
                self.ordered_entries.remove(index);
                let visible = self.visible_ordered_entry_indices().len();
                if self.ordered_entries_selected >= visible {
                    self.ordered_entries_selected = visible.saturating_sub(1);
                }
                if self.screen == Screen::OrderedValue {
                    self.screen = Screen::OrderedEntries;
                }
                self.status = "deleted".to_string();
            }
            Err(err) => {
                self.status = self.datastore_error(err);
            }
        }
    }

    pub async fn bulk_delete_ordered_entries(&mut self) {
        let mut indices: Vec<usize> = self.ordered_entries_marked.iter().copied().collect();
        indices.sort_unstable();

        let total = indices.len();
        if total == 0 {
            self.status = "no entries to delete".to_string();
            return;
        }

        let mut deleted_indices = Vec::new();
        let mut errors = 0;

        for &i in &indices {
            let id = self.ordered_entries[i].id.clone();
            self.status = format!("deleting {}/{total}...", deleted_indices.len() + errors + 1);
            match self
                .client
                .delete_ordered_entry(
                    self.universe_id,
                    &self.ordered_data_store_id.value,
                    &self.ordered_scope.value,
                    &id,
                )
                .await
            {
                Ok(()) => deleted_indices.push(i),
                Err(_) => errors += 1,
            }
        }

        for &i in deleted_indices.iter().rev() {
            self.ordered_entries.remove(i);
        }

        self.ordered_entries_marked.clear();

        let visible = self.visible_ordered_entry_indices().len();
        if self.ordered_entries_selected >= visible {
            self.ordered_entries_selected = visible.saturating_sub(1);
        }

        self.status = if errors == 0 {
            format!("deleted {} entries", deleted_indices.len())
        } else {
            format!("deleted {} entries, {errors} failed", deleted_indices.len())
        };
    }

    pub async fn create_ordered_entry(&mut self) {
        let id = self.ordered_create_id.value.trim();
        if id.is_empty() || id.len() > 63 {
            self.status = "entry id must be 1-63 characters".to_string();
            return;
        }
        let value: f64 = match self.ordered_create_value.value.parse() {
            Ok(value) => value,
            Err(_) => {
                self.status = "invalid number".to_string();
                return;
            }
        };

        self.status = "creating...".to_string();
        match self
            .client
            .create_ordered_entry(
                self.universe_id,
                &self.ordered_data_store_id.value,
                &self.ordered_scope.value,
                id,
                value,
            )
            .await
        {
            Ok(_) => {
                self.ordered_create_id.clear();
                self.ordered_create_value.clear();
                self.ordered_create_active = false;
                self.status = "created".to_string();
                self.load_ordered_entries_page().await;
            }
            Err(err) => {
                self.status = self.datastore_error(err);
            }
        }
    }

    pub async fn publish_message(&mut self) {
        if self.messaging_topic.value.is_empty() {
            self.status = "topic cannot be empty".to_string();
            return;
        }

        self.status = "publishing...".to_string();
        match self
            .client
            .publish_message(
                self.universe_id,
                &self.messaging_topic.value,
                &self.messaging_message.value,
            )
            .await
        {
            Ok(()) => {
                self.status = format!("published to '{}'", self.messaging_topic.value);
            }
            Err(err) => {
                self.status = format!("error: {err}");
            }
        }
    }
}
