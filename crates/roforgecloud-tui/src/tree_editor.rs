use tui_textarea::TextArea;

use crate::app::TextField;
use crate::json_tree::{flatten, JsonNode, JsonNodeValue};

pub struct TreeEditor {
    root: JsonNode,
    cursor: usize,
    edit_text: Option<TextArea<'static>>,
    editing_key: bool,
    edit_key: TextField,
    adding: bool,
    pending_leader: bool,
    dirty: bool,
}

impl TreeEditor {
    pub fn new(value: &serde_json::Value) -> Self {
        let mut root = JsonNode::from_value(value);
        root.collapse_below_root();
        Self {
            root,
            cursor: 0,
            edit_text: None,
            editing_key: false,
            edit_key: TextField::default(),
            adding: false,
            pending_leader: false,
            dirty: false,
        }
    }

    pub fn root(&self) -> &JsonNode {
        &self.root
    }

    pub fn to_value(&self) -> serde_json::Value {
        self.root.to_value()
    }

    pub fn cursor(&self) -> usize {
        self.cursor
    }

    pub fn set_cursor(&mut self, cursor: usize) {
        let len = flatten(&self.root).len();
        self.cursor = cursor.min(len.saturating_sub(1));
    }

    pub fn dirty(&self) -> bool {
        self.dirty
    }

    pub fn is_editing(&self) -> bool {
        self.edit_text.is_some() || self.editing_key
    }

    pub fn editing(&self) -> bool {
        self.edit_text.is_some()
    }

    pub fn editing_key(&self) -> bool {
        self.editing_key
    }

