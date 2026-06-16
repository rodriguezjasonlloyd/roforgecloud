pub const LOADING: &str = "loading...";
pub const SAVING: &str = "saving...";
pub const DELETING: &str = "deleting...";
pub const CREATING: &str = "creating...";
pub const INCREMENTING: &str = "incrementing...";
pub const PUBLISHING: &str = "publishing...";
pub const RESTORING: &str = "restoring...";
pub const FETCHING: &str = "fetching...";
pub const OPENING_BROWSER: &str = "opening browser for login...";

pub const SAVED: &str = "saved";
pub const DELETED: &str = "deleted";
pub const CREATED: &str = "created";
pub const INCREMENTED: &str = "incremented";
pub const LOGGED_IN: &str = "logged in";
pub const LOGGED_OUT: &str = "logged out";

pub const INVALID_NUMBER: &str = "invalid number";
pub const INVALID_TTL: &str = "invalid ttl";
pub const NO_ENTRIES_TO_DELETE: &str = "no entries to delete";
pub const NO_ITEMS_TO_DELETE: &str = "no items to delete";
pub const CONFLICT: &str = "conflict: value changed on server — reloaded latest, your edit was discarded";
pub const ID_EMPTY: &str = "entry id cannot be empty";
pub const STORE_DELETED: &str = "data store scheduled for deletion";
pub const STORE_RESTORED: &str = "data store restored";
pub const ID_TOO_LONG: &str = "id must be 1-63 characters";
pub const TOPIC_EMPTY: &str = "topic cannot be empty";
pub const NO_UNIVERSES: &str = "no authorized universes found for this token";
pub const OAUTH_NOT_CONFIGURED: &str =
    "OAuth not configured: set ROFORGE_OAUTH_CLIENT_ID/ROFORGE_OAUTH_CLIENT_SECRET";
pub const NO_CHANGES: &str = "no changes";
pub const EXPECT_SINGLE_KEY_OBJ: &str = "error: expected a single-key JSON object";
pub const ID_FIELD_REQUIRED: &str = "error: \"id\" field must be a non-empty string";
pub const VALUE_FIELD_NUMBER: &str = "error: \"value\" field must be a number";

pub fn editor_launch_error(editor: &str, err: impl std::fmt::Display) -> String {
    format!("error: failed to launch '{editor}': {err}")
}

pub fn editor_exit_error(editor: &str, code: impl std::fmt::Display) -> String {
    format!("error: '{editor}' exited with {code}")
}

pub fn editor_json_error(err: impl std::fmt::Display) -> String {
    format!("error: invalid JSON: {err}")
}

pub fn api_error(err: impl std::fmt::Display) -> String {
    format!("error: {err}")
}

pub fn json_error(err: impl std::fmt::Display) -> String {
    format!("invalid JSON: {err}")
}

pub fn published(topic: &str) -> String {
    format!("published to '{topic}'")
}

pub fn store_count(n: usize) -> String {
    format!("{n} data stores")
}

pub fn bulk_progress(done: usize, total: usize, verb: &str) -> String {
    format!("{verb} {done}/{total}...")
}

pub fn bulk_result(count: usize, failed: usize, noun: &str, verb: &str) -> String {
    if failed == 0 {
        format!("{verb} {count} {noun}")
    } else {
        format!("{verb} {count} {noun}, {failed} failed")
    }
}

pub fn page_count(n: usize, noun: &str, page: usize) -> String {
    format!("{n} {noun} (page {page})")
}

pub fn search_count(n: usize, noun: &str) -> String {
    format!("{n} {noun} (search across whole store)")
}

pub fn loading_search(noun: &str) -> String {
    format!("loading all {noun} for search...")
}
