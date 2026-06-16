pub const LOADING: &str = "loading...";
pub const SAVING: &str = "saving...";
pub const DELETING: &str = "deleting...";
pub const CREATING: &str = "creating...";
pub const INCREMENTING: &str = "incrementing...";
pub const PUBLISHING: &str = "publishing...";

pub const SAVED: &str = "saved";
pub const DELETED: &str = "deleted";
pub const CREATED: &str = "created";
pub const INCREMENTED: &str = "incremented";

pub const INVALID_NUMBER: &str = "invalid number";
pub const INVALID_TTL: &str = "invalid ttl";
pub const NO_ENTRIES_TO_DELETE: &str = "no entries to delete";
pub const NO_ITEMS_TO_DELETE: &str = "no items to delete";
pub const CONFLICT: &str = "conflict: value changed on server — reloaded latest, your edit was discarded";

pub fn api_error(err: impl std::fmt::Display) -> String {
    format!("error: {err}")
}

pub fn json_error(err: impl std::fmt::Display) -> String {
    format!("invalid JSON: {err}")
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