    pub fn edit_text(&self) -> &TextArea<'static> {
        self.edit_text.as_ref().unwrap()
    }

    pub fn edit_text_mut(&mut self) -> &mut TextArea<'static> {
        self.edit_text.as_mut().unwrap()
    }

    pub fn edit_key(&self) -> &TextField {
        &self.edit_key
    }

    pub fn edit_key_mut(&mut self) -> &mut TextField {
        &mut self.edit_key
    }

    pub fn pending_leader(&self) -> bool {
        self.pending_leader
    }

    pub fn set_pending_leader(&mut self, value: bool) {
        self.pending_leader = value;
    }

    pub fn move_cursor(&mut self, delta: isize) {
        let len = flatten(&self.root).len();
        if len == 0 {
            return;
        }
        let cursor = self.cursor as isize + delta;
        self.cursor = cursor.clamp(0, len as isize - 1) as usize;
    }

    pub fn toggle(&mut self) {
        let rows = flatten(&self.root);
        let Some(current) = rows.get(self.cursor) else {
            return;
        };

        if !current.is_closing && !current.is_container {
            return;
        }
        let path = current.path.clone();

        if let Some(node) = self.root.get_mut(&path) {
            node.collapsed = !node.collapsed;
        }

        if current.is_closing {
            let rows = flatten(&self.root);
            if let Some(idx) = rows
                .iter()
                .position(|r| r.path == path && r.is_container && !r.is_closing)
            {
                self.cursor = idx;
            }
        }
    }

    pub fn edit_leaf(&mut self) {
        let rows = flatten(&self.root);
        let Some(current) = rows.get(self.cursor) else {
            return;
        };

        if current.is_leaf {
            self.edit_text = Some(if self.adding {
                TextArea::default()
            } else {
                let lines: Vec<String> = current.preview.lines().map(String::from).collect();
                TextArea::from(lines)
            });
        }
    }

    pub fn confirm_edit(&mut self) {
        let rows = flatten(&self.root);
        let Some(current) = rows.get(self.cursor) else {
            self.edit_text = None;
            return;
        };

        let Some(textarea) = self.edit_text.take() else {
            return;
        };
        let text = textarea.lines().join("\n");
        let value = serde_json::from_str::<serde_json::Value>(&text)
            .unwrap_or_else(|_| serde_json::Value::String(text));

        if let Some(node) = self.root.get_mut(&current.path) {
            node.value = JsonNodeValue::Leaf(value);
        }
        self.dirty = true;
        self.adding = false;
    }

    pub fn yank(&self, clipboard: &mut Option<arboard::Clipboard>) -> Option<String> {
        let value = self.current_value()?;
        let text = serde_json::to_string_pretty(&value).unwrap_or_default();
        let clipboard = clipboard.as_mut()?;
        clipboard.set_text(&text).ok()?;
        Some("yanked".to_string())
    }

    pub fn paste(&mut self, clipboard: &mut Option<arboard::Clipboard>) -> Option<String> {
        let clipboard = clipboard.as_mut()?;
        let text = clipboard.get_text().ok()?;
        let value = serde_json::from_str::<serde_json::Value>(&text)
            .unwrap_or_else(|_| serde_json::Value::String(text));
        self.replace_value(None, value);
        Some("pasted".to_string())
    }

    pub fn current_value(&self) -> Option<serde_json::Value> {
        let rows = flatten(&self.root);
        let current = rows.get(self.cursor)?;
        Some(self.root.get(&current.path)?.to_value())
    }

    pub fn replace_value(&mut self, key: Option<String>, value: serde_json::Value) {
        let rows = flatten(&self.root);
        let Some(current) = rows.get(self.cursor) else {
            return;
        };
        let path = current.path.clone();
        if let Some(node) = self.root.get_mut(&path) {
            let mut new_node = JsonNode::from_value(&value);
            new_node.collapse_below_root();
            new_node.key = key.or_else(|| node.key.clone());
            *node = new_node;
        }
        self.dirty = true;
        let len = flatten(&self.root).len();
        if self.cursor >= len {
            self.cursor = len.saturating_sub(1);
        }
    }

    pub fn cancel_edit(&mut self) {
        self.edit_text = None;
        self.editing_key = false;
        self.edit_key.clear();
        if self.adding {
            self.remove_current();
            self.adding = false;
        }
    }

    pub fn add_entry(&mut self) {
        let rows = flatten(&self.root);
        let Some(current) = rows.get(self.cursor) else {
            return;
        };

        let expanded_container = current.is_container
            && !current.is_closing
            && !self
                .root
                .get_mut(&current.path)
                .is_some_and(|n| n.collapsed);

        let (parent_path, insert_index) = if expanded_container {
            (current.path.clone(), 0)
        } else if current.path.is_empty() {
            (Vec::new(), 0)
        } else {
            let mut parent_path = current.path.clone();
            let idx = parent_path.pop().unwrap();
            (parent_path, idx + 1)
        };

        let Some(parent) = self.root.get_mut(&parent_path) else {
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
            self.edit_key.clear();
            self.editing_key = true;
        } else {
            items.insert(
                insert_index,
                JsonNode {
                    key: None,
                    value: JsonNodeValue::Leaf(serde_json::Value::String(String::new())),
                    collapsed: false,
                },
            );
            self.edit_text = Some(TextArea::default());
        }

        self.adding = true;
        self.dirty = true;

        let mut new_path = parent_path;
        new_path.push(insert_index);
        let rows = flatten(&self.root);
        if let Some(idx) = rows
            .iter()
            .position(|r| r.path == new_path && !r.is_closing)
        {
            self.cursor = idx;
        }
    }

    pub fn edit_key_start(&mut self) {
        let rows = flatten(&self.root);
        let Some(current) = rows.get(self.cursor) else {
            return;
        };
        let Some(key) = &current.key else {
            return;
        };
        self.edit_key.set(key.clone());
        self.editing_key = true;
    }

    pub fn confirm_key(&mut self) {
        let rows = flatten(&self.root);
        let Some(current) = rows.get(self.cursor) else {
            return;
        };
        let path = current.path.clone();
        if let Some(node) = self.root.get_mut(&path) {
            node.key = Some(self.edit_key.value.clone());
        }
        self.editing_key = false;
        self.edit_key.clear();
        self.dirty = true;
        if self.adding {
            self.edit_text = Some(TextArea::default());
        }
    }

    fn remove_current(&mut self) {
        let rows = flatten(&self.root);
        let Some(current) = rows.get(self.cursor) else {
            return;
        };
        let mut path = current.path.clone();
        if path.is_empty() {
            return;
        }
        let idx = path.pop().unwrap();
        if let Some(parent) = self.root.get_mut(&path) {
            match &mut parent.value {
                JsonNodeValue::Array(items) | JsonNodeValue::Object(items) => {
                    if idx < items.len() {
                        items.remove(idx);
                    }
                }
                JsonNodeValue::Leaf(_) => {}
            }
        }
        let len = flatten(&self.root).len();
        if self.cursor >= len {
            self.cursor = len.saturating_sub(1);
        }
    }

    pub fn delete_current(&mut self) {
        let rows = flatten(&self.root);
        let Some(current) = rows.get(self.cursor) else {
            return;
        };
        if current.path.is_empty() {
            return;
        }
        self.remove_current();
        self.dirty = true;
    }
}
