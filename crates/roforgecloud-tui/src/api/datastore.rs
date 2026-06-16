use roforgecloud_core::opencloud::ListQuery;

use crate::app::{App, TextFieldExt, Screen, TreeTarget, ValueSource};
use crate::status;
use crate::tree_editor::TreeEditor;

impl App {
    pub(crate) fn datastore_error(&self, err: roforgecloud_core::error::Error) -> status::Msg {
        if !self.has_api_key
            && matches!(&err, roforgecloud_core::error::Error::Api { status, .. } if status.as_u16() == 401)
        {
            status::datastore_needs_api_key()
        } else {
            status::api_error(err)
        }
    }

    pub async fn load_stores(&mut self) {
        self.resolve_current_universe_name();
        self.status = status::loading();
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
                self.status = status::store_count(self.stores.items.len());
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

        self.status = status::deleting();
        match self
            .client
            .delete_data_store(self.universe_id, &store.id)
            .await
        {
            Ok(info) => {
                self.status = status::store_deleted();
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

        self.status = status::restoring();
        match self
            .client
            .undelete_data_store(self.universe_id, &store.id)
            .await
        {
            Ok(info) => {
                self.status = status::store_restored();
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
            self.status = status::bulk_progress(n + 1, total, "scheduling deletion");
            match self.client.delete_data_store(self.universe_id, &id).await {
                Ok(info) => self.stores.items[i] = info,
                Err(_) => errors += 1,
            }
        }

        self.stores.marked.clear();
        self.status = status::bulk_result(total - errors, errors, "data stores", "scheduled for deletion");
    }

    pub async fn bulk_undelete_data_stores(&mut self) {
        let mut indices: Vec<usize> = self.stores.marked.iter().copied().collect();
        indices.sort_unstable();

        let total = indices.len();
        let mut errors = 0;

        for (n, &i) in indices.iter().enumerate() {
            let id = self.stores.items[i].id.clone();
            self.status = status::bulk_progress(n + 1, total, "restoring");
            match self.client.undelete_data_store(self.universe_id, &id).await {
                Ok(info) => self.stores.items[i] = info,
                Err(_) => errors += 1,
            }
        }

        self.stores.marked.clear();
        self.status = status::bulk_result(total - errors, errors, "data stores", "restored");
    }

    pub async fn load_entries(&mut self) {
        self.entries.page_tokens = vec![None];
        self.load_entries_page().await;
    }

    pub async fn load_next_entries_page(&mut self) {
        let Some(token) = self.entries.next_page_token.clone() else {
            return;
        };
        self.entries.page_tokens.push(Some(token));
        self.load_entries_page().await;
    }

    pub async fn load_prev_entries_page(&mut self) {
        if self.entries.page_tokens.len() <= 1 {
            return;
        }
        self.entries.page_tokens.pop();
        self.load_entries_page().await;
    }

    pub async fn load_entries_page(&mut self) {
        let page_token = self.entries.page_tokens.last().cloned().flatten();

        self.status = status::loading();
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
                self.entries.items = result.data_store_entries;
                self.entries.selected = 0;
                self.entries.marked.clear();
                self.entries.next_page_token = result.next_page_token;
                let page = self.entries.page_tokens.len();
                self.status = status::page_count(self.entries.items.len(), "entries", page);
                self.resolve_entry_usernames();
            }
            Err(err) => {
                self.status = self.datastore_error(err);
            }
        }
    }

    pub async fn load_all_entries_for_search(&mut self) {
        self.status = status::loading_search("entries");
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
        self.entries.items = all;
        self.entries.selected = 0;
        self.entries.marked.clear();
        self.entries.next_page_token = None;
        self.entries.page_tokens = vec![None];
        self.status = status::search_count(self.entries.items.len(), "entries");
        self.resolve_entry_usernames();
    }

    pub async fn load_value(&mut self) {
        if self.value.source == ValueSource::MemoryStoreSortedMap {
            return self.load_memory_value().await;
        }

        let Some((scope, key)) = self.current_entry_scope_key() else {
            return;
        };

        self.status = status::loading();
        match self
            .client
            .get_entry_with_revision(self.universe_id, &self.stores.data_store_id, &key, Some(&scope))
            .await
        {
            Ok((value, revision)) => {
                self.value.title = key.to_string();
                self.value.scope = format!("scope: {scope}");
                self.value.text = serde_json::to_string_pretty(&value).unwrap_or_default();
                self.value.revision = revision;
                self.value.scroll = 0;
                self.tree_editor = None;
                self.value.source = ValueSource::DataStore;
                self.status.clear();
            }
            Err(err) => {
                self.status = self.datastore_error(err);
            }
        }
    }

    pub async fn refresh_tree(&mut self) {
        let cursor = self.tree_editor.as_ref().map(|t| t.cursor()).unwrap_or(0);
        self.load_value().await;
        if self.status.text.is_empty() {
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
                self.value.edit_text = json;
                self.save_value().await;
                if let Ok(value) = serde_json::from_str::<serde_json::Value>(&self.value.text) {
                    self.tree_editor = Some(TreeEditor::new(&value));
                    self.pending_confirm = None;
                }
            }
            TreeTarget::EntriesCreate => {
                self.entries.create_value.set_value(json);
                self.exit_tree_mode();
            }
            TreeTarget::MemoryCreate => {
                self.memory_entries.create_value.set_value(json);
                self.exit_tree_mode();
            }
        }
    }

