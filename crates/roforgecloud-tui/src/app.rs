use roforgecloud_core::oauth::OAuthClient;
use roforgecloud_core::opencloud::OpenCloudClient;

use crate::screens;
use crate::status;
use crate::tree_editor::TreeEditor;
use crate::update;

pub type TextField = tui_textarea::TextArea<'static>;

pub trait TextFieldExt {
    fn get_value(&self) -> &str;
    fn set_value(&mut self, s: impl Into<String>);
}

impl TextFieldExt for TextField {
    fn get_value(&self) -> &str {
        self.lines().first().map(|s| s.as_str()).unwrap_or("")
    }
    fn set_value(&mut self, s: impl Into<String>) {
        *self = tui_textarea::TextArea::from([s.into()]);
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

    pub universe_choice: screens::universe_choice::State,
    pub universe_select: screens::universe_select::State,
    pub universe_input: screens::universe_input::State,
    pub screen: Screen,
    pub should_quit: bool,
    pub loading: bool,
    pub status: status::Msg,

    pub menu: screens::menu::State,

    pub stores: screens::stores::State,

    pub entries: screens::entries::State,
    pub pending_confirm: Option<PendingConfirm>,
    pub confirm_deadline: Option<std::time::Instant>,

    pub value: screens::value::State,

    pub tree_editor: Option<TreeEditor>,
    pub tree_target: TreeTarget,
    pub clipboard: Option<arboard::Clipboard>,

    pub which_key: update::Keys,

    pub messaging: screens::messaging::State,

    pub ordered_store_input: screens::ordered_store_input::State,

    pub ordered_entries: screens::ordered_entries::State,

    pub ordered_value: screens::ordered_value::State,

    pub memory_store_input: screens::memory_store_input::State,

    pub memory_entries: screens::memory_entries::State,

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
            universe_choice: screens::universe_choice::State::new(),
            universe_select: screens::universe_select::State::new(),
            universe_input: screens::universe_input::State::new(),
            screen: Screen::Menu,
            should_quit: false,
            loading: false,
            status: status::Msg::default(),
            menu: screens::menu::State::new(),
            stores: screens::stores::State::new(),
            entries: screens::entries::State::new(),
            pending_confirm: None,
            confirm_deadline: None,
            value: screens::value::State::new(),
            tree_editor: None,
            tree_target: TreeTarget::Value,
            clipboard: arboard::Clipboard::new().ok(),
            messaging: screens::messaging::State::new(),
            ordered_store_input: screens::ordered_store_input::State::new(),
            ordered_entries: screens::ordered_entries::State::new(),
            ordered_value: screens::ordered_value::State::new(),
            memory_store_input: screens::memory_store_input::State::new(),
            memory_entries: screens::memory_entries::State::new(),
            memory_item_editing_id: String::new(),
            memory_item_ttl_seconds: 3600,
            which_key: update::Keys::new(
                update::build_keymap(),
                update::Scope::Menu,
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

    pub fn visible_universe_indices(&self) -> Vec<usize> {
        if self.universe_select.search.get_value().is_empty() {
            return (0..self.available_universes.len()).collect();
        }

        let needle = self.universe_select.search.get_value().to_lowercase();

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
            || !self.entries.marked.is_empty()
            || !self.ordered_entries.marked.is_empty()
            || !self.memory_entries.marked.is_empty()
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

    pub fn visible_entry_indices(&self) -> Vec<usize> {
        self.entries.visible_indices()
    }

    pub(crate) fn current_entry_index(&self) -> Option<usize> {
        self.entries.current_index()
    }

    pub(crate) fn current_entry_scope_key(&self) -> Option<(String, String)> {
        self.entries.current_scope_key()
    }

    pub(crate) fn resolve_entry_usernames(&mut self) {
        self.entries.resolve_usernames();
    }

    pub fn visible_ordered_entry_indices(&self) -> Vec<usize> {
        self.ordered_entries.visible_indices()
    }

    pub fn visible_memory_item_indices(&self) -> Vec<usize> {
        self.memory_entries.visible_indices()
    }

    pub fn enter_tree_mode(&mut self) {
        self.enter_tree_mode_for(TreeTarget::Value);
    }

    pub fn enter_tree_mode_for(&mut self, target: TreeTarget) {
        let source = match target {
            TreeTarget::Value => self.value.text.clone(),
            TreeTarget::EntriesCreate => self.entries.create_value.get_value().to_string(),
            TreeTarget::MemoryCreate => self.memory_entries.create_value.get_value().to_string(),
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
                self.status = status::json_error(err);
            }
        }
    }

    pub fn exit_tree_mode(&mut self) {
        self.tree_editor = None;
        self.pending_confirm = None;
        self.status.clear();
    }
}
