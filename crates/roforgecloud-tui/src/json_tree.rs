#[derive(Debug, Clone)]
pub struct JsonNode {
    pub key: Option<String>,
    pub value: JsonNodeValue,
    pub collapsed: bool,
}

#[derive(Debug, Clone)]
pub enum JsonNodeValue {
    Leaf(serde_json::Value),
    Array(Vec<JsonNode>),
    Object(Vec<JsonNode>),
}

impl JsonNode {
    pub fn from_value(value: &serde_json::Value) -> Self {
        Self::build(None, value)
    }

    fn build(key: Option<String>, value: &serde_json::Value) -> Self {
        let value = match value {
            serde_json::Value::Object(map) => JsonNodeValue::Object(
                map.iter()
                    .map(|(k, v)| Self::build(Some(k.clone()), v))
                    .collect(),
            ),
            serde_json::Value::Array(items) => {
                JsonNodeValue::Array(items.iter().map(|v| Self::build(None, v)).collect())
            }
            other => JsonNodeValue::Leaf(other.clone()),
        };
        JsonNode {
            key,
            value,
            collapsed: false,
        }
    }

    pub fn to_value(&self) -> serde_json::Value {
        match &self.value {
            JsonNodeValue::Leaf(v) => v.clone(),
            JsonNodeValue::Array(items) => {
                serde_json::Value::Array(items.iter().map(JsonNode::to_value).collect())
            }
            JsonNodeValue::Object(items) => serde_json::Value::Object(
                items
                    .iter()
                    .map(|n| (n.key.clone().unwrap_or_default(), n.to_value()))
                    .collect(),
            ),
        }
    }

    /// Collapses every container below the root, leaving the root's direct
    /// entries visible but their contents folded.
    pub fn collapse_below_root(&mut self) {
        match &mut self.value {
            JsonNodeValue::Array(items) | JsonNodeValue::Object(items) => {
                for child in items.iter_mut() {
                    child.collapse_all();
                }
            }
            JsonNodeValue::Leaf(_) => {}
        }
    }

    fn collapse_all(&mut self) {
        match &mut self.value {
            JsonNodeValue::Array(items) | JsonNodeValue::Object(items) => {
                self.collapsed = true;
                for child in items.iter_mut() {
                    child.collapse_all();
                }
            }
            JsonNodeValue::Leaf(_) => {}
        }
    }

    pub fn get_mut(&mut self, path: &[usize]) -> Option<&mut JsonNode> {
        let mut node = self;
        for &idx in path {
            node = match &mut node.value {
                JsonNodeValue::Array(items) => items.get_mut(idx)?,
                JsonNodeValue::Object(items) => items.get_mut(idx)?,
                JsonNodeValue::Leaf(_) => return None,
            };
        }
        Some(node)
    }
}

#[derive(Debug, Clone)]
pub struct FlatRow {
    pub depth: usize,
    pub path: Vec<usize>,
    pub key: Option<String>,
    pub preview: String,
    pub is_container: bool,
    pub is_leaf: bool,
    pub is_closing: bool,
}

pub fn flatten(root: &JsonNode) -> Vec<FlatRow> {
    let mut rows = Vec::new();
    let mut path = Vec::new();
    flatten_node(root, 0, &mut path, &mut rows);
    rows
}

fn flatten_node(node: &JsonNode, depth: usize, path: &mut Vec<usize>, rows: &mut Vec<FlatRow>) {
    match &node.value {
        JsonNodeValue::Leaf(value) => {
            rows.push(FlatRow {
                depth,
                path: path.clone(),
                key: node.key.clone(),
                preview: format_scalar(value),
                is_container: false,
                is_leaf: true,
                is_closing: false,
            });
        }
        JsonNodeValue::Array(items) | JsonNodeValue::Object(items) => {
            let (open, close) = match &node.value {
                JsonNodeValue::Array(_) => ("[", "]"),
                _ => ("{", "}"),
            };
            if node.collapsed {
                rows.push(FlatRow {
                    depth,
                    path: path.clone(),
                    key: node.key.clone(),
                    preview: format!("{open}…{close} ({} items)", items.len()),
                    is_container: true,
                    is_leaf: false,
                    is_closing: false,
                });
            } else {
                rows.push(FlatRow {
                    depth,
                    path: path.clone(),
                    key: node.key.clone(),
                    preview: open.to_string(),
                    is_container: true,
                    is_leaf: false,
                    is_closing: false,
                });
                for (i, child) in items.iter().enumerate() {
                    path.push(i);
                    flatten_node(child, depth + 1, path, rows);
                    path.pop();
                }
                rows.push(FlatRow {
                    depth,
                    path: path.clone(),
                    key: None,
                    preview: close.to_string(),
                    is_container: false,
                    is_leaf: false,
                    is_closing: true,
                });
            }
        }
    }
}

fn format_scalar(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::String(s) => format!("{s:?}"),
        serde_json::Value::Null => "null".to_string(),
        other => other.to_string(),
    }
}
