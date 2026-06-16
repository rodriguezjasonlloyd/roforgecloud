use roforgecloud_core::auth;
use roforgecloud_core::oauth::{self, OAuthClient};
use roforgecloud_core::opencloud::datastore::DataStoreEntryInfo;
use roforgecloud_core::opencloud::memory_store::SortedMapItem;
use roforgecloud_core::opencloud::{ListQuery, OpenCloudClient};

use crate::tree_editor::TreeEditor;
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
        let prev_len = self.value[..self.cursor]
            .chars()
            .last()
            .map_or(0, char::len_utf8);
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
            let prev_len = self.value[..self.cursor]
                .chars()
                .last()
                .map_or(0, char::len_utf8);
            self.cursor -= prev_len;
        }
    }

    pub fn right(&mut self) {
        if self.cursor < self.value.len() {
            let next_len = self.value[self.cursor..]
                .chars()
                .next()
                .map_or(0, char::len_utf8);
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
    MemoryStoreInput,
    MemoryStoreEntries,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueSource {
    DataStore,
    MemoryStoreSortedMap,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TreeTarget {
    Value,
    EntriesCreate,
    MemoryCreate,
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
    LoadMemoryItems,
    LoadNextMemoryItemsPage,
    LoadPrevMemoryItemsPage,
    RefreshMemoryItems,
    LoadAllMemoryItemsForSearch,
    CreateMemoryItem,
    CreateMemoryItemExternal,
    LoadMemoryValue,
    SaveMemoryTtl,
    DeleteMemoryItem,
    BulkDeleteMemoryItems,
}

pub const UNIVERSE_CHOICE_LIST_ALL: usize = 0;
pub const UNIVERSE_CHOICE_ENTER_ID: usize = 1;
pub const UNIVERSE_CHOICE_ITEMS: &[&str] = &["List my universes (OAuth)", "Enter universe ID"];

pub const SERVICE_DATA_STORES: usize = 0;
pub const SERVICE_ORDERED_DATA_STORES: usize = 1;
pub const SERVICE_MEMORY_STORES: usize = 2;
pub const SERVICE_MESSAGING: usize = 3;
pub const SERVICE_ACCOUNT: usize = 4;

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
pub enum MemoryCreateField {
    Id,
    Value,
    Ttl,
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
    DeleteMemoryItem,
    BulkDeleteMemoryItems,
    Quit,
    TreeQuit,
    TreeRefresh,
}

impl PendingConfirm {
    pub fn footer_hint(&self) -> String {
        use crate::update::{render_hints, HintEntry, ANY_OTHER_KEY_CANCEL};

        let confirm = match self {
            PendingConfirm::DeleteStore
            | PendingConfirm::BulkDeleteStores
            | PendingConfirm::DeleteEntry
            | PendingConfirm::BulkDeleteEntries
            | PendingConfirm::DeleteOrderedEntry
            | PendingConfirm::BulkDeleteOrderedEntries
            | PendingConfirm::DeleteMemoryItem
            | PendingConfirm::BulkDeleteMemoryItems => HintEntry::new("d", "confirm delete"),
            PendingConfirm::BulkUndeleteStores => HintEntry::new("u", "confirm undelete"),
            PendingConfirm::Quit => HintEntry::new("q", "confirm quit"),
            PendingConfirm::TreeQuit => HintEntry::new("q/esc", "discard changes and exit tree"),
            PendingConfirm::TreeRefresh => HintEntry::new("r", "discard changes and refresh"),
        };

        render_hints(&[confirm, ANY_OTHER_KEY_CANCEL])
    }
}

pub struct App {
    pub client: OpenCloudClient,
    pub has_api_key: bool,
    pub oauth: Option<OAuthClient>,
    pub redirect_uri: String,
    pub logged_in: bool,
    pub universe_id: u64,
    pub available_universes: Vec<u64>,
    pub universe_names: std::collections::HashMap<u64, String>,
    pub universe_name_rx: tokio::sync::mpsc::UnboundedReceiver<(u64, String)>,
    pub universe_name_tx: tokio::sync::mpsc::UnboundedSender<(u64, String)>,

    pub universe_choice: crate::screens::universe_choice::State,
    pub universe_select: crate::screens::universe_select::State,
    pub universe_input: crate::screens::universe_input::State,
    pub screen: Screen,
    pub should_quit: bool,
    pub loading: bool,
    pub status: String,

    pub menu: crate::screens::menu::State,

    pub stores: crate::screens::stores::State,

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

    pub tree_editor: Option<TreeEditor>,
    pub tree_target: TreeTarget,
    pub clipboard: Option<arboard::Clipboard>,

    pub which_key: crate::update::Keys,

    pub messaging: crate::screens::messaging::State,

    pub ordered_store_input: crate::screens::ordered_store_input::State,

    pub ordered_entries: crate::screens::ordered_entries::State,

    pub ordered_value: crate::screens::ordered_value::State,

    pub value_source: ValueSource,

    pub memory_store_input: crate::screens::memory_store_input::State,

    pub memory_items: Vec<SortedMapItem>,
    pub memory_items_selected: usize,
    pub memory_items_next_page_token: Option<String>,
    pub memory_items_page_tokens: Vec<Option<String>>,
    pub memory_items_marked: std::collections::HashSet<usize>,
    pub memory_items_search: TextField,
    pub memory_items_search_active: bool,

    pub memory_create_id: TextField,
    pub memory_create_value: TextField,
    pub memory_create_ttl: TextField,
    pub memory_create_field: MemoryCreateField,
    pub memory_create_active: bool,
    pub memory_create_choosing: bool,

    pub memory_ttl_edit: TextField,
    pub memory_ttl_editing: bool,

    pub memory_item_editing_id: String,
    pub memory_item_ttl_seconds: u64,
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
        let username_channel = tokio::sync::mpsc::unbounded_channel();
        let universe_name_channel = tokio::sync::mpsc::unbounded_channel();

        Self {
            client,
            has_api_key,
            oauth,
            redirect_uri,
            logged_in,
            universe_id: 0,
            available_universes,
            universe_names: std::collections::HashMap::new(),
            universe_name_rx: universe_name_channel.1,
            universe_name_tx: universe_name_channel.0,
            universe_choice: crate::screens::universe_choice::State::new(),
            universe_select: crate::screens::universe_select::State::new(),
            universe_input: crate::screens::universe_input::State::new(),
            screen: Screen::Menu,
            should_quit: false,
            loading: false,
            status: String::new(),
            menu: crate::screens::menu::State::new(),
            stores: crate::screens::stores::State::new(),
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
            tree_editor: None,
            tree_target: TreeTarget::Value,
            clipboard: arboard::Clipboard::new().ok(),
            messaging: crate::screens::messaging::State::new(),
            ordered_store_input: crate::screens::ordered_store_input::State::new(),
            ordered_entries: crate::screens::ordered_entries::State::new(),
            ordered_value: crate::screens::ordered_value::State::new(),
            value_source: ValueSource::DataStore,
            memory_store_input: crate::screens::memory_store_input::State::new(),
            memory_items: Vec::new(),
            memory_items_selected: 0,
            memory_items_next_page_token: None,
            memory_items_page_tokens: vec![None],
            memory_items_marked: std::collections::HashSet::new(),
            memory_items_search: TextField::default(),
            memory_items_search_active: false,
            memory_create_id: TextField::default(),
            memory_create_value: TextField::default(),
            memory_create_ttl: TextField {
                value: "3600".to_string(),
                cursor: "3600".len(),
            },
            memory_create_field: MemoryCreateField::Id,
            memory_create_active: false,
            memory_create_choosing: false,
            memory_ttl_edit: TextField::default(),
            memory_ttl_editing: false,
            memory_item_editing_id: String::new(),
            memory_item_ttl_seconds: 3600,
            which_key: crate::update::Keys::new(
                crate::update::build_keymap(),
                crate::update::Scope::Menu,
            ),
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
            Action::LoadAllOrderedEntriesForSearch => {
                self.load_all_ordered_entries_for_search().await
            }
            Action::CreateOrderedEntry => self.create_ordered_entry().await,
            Action::CreateOrderedEntryExternal => {}
            Action::LoadOrderedValue => self.load_ordered_value().await,
            Action::SaveOrderedValue => self.save_ordered_value().await,
            Action::DeleteOrderedEntry => self.delete_ordered_entry().await,
            Action::BulkDeleteOrderedEntries => self.bulk_delete_ordered_entries().await,
            Action::IncrementOrderedEntry => self.increment_ordered_entry().await,
            Action::LoadMemoryItems => self.load_memory_items().await,
            Action::LoadNextMemoryItemsPage => self.load_next_memory_items_page().await,
            Action::LoadPrevMemoryItemsPage => self.load_prev_memory_items_page().await,
            Action::RefreshMemoryItems => self.load_memory_items_page().await,
            Action::LoadAllMemoryItemsForSearch => self.load_all_memory_items_for_search().await,
            Action::CreateMemoryItem => self.create_memory_item().await,
            Action::CreateMemoryItemExternal => {}
            Action::LoadMemoryValue => self.load_memory_value().await,
            Action::SaveMemoryTtl => self.save_memory_ttl().await,
            Action::DeleteMemoryItem => self.delete_memory_item().await,
            Action::BulkDeleteMemoryItems => self.bulk_delete_memory_items().await,
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
        match auth::force_login(oauth, &self.redirect_uri, &auth::NoopLoginPrompt).await {
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
            let token = auth::access_token(oauth, &self.redirect_uri, &auth::NoopLoginPrompt).await?;
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
                self.universe_select.selected = 0;
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
            .list_data_stores(
                self.universe_id,
                &ListQuery {
                    show_deleted: true,
                    ..Default::default()
                },
            )
            .await
        {
            Ok(result) => {
                self.stores.items = result.data_stores;
                self.stores.selected = 0;
                self.stores.marked.clear();
                self.status = format!("{} data stores", self.stores.items.len());
            }
            Err(err) => {
                self.status = self.datastore_error(err);
            }
        }
    }

    pub async fn delete_data_store(&mut self) {
        let Some(store) = self.stores.items.get(self.stores.selected) else {
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
                self.stores.items[self.stores.selected] = info;
            }
            Err(err) => {
                self.status = self.datastore_error(err);
            }
        }
    }

    pub async fn undelete_data_store(&mut self) {
        let Some(store) = self.stores.items.get(self.stores.selected) else {
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
                self.stores.items[self.stores.selected] = info;
            }
            Err(err) => {
                self.status = self.datastore_error(err);
            }
        }
    }

    pub async fn bulk_delete_data_stores(&mut self) {
        let mut indices: Vec<usize> = self.stores.marked.iter().copied().collect();
        indices.sort_unstable();

        let total = indices.len();
        let mut errors = 0;

        for (n, &i) in indices.iter().enumerate() {
            let id = self.stores.items[i].id.clone();
            self.status = format!("scheduling deletion {}/{total}...", n + 1);
            match self.client.delete_data_store(self.universe_id, &id).await {
                Ok(info) => self.stores.items[i] = info,
                Err(_) => errors += 1,
            }
        }

        self.stores.marked.clear();
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
        let mut indices: Vec<usize> = self.stores.marked.iter().copied().collect();
        indices.sort_unstable();

        let total = indices.len();
        let mut errors = 0;

        for (n, &i) in indices.iter().enumerate() {
            let id = self.stores.items[i].id.clone();
            self.status = format!("restoring {}/{total}...", n + 1);
            match self.client.undelete_data_store(self.universe_id, &id).await {
                Ok(info) => self.stores.items[i] = info,
                Err(_) => errors += 1,
            }
        }

        self.stores.marked.clear();
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
                &self.stores.data_store_id,
                None,
                &ListQuery {
                    page_token: page_token.as_deref(),
                    max_page_size: Some(256),
                    ..Default::default()
                },
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
                .list_entries(
                    self.universe_id,
                    &self.stores.data_store_id,
                    None,
                    &ListQuery {
                        page_token: page_token.as_deref(),
                        max_page_size: Some(256),
                        ..Default::default()
                    },
                )
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
        if self.universe_select.search.value.is_empty() {
            return (0..self.available_universes.len()).collect();
        }

        let needle = self.universe_select.search.value.to_lowercase();
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
        !self.stores.marked.is_empty()
            || !self.entries_marked.is_empty()
            || !self.ordered_entries.marked.is_empty()
            || !self.memory_items_marked.is_empty()
    }

    pub fn text_input_active(&self) -> bool {
        match self.screen {
            Screen::UniverseInput
            | Screen::Messaging
            | Screen::OrderedStoreInput
            | Screen::MemoryStoreInput => true,
            Screen::UniverseSelect => self.universe_select.search_active,
            Screen::Stores => self.stores.new_active,
            Screen::Entries => {
                self.entries_search_active
                    || self.entries_create_active
                    || self.entries_create_choosing
            }
            Screen::Value => {
                self.tree_editor.as_ref().is_some_and(|t| t.is_editing())
                    || self.memory_ttl_editing
            }
            Screen::OrderedEntries => {
                self.ordered_entries.search_active
                    || self.ordered_entries.create_active
                    || self.ordered_entries.create_choosing
            }
            Screen::OrderedValue => self.ordered_value.editing || self.ordered_value.increment_editing,
            Screen::MemoryStoreEntries => {
                self.memory_items_search_active
                    || self.memory_create_active
                    || self.memory_create_choosing
            }
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
        if self.value_source == ValueSource::MemoryStoreSortedMap {
            return self.load_memory_value().await;
        }

        let Some((scope, key)) = self.current_entry_scope_key() else {
            return;
        };

        self.status = "loading value...".to_string();
        match self
            .client
            .get_entry_with_revision(self.universe_id, &self.stores.data_store_id, &key, Some(&scope))
            .await
        {
            Ok((value, revision)) => {
                self.value_title = format!("{}/{} (scope: {scope})", self.stores.data_store_id, key);
                self.value_text = serde_json::to_string_pretty(&value).unwrap_or_default();
                self.value_revision = revision;
                self.value_scroll = 0;
                self.tree_editor = None;
                self.value_source = ValueSource::DataStore;
                self.status.clear();
            }
            Err(err) => {
                self.status = self.datastore_error(err);
            }
        }
    }

    pub fn enter_tree_mode(&mut self) {
        self.enter_tree_mode_for(TreeTarget::Value);
    }

    pub fn enter_tree_mode_for(&mut self, target: TreeTarget) {
        let source = match target {
            TreeTarget::Value => self.value_text.clone(),
            TreeTarget::EntriesCreate => self.entries_create_value.value.clone(),
            TreeTarget::MemoryCreate => self.memory_create_value.value.clone(),
        };
        let source = source.trim();
        let source = if source.is_empty() { "{}" } else { source };

        match serde_json::from_str::<serde_json::Value>(source) {
            Ok(value) => {
                self.tree_editor = Some(TreeEditor::new(&value));
                self.tree_target = target;
                self.pending_confirm = None;
                self.status.clear();
            }
            Err(err) => {
                self.status = format!("invalid JSON: {err}");
            }
        }
    }

    pub fn exit_tree_mode(&mut self) {
        self.tree_editor = None;
        self.pending_confirm = None;
        self.status.clear();
    }

    pub async fn refresh_tree(&mut self) {
        let cursor = self.tree_editor.as_ref().map(|t| t.cursor()).unwrap_or(0);
        self.load_value().await;
        if self.status.is_empty() {
            self.enter_tree_mode();
            if let Some(editor) = &mut self.tree_editor {
                editor.set_cursor(cursor);
            }
        }
    }

    pub async fn save_tree(&mut self) {
        let Some(editor) = &self.tree_editor else {
            return;
        };
        let json = serde_json::to_string_pretty(&editor.to_value()).unwrap_or_default();

        match self.tree_target {
            TreeTarget::Value => {
                self.value_edit_text = json;
                self.save_value().await;
                if let Ok(value) = serde_json::from_str::<serde_json::Value>(&self.value_text) {
                    self.tree_editor = Some(TreeEditor::new(&value));
                    self.pending_confirm = None;
                }
            }
            TreeTarget::EntriesCreate => {
                self.entries_create_value.set(json);
                self.exit_tree_mode();
            }
            TreeTarget::MemoryCreate => {
                self.memory_create_value.set(json);
                self.exit_tree_mode();
            }
        }
    }

    pub async fn save_value(&mut self) {
        if self.value_source == ValueSource::MemoryStoreSortedMap {
            return self.save_memory_value().await;
        }

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
                &self.stores.data_store_id,
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
            Err(err) if matches!(&err, roforgecloud_core::error::Error::Api { status, .. } if status.as_u16() == 409 || status.as_u16() == 412) =>
            {
                self.load_value().await;
                self.status = "conflict: entry changed on server — reloaded latest value, your edit was discarded".to_string();
            }
            Err(err) => {
                self.status = self.datastore_error(err);
            }
        }
    }

    pub async fn save_memory_value(&mut self) {
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
            .update_sorted_map_item(
                self.universe_id,
                &self.memory_store_input.id,
                &self.memory_item_editing_id,
                &value,
                self.memory_item_ttl_seconds,
                self.value_revision.as_deref(),
            )
            .await
        {
            Ok(item) => {
                self.value_text = serde_json::to_string_pretty(&item.value).unwrap_or_default();
                self.value_revision = item.etag.clone();
                self.value_scroll = 0;
                if let Some(cached) = self.memory_items.iter_mut().find(|i| i.id == item.id) {
                    cached.value = item.value;
                    cached.etag = item.etag;
                    cached.expire_time = item.expire_time;
                }
                self.status = "saved".to_string();
            }
            Err(err) if matches!(&err, roforgecloud_core::error::Error::Api { status, .. } if status.as_u16() == 409 || status.as_u16() == 412) =>
            {
                self.load_memory_value().await;
                self.status = "conflict: item changed on server — reloaded latest value, your edit was discarded".to_string();
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

        let value: serde_json::Value = match serde_json::from_str(&self.entries_create_value.value)
        {
            Ok(value) => value,
            Err(err) => {
                self.status = format!("invalid JSON: {err}");
                return;
            }
        };

        self.status = "creating...".to_string();
        match self
            .client
            .create_entry(
                self.universe_id,
                &self.stores.data_store_id,
                &key,
                Some(&scope),
                &value,
            )
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
            .delete_entry(self.universe_id, &self.stores.data_store_id, &key, Some(&scope))
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
                .delete_entry(self.universe_id, &self.stores.data_store_id, key, Some(scope))
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
        self.ordered_entries.page_tokens = vec![None];
        self.load_ordered_entries_page().await;
    }

    pub async fn load_next_ordered_entries_page(&mut self) {
        let Some(token) = self.ordered_entries.next_page_token.clone() else {
            return;
        };
        self.ordered_entries.page_tokens.push(Some(token));
        self.load_ordered_entries_page().await;
    }

    pub async fn load_prev_ordered_entries_page(&mut self) {
        if self.ordered_entries.page_tokens.len() <= 1 {
            return;
        }
        self.ordered_entries.page_tokens.pop();
        self.load_ordered_entries_page().await;
    }

    pub async fn load_ordered_entries_page(&mut self) {
        let page_token = self.ordered_entries.page_tokens.last().cloned().flatten();

        self.status = "loading entries...".to_string();
        match self
            .client
            .list_ordered_entries(
                self.universe_id,
                &self.ordered_store_input.store_id.value,
                &self.ordered_store_input.scope.value,
                &ListQuery {
                    page_token: page_token.as_deref(),
                    max_page_size: Some(256),
                    ..Default::default()
                },
            )
            .await
        {
            Ok(result) => {
                self.ordered_entries.items = result.ordered_data_store_entries;
                self.ordered_entries.selected = 0;
                self.ordered_entries.marked.clear();
                self.ordered_entries.next_page_token = result.next_page_token;
                let page = self.ordered_entries.page_tokens.len();
                self.status = format!("{} entries (page {page})", self.ordered_entries.items.len());
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
                    &self.ordered_store_input.store_id.value,
                    &self.ordered_store_input.scope.value,
                    &ListQuery {
                        page_token: page_token.as_deref(),
                        max_page_size: Some(256),
                        ..Default::default()
                    },
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
        self.ordered_entries.items = all;
        self.ordered_entries.selected = 0;
        self.ordered_entries.marked.clear();
        self.ordered_entries.next_page_token = None;
        self.ordered_entries.page_tokens = vec![None];
        self.status = format!(
            "{} entries (search across whole store)",
            self.ordered_entries.items.len()
        );
    }

    pub fn visible_ordered_entry_indices(&self) -> Vec<usize> {
        self.ordered_entries.visible_indices()
    }

    pub async fn load_ordered_value(&mut self) {
        let Some(index) = self.ordered_entries.current_index() else {
            return;
        };
        let id = self.ordered_entries.items[index].id.clone();

        self.status = "loading value...".to_string();
        match self
            .client
            .get_ordered_entry(
                self.universe_id,
                &self.ordered_store_input.store_id.value,
                &self.ordered_store_input.scope.value,
                &id,
            )
            .await
        {
            Ok(entry) => {
                self.ordered_value.title = format!(
                    "{}/{id} (scope: {})",
                    self.ordered_store_input.store_id.value, self.ordered_store_input.scope.value
                );
                self.ordered_value.value = entry.value;
                self.ordered_value.editing = false;
                self.ordered_value.increment_editing = false;
                self.status.clear();
            }
            Err(err) => {
                self.status = self.datastore_error(err);
            }
        }
    }

    pub async fn save_ordered_value(&mut self) {
        let Some(index) = self.ordered_entries.current_index() else {
            return;
        };
        let id = self.ordered_entries.items[index].id.clone();

        let value: f64 = match self.ordered_value.edit.parse() {
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
                &self.ordered_store_input.store_id.value,
                &self.ordered_store_input.scope.value,
                &id,
                value,
            )
            .await
        {
            Ok(entry) => {
                self.ordered_value.value = entry.value;
                self.ordered_entries.items[index].value = entry.value;
                self.ordered_value.editing = false;
                self.status = "saved".to_string();
            }
            Err(err) => {
                self.status = self.datastore_error(err);
            }
        }
    }

    pub async fn increment_ordered_entry(&mut self) {
        let Some(index) = self.ordered_entries.current_index() else {
            return;
        };
        let id = self.ordered_entries.items[index].id.clone();

        let amount: f64 = match self.ordered_value.increment_edit.parse() {
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
                &self.ordered_store_input.store_id.value,
                &self.ordered_store_input.scope.value,
                &id,
                amount,
            )
            .await
        {
            Ok(entry) => {
                self.ordered_value.value = entry.value;
                self.ordered_entries.items[index].value = entry.value;
                self.ordered_value.increment_editing = false;
                self.status = "incremented".to_string();
            }
            Err(err) => {
                self.status = self.datastore_error(err);
            }
        }
    }

    pub async fn delete_ordered_entry(&mut self) {
        let Some(index) = self.ordered_entries.current_index() else {
            return;
        };
        let id = self.ordered_entries.items[index].id.clone();

        self.status = "deleting...".to_string();
        match self
            .client
            .delete_ordered_entry(
                self.universe_id,
                &self.ordered_store_input.store_id.value,
                &self.ordered_store_input.scope.value,
                &id,
            )
            .await
        {
            Ok(()) => {
                self.ordered_entries.items.remove(index);
                let visible = self.visible_ordered_entry_indices().len();
                if self.ordered_entries.selected >= visible {
                    self.ordered_entries.selected = visible.saturating_sub(1);
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
        let mut indices: Vec<usize> = self.ordered_entries.marked.iter().copied().collect();
        indices.sort_unstable();

        let total = indices.len();
        if total == 0 {
            self.status = "no entries to delete".to_string();
            return;
        }

        let mut deleted_indices = Vec::new();
        let mut errors = 0;

        for &i in &indices {
            let id = self.ordered_entries.items[i].id.clone();
            self.status = format!("deleting {}/{total}...", deleted_indices.len() + errors + 1);
            match self
                .client
                .delete_ordered_entry(
                    self.universe_id,
                    &self.ordered_store_input.store_id.value,
                    &self.ordered_store_input.scope.value,
                    &id,
                )
                .await
            {
                Ok(()) => deleted_indices.push(i),
                Err(_) => errors += 1,
            }
        }

        for &i in deleted_indices.iter().rev() {
            self.ordered_entries.items.remove(i);
        }

        self.ordered_entries.marked.clear();

        let visible = self.visible_ordered_entry_indices().len();
        if self.ordered_entries.selected >= visible {
            self.ordered_entries.selected = visible.saturating_sub(1);
        }

        self.status = if errors == 0 {
            format!("deleted {} entries", deleted_indices.len())
        } else {
            format!("deleted {} entries, {errors} failed", deleted_indices.len())
        };
    }

    pub async fn create_ordered_entry(&mut self) {
        let id = self.ordered_entries.create_id.value.trim();
        if id.is_empty() || id.len() > 63 {
            self.status = "entry id must be 1-63 characters".to_string();
            return;
        }
        let value: f64 = match self.ordered_entries.create_value.value.parse() {
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
                &self.ordered_store_input.store_id.value,
                &self.ordered_store_input.scope.value,
                id,
                value,
            )
            .await
        {
            Ok(_) => {
                self.ordered_entries.create_id.clear();
                self.ordered_entries.create_value.clear();
                self.ordered_entries.create_active = false;
                self.status = "created".to_string();
                self.load_ordered_entries_page().await;
            }
            Err(err) => {
                self.status = self.datastore_error(err);
            }
        }
    }

    pub async fn publish_message(&mut self) {
        if self.messaging.topic.value.is_empty() {
            self.status = "topic cannot be empty".to_string();
            return;
        }

        self.status = "publishing...".to_string();
        match self
            .client
            .publish_message(
                self.universe_id,
                &self.messaging.topic.value,
                &self.messaging.message.value,
            )
            .await
        {
            Ok(()) => {
                self.status = format!("published to '{}'", self.messaging.topic.value);
            }
            Err(err) => {
                self.status = format!("error: {err}");
            }
        }
    }

    pub async fn load_memory_items(&mut self) {
        self.memory_items_page_tokens = vec![None];
        self.load_memory_items_page().await;
    }

    pub async fn load_next_memory_items_page(&mut self) {
        let Some(token) = self.memory_items_next_page_token.clone() else {
            return;
        };
        self.memory_items_page_tokens.push(Some(token));
        self.load_memory_items_page().await;
    }

    pub async fn load_prev_memory_items_page(&mut self) {
        if self.memory_items_page_tokens.len() <= 1 {
            return;
        }
        self.memory_items_page_tokens.pop();
        self.load_memory_items_page().await;
    }

    pub async fn load_memory_items_page(&mut self) {
        let page_token = self.memory_items_page_tokens.last().cloned().flatten();

        self.status = "loading items...".to_string();
        match self
            .client
            .list_sorted_map_items(
                self.universe_id,
                &self.memory_store_input.id,
                &ListQuery {
                    page_token: page_token.as_deref(),
                    max_page_size: Some(256),
                    ..Default::default()
                },
            )
            .await
        {
            Ok(result) => {
                self.memory_items = result.items;
                self.memory_items_selected = 0;
                self.memory_items_marked.clear();
                self.memory_items_next_page_token =
                    result.next_page_token.filter(|t| !t.is_empty());
                let page = self.memory_items_page_tokens.len();
                self.status = format!("{} items (page {page})", self.memory_items.len());
            }
            Err(err) => {
                self.status = self.datastore_error(err);
            }
        }
    }

    pub async fn load_all_memory_items_for_search(&mut self) {
        self.status = "loading all items for search...".to_string();
        let mut all = Vec::new();
        let mut page_token: Option<String> = None;
        loop {
            match self
                .client
                .list_sorted_map_items(
                    self.universe_id,
                    &self.memory_store_input.id,
                    &ListQuery {
                        page_token: page_token.as_deref(),
                        max_page_size: Some(256),
                        ..Default::default()
                    },
                )
                .await
            {
                Ok(result) => {
                    all.extend(result.items);
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
        self.memory_items = all;
        self.memory_items_selected = 0;
        self.memory_items_marked.clear();
        self.memory_items_next_page_token = None;
        self.memory_items_page_tokens = vec![None];
        self.status = format!(
            "{} items (search across whole sorted map)",
            self.memory_items.len()
        );
    }

    pub fn visible_memory_item_indices(&self) -> Vec<usize> {
        if self.memory_items_search.value.is_empty() {
            return (0..self.memory_items.len()).collect();
        }

        let needle = self.memory_items_search.value.to_lowercase();
        self.memory_items
            .iter()
            .enumerate()
            .filter(|(_, item)| item.id.to_lowercase().contains(&needle))
            .map(|(i, _)| i)
            .collect()
    }

    fn current_memory_item_index(&self) -> Option<usize> {
        self.visible_memory_item_indices()
            .get(self.memory_items_selected)
            .copied()
    }

    pub fn toggle_memory_item_mark(&mut self) {
        if let Some(index) = self.current_memory_item_index() {
            if !self.memory_items_marked.remove(&index) {
                self.memory_items_marked.insert(index);
            }
        }
    }

    pub fn toggle_select_all_memory_visible(&mut self) {
        let visible = self.visible_memory_item_indices();
        if visible.iter().all(|i| self.memory_items_marked.contains(i)) {
            for i in &visible {
                self.memory_items_marked.remove(i);
            }
        } else {
            self.memory_items_marked.extend(visible);
        }
    }

    pub async fn load_memory_value(&mut self) {
        let Some(index) = self.current_memory_item_index() else {
            return;
        };
        let id = self.memory_items[index].id.clone();

        self.status = "loading value...".to_string();
        match self
            .client
            .get_sorted_map_item(self.universe_id, &self.memory_store_input.id, &id)
            .await
        {
            Ok(item) => {
                let expire = item.expire_time.clone().unwrap_or_else(|| "—".to_string());
                self.value_title =
                    format!("{}/{id} (expires: {expire})", self.memory_store_input.id);
                self.value_text = serde_json::to_string_pretty(&item.value).unwrap_or_default();
                self.value_revision = item.etag;
                self.value_scroll = 0;
                self.tree_editor = None;
                self.value_source = ValueSource::MemoryStoreSortedMap;
                self.memory_item_editing_id = id;
                self.memory_item_ttl_seconds = 3600;
                self.screen = Screen::Value;
                self.status.clear();
            }
            Err(err) => {
                self.status = self.datastore_error(err);
            }
        }
    }

    pub async fn create_memory_item(&mut self) {
        let id = self.memory_create_id.value.trim();
        if id.is_empty() || id.len() > 63 {
            self.status = "item id must be 1-63 characters".to_string();
            return;
        }
        let value: serde_json::Value = match serde_json::from_str(&self.memory_create_value.value) {
            Ok(value) => value,
            Err(err) => {
                self.status = format!("invalid JSON: {err}");
                return;
            }
        };
        let ttl: u64 = match self.memory_create_ttl.value.parse() {
            Ok(ttl) => ttl,
            Err(_) => {
                self.status = "invalid ttl".to_string();
                return;
            }
        };

        self.status = "creating...".to_string();
        match self
            .client
            .create_sorted_map_item(
                self.universe_id,
                &self.memory_store_input.id,
                id,
                &value,
                ttl,
            )
            .await
        {
            Ok(_) => {
                self.memory_create_id.clear();
                self.memory_create_value.clear();
                self.memory_create_ttl.set("3600");
                self.memory_create_active = false;
                self.status = "created".to_string();
                self.load_memory_items_page().await;
            }
            Err(err) => {
                self.status = self.datastore_error(err);
            }
        }
    }

    pub async fn delete_memory_item(&mut self) {
        let Some(index) = self.current_memory_item_index() else {
            return;
        };
        let id = self.memory_items[index].id.clone();

        self.status = "deleting...".to_string();
        match self
            .client
            .delete_sorted_map_item(self.universe_id, &self.memory_store_input.id, &id)
            .await
        {
            Ok(()) => {
                self.memory_items.remove(index);
                let visible = self.visible_memory_item_indices().len();
                if self.memory_items_selected >= visible {
                    self.memory_items_selected = visible.saturating_sub(1);
                }
                if self.screen == Screen::Value {
                    self.screen = Screen::MemoryStoreEntries;
                }
                self.status = "deleted".to_string();
            }
            Err(err) => {
                self.status = self.datastore_error(err);
            }
        }
    }

    pub async fn bulk_delete_memory_items(&mut self) {
        let mut indices: Vec<usize> = self.memory_items_marked.iter().copied().collect();
        indices.sort_unstable();

        let total = indices.len();
        if total == 0 {
            self.status = "no items to delete".to_string();
            return;
        }

        let mut deleted_indices = Vec::new();
        let mut errors = 0;

        for &i in &indices {
            let id = self.memory_items[i].id.clone();
            self.status = format!("deleting {}/{total}...", deleted_indices.len() + errors + 1);
            match self
                .client
                .delete_sorted_map_item(self.universe_id, &self.memory_store_input.id, &id)
                .await
            {
                Ok(()) => deleted_indices.push(i),
                Err(_) => errors += 1,
            }
        }

        for &i in deleted_indices.iter().rev() {
            self.memory_items.remove(i);
        }

        self.memory_items_marked.clear();

        let visible = self.visible_memory_item_indices().len();
        if self.memory_items_selected >= visible {
            self.memory_items_selected = visible.saturating_sub(1);
        }

        self.status = if errors == 0 {
            format!("deleted {} items", deleted_indices.len())
        } else {
            format!("deleted {} items, {errors} failed", deleted_indices.len())
        };
    }

    pub async fn save_memory_ttl(&mut self) {
        let Some(index) = self.current_memory_item_index() else {
            return;
        };
        let item = self.memory_items[index].clone();

        let ttl: u64 = match self.memory_ttl_edit.value.parse() {
            Ok(ttl) => ttl,
            Err(_) => {
                self.status = "invalid ttl".to_string();
                return;
            }
        };

        self.status = "saving...".to_string();
        match self
            .client
            .update_sorted_map_item(
                self.universe_id,
                &self.memory_store_input.id,
                &item.id,
                &item.value,
                ttl,
                item.etag.as_deref(),
            )
            .await
        {
            Ok(updated) => {
                self.memory_items[index].etag = updated.etag;
                self.memory_items[index].expire_time = updated.expire_time;
                self.memory_ttl_editing = false;
                self.status = "saved".to_string();
            }
            Err(err) => {
                self.status = self.datastore_error(err);
            }
        }
    }
}
