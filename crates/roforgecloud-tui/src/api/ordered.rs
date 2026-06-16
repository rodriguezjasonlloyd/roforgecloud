use roforgecloud_core::opencloud::ListQuery;

use crate::app::{App, TextFieldExt, Screen};
use crate::status;

impl App {
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

        self.status = status::LOADING.to_string();
        match self
            .client
            .list_ordered_entries(
                self.universe_id,
                self.ordered_store_input.store_id.get_value(),
                self.ordered_store_input.scope.get_value(),
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
                self.status = status::page_count(self.ordered_entries.items.len(), "entries", page);
            }
            Err(err) => {
                self.status = self.datastore_error(err);
            }
        }
    }

    pub async fn load_all_ordered_entries_for_search(&mut self) {
        self.status = status::loading_search("ordered entries");
        let mut all = Vec::new();
        let mut page_token: Option<String> = None;
        loop {
            match self
                .client
                .list_ordered_entries(
                    self.universe_id,
                    self.ordered_store_input.store_id.get_value(),
                    self.ordered_store_input.scope.get_value(),
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
        self.status = status::search_count(self.ordered_entries.items.len(), "entries");
    }

    pub async fn load_ordered_value(&mut self) {
        let Some(index) = self.ordered_entries.current_index() else {
            return;
        };
        let id = self.ordered_entries.items[index].id.clone();

        self.status = status::LOADING.to_string();
        match self
            .client
            .get_ordered_entry(
                self.universe_id,
                self.ordered_store_input.store_id.get_value(),
                self.ordered_store_input.scope.get_value(),
                &id,
            )
            .await
        {
            Ok(entry) => {
                self.ordered_value.title = format!(
                    "{}/{id} (scope: {})",
                    self.ordered_store_input.store_id.get_value(), self.ordered_store_input.scope.get_value()
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
                self.status = status::INVALID_NUMBER.to_string();
                return;
            }
        };

        self.status = status::SAVING.to_string();
        match self
            .client
            .update_ordered_entry(
                self.universe_id,
                self.ordered_store_input.store_id.get_value(),
                self.ordered_store_input.scope.get_value(),
                &id,
                value,
            )
            .await
        {
            Ok(entry) => {
                self.ordered_value.value = entry.value;
                self.ordered_entries.items[index].value = entry.value;
                self.ordered_value.editing = false;
                self.status = status::SAVED.to_string();
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
                self.status = status::INVALID_NUMBER.to_string();
                return;
            }
        };

        self.status = status::INCREMENTING.to_string();
        match self
            .client
            .increment_ordered_entry(
                self.universe_id,
                self.ordered_store_input.store_id.get_value(),
                self.ordered_store_input.scope.get_value(),
                &id,
                amount,
            )
            .await
        {
            Ok(entry) => {
                self.ordered_value.value = entry.value;
                self.ordered_entries.items[index].value = entry.value;
                self.ordered_value.increment_editing = false;
                self.status = status::INCREMENTED.to_string();
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

        self.status = status::DELETING.to_string();
        match self
            .client
            .delete_ordered_entry(
                self.universe_id,
                self.ordered_store_input.store_id.get_value(),
                self.ordered_store_input.scope.get_value(),
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
                self.status = status::DELETED.to_string();
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
            self.status = status::NO_ENTRIES_TO_DELETE.to_string();
            return;
        }

        let mut deleted_indices = Vec::new();
        let mut errors = 0;

        for &i in &indices {
            let id = self.ordered_entries.items[i].id.clone();
            self.status = status::bulk_progress(deleted_indices.len() + errors + 1, total, "deleting");
            match self
                .client
                .delete_ordered_entry(
                    self.universe_id,
                    self.ordered_store_input.store_id.get_value(),
                    self.ordered_store_input.scope.get_value(),
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

        self.status = status::bulk_result(deleted_indices.len(), errors, "entries", "deleted");
    }

    pub async fn create_ordered_entry(&mut self) {
        let id = self.ordered_entries.create_id.get_value().trim();
        if id.is_empty() || id.len() > 63 {
            self.status = status::ID_TOO_LONG.to_string();
            return;
        }
        let value: f64 = match self.ordered_entries.create_value.get_value().parse() {
            Ok(value) => value,
            Err(_) => {
                self.status = status::INVALID_NUMBER.to_string();
                return;
            }
        };

        self.status = status::CREATING.to_string();
        match self
            .client
            .create_ordered_entry(
                self.universe_id,
                self.ordered_store_input.store_id.get_value(),
                self.ordered_store_input.scope.get_value(),
                id,
                value,
            )
            .await
        {
            Ok(_) => {
                self.ordered_entries.create_id.clear();
                self.ordered_entries.create_value.clear();
                self.ordered_entries.create_active = false;
                self.status = status::CREATED.to_string();
                self.load_ordered_entries_page().await;
            }
            Err(err) => {
                self.status = self.datastore_error(err);
            }
        }
    }
}
