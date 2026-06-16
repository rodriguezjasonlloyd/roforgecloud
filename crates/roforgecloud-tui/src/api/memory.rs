use roforgecloud_core::opencloud::ListQuery;

use crate::app::{App, TextFieldExt, Screen, ValueSource};

impl App {
    pub async fn load_memory_items(&mut self) {
        self.memory_entries.page_tokens = vec![None];
        self.load_memory_items_page().await;
    }

    pub async fn load_next_memory_items_page(&mut self) {
        let Some(token) = self.memory_entries.next_page_token.clone() else {
            return;
        };
        self.memory_entries.page_tokens.push(Some(token));
        self.load_memory_items_page().await;
    }

    pub async fn load_prev_memory_items_page(&mut self) {
        if self.memory_entries.page_tokens.len() <= 1 {
            return;
        }
        self.memory_entries.page_tokens.pop();
        self.load_memory_items_page().await;
    }

    pub async fn load_memory_items_page(&mut self) {
        let page_token = self.memory_entries.page_tokens.last().cloned().flatten();

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
                self.memory_entries.items = result.items;
                self.memory_entries.selected = 0;
                self.memory_entries.marked.clear();
                self.memory_entries.next_page_token =
                    result.next_page_token.filter(|t| !t.is_empty());
                let page = self.memory_entries.page_tokens.len();
                self.status = format!("{} items (page {page})", self.memory_entries.items.len());
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
        self.memory_entries.items = all;
        self.memory_entries.selected = 0;
        self.memory_entries.marked.clear();
        self.memory_entries.next_page_token = None;
        self.memory_entries.page_tokens = vec![None];
        self.status = format!(
            "{} items (search across whole sorted map)",
            self.memory_entries.items.len()
        );
    }

    pub async fn load_memory_value(&mut self) {
        let Some(index) = self.memory_entries.current_index() else {
            return;
        };
        let id = self.memory_entries.items[index].id.clone();

        self.status = "loading value...".to_string();
        match self
            .client
            .get_sorted_map_item(self.universe_id, &self.memory_store_input.id, &id)
            .await
        {
            Ok(item) => {
                let expire = item.expire_time.clone().unwrap_or_else(|| "—".to_string());
                self.value.title =
                    format!("{}/{id} (expires: {expire})", self.memory_store_input.id);
                self.value.text = serde_json::to_string_pretty(&item.value).unwrap_or_default();
                self.value.revision = item.etag;
                self.value.scroll = 0;
                self.tree_editor = None;
                self.value.source = ValueSource::MemoryStoreSortedMap;
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

    pub async fn save_memory_value(&mut self) {
        let value: serde_json::Value = match serde_json::from_str(&self.value.edit_text) {
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
                self.value.revision.as_deref(),
            )
            .await
        {
            Ok(item) => {
                self.value.text = serde_json::to_string_pretty(&item.value).unwrap_or_default();
                self.value.revision = item.etag.clone();
                self.value.scroll = 0;
                if let Some(cached) = self.memory_entries.items.iter_mut().find(|i| i.id == item.id) {
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

    pub async fn create_memory_item(&mut self) {
        let id = self.memory_entries.create_id.get_value().trim();
        if id.is_empty() || id.len() > 63 {
            self.status = "item id must be 1-63 characters".to_string();
            return;
        }
        let value: serde_json::Value = match serde_json::from_str(self.memory_entries.create_value.get_value()) {
            Ok(value) => value,
            Err(err) => {
                self.status = format!("invalid JSON: {err}");
                return;
            }
        };
        let ttl: u64 = match self.memory_entries.create_ttl.get_value().parse() {
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
                self.memory_entries.create_id.clear();
                self.memory_entries.create_value.clear();
                self.memory_entries.create_ttl.set_value("3600");
                self.memory_entries.create_active = false;
                self.status = "created".to_string();
                self.load_memory_items_page().await;
            }
            Err(err) => {
                self.status = self.datastore_error(err);
            }
        }
    }

    pub async fn delete_memory_item(&mut self) {
        let Some(index) = self.memory_entries.current_index() else {
            return;
        };
        let id = self.memory_entries.items[index].id.clone();

        self.status = "deleting...".to_string();
        match self
            .client
            .delete_sorted_map_item(self.universe_id, &self.memory_store_input.id, &id)
            .await
        {
            Ok(()) => {
                self.memory_entries.items.remove(index);
                let visible = self.visible_memory_item_indices().len();
                if self.memory_entries.selected >= visible {
                    self.memory_entries.selected = visible.saturating_sub(1);
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
        let mut indices: Vec<usize> = self.memory_entries.marked.iter().copied().collect();
        indices.sort_unstable();

        let total = indices.len();
        if total == 0 {
            self.status = "no items to delete".to_string();
            return;
        }

        let mut deleted_indices = Vec::new();
        let mut errors = 0;

        for &i in &indices {
            let id = self.memory_entries.items[i].id.clone();
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
            self.memory_entries.items.remove(i);
        }

        self.memory_entries.marked.clear();

        let visible = self.visible_memory_item_indices().len();
        if self.memory_entries.selected >= visible {
            self.memory_entries.selected = visible.saturating_sub(1);
        }

        self.status = if errors == 0 {
            format!("deleted {} items", deleted_indices.len())
        } else {
            format!("deleted {} items, {errors} failed", deleted_indices.len())
        };
    }

    pub async fn save_memory_ttl(&mut self) {
        let Some(index) = self.memory_entries.current_index() else {
            return;
        };
        let item = self.memory_entries.items[index].clone();

        let ttl: u64 = match self.memory_entries.ttl_edit.get_value().parse() {
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
                self.memory_entries.items[index].etag = updated.etag;
                self.memory_entries.items[index].expire_time = updated.expire_time;
                self.memory_entries.ttl_editing = false;
                self.status = "saved".to_string();
            }
            Err(err) => {
                self.status = self.datastore_error(err);
            }
        }
    }
}