    pub async fn save_value(&mut self) {
        if self.value.source == ValueSource::MemoryStoreSortedMap {
            return self.save_memory_value().await;
        }

        let Some((scope, key)) = self.current_entry_scope_key() else {
            return;
        };

        let value: serde_json::Value = match serde_json::from_str(&self.value.edit_text) {
            Ok(value) => value,
            Err(err) => {
                self.status = status::json_error(err);
                return;
            }
        };

        self.status = status::saving();
        match self
            .client
            .set_entry(
                self.universe_id,
                &self.stores.data_store_id,
                &key,
                Some(&scope),
                &value,
                self.value.revision.as_deref(),
            )
            .await
        {
            Ok(revision) => {
                self.value.text = serde_json::to_string_pretty(&value).unwrap_or_default();
                self.value.revision = revision;
                self.value.scroll = 0;
                self.status = status::saved();
            }
            Err(err) if matches!(&err, roforgecloud_core::error::Error::Api { status, .. } if status.as_u16() == 409 || status.as_u16() == 412) =>
            {
                self.load_value().await;
                self.status = status::conflict();
            }
            Err(err) => {
                self.status = self.datastore_error(err);
            }
        }
    }

    pub async fn create_entry(&mut self) {
        let id = self.entries.create_id.get_value().trim();
        if id.is_empty() {
            self.status = status::id_empty();
            return;
        }
        let (scope, key) = match id.split_once('/') {
            Some((scope, key)) => (scope.to_string(), key.to_string()),
            None => ("global".to_string(), id.to_string()),
        };

        let value: serde_json::Value = match serde_json::from_str(self.entries.create_value.get_value()) {
            Ok(value) => value,
            Err(err) => {
                self.status = status::json_error(err);
                return;
            }
        };

        self.status = status::creating();
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
                self.entries.create_id.clear();
                self.entries.create_value.clear();
                self.entries.create_active = false;
                self.status = status::created();
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

        self.status = status::deleting();
        match self
            .client
            .delete_entry(self.universe_id, &self.stores.data_store_id, &key, Some(&scope))
            .await
        {
            Ok(()) => {
                self.entries.items.remove(index);
                let visible = self.visible_entry_indices().len();
                if self.entries.selected >= visible {
                    self.entries.selected = visible.saturating_sub(1);
                }
                if self.screen == Screen::Value {
                    self.exit_tree_mode();
                    self.screen = Screen::Entries;
                }
                self.status = status::deleted();
            }
            Err(err) => {
                self.status = self.datastore_error(err);
            }
        }
    }

    pub async fn bulk_delete_entries(&mut self) {
        let mut indices: Vec<usize> = self.entries.marked.iter().copied().collect();
        indices.sort_unstable();

        let targets: Vec<(usize, String, String)> = indices
            .into_iter()
            .map(|i| {
                let entry = &self.entries.items[i];
                let (scope, key) = match entry.id.split_once('/') {
                    Some((scope, key)) => (scope.to_string(), key.to_string()),
                    None => ("global".to_string(), entry.id.clone()),
                };
                (i, scope, key)
            })
            .collect();

        if targets.is_empty() {
            self.status = status::no_entries_to_delete();
            return;
        }

        let total = targets.len();
        let mut deleted_indices = Vec::new();
        let mut errors = 0;

        for (i, scope, key) in &targets {
            self.status = status::bulk_progress(deleted_indices.len() + errors + 1, total, "deleting");
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
            self.entries.items.remove(i);
        }

        self.entries.marked.clear();

        let visible = self.visible_entry_indices().len();
        if self.entries.selected >= visible {
            self.entries.selected = visible.saturating_sub(1);
        }

        self.status = status::bulk_result(deleted_indices.len(), errors, "entries", "deleted");
    }
}
