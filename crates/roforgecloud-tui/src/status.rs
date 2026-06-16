#[derive(Default, Clone, Copy, PartialEq)]
pub enum Kind {
    #[default]
    Info,
    Ok,
    Err,
}

#[derive(Default, Clone)]
pub struct Msg {
    pub text: String,
    pub kind: Kind,
}

impl Msg {
    fn info(text: impl Into<String>) -> Self { Self { text: text.into(), kind: Kind::Info } }
    fn ok(text: impl Into<String>) -> Self { Self { text: text.into(), kind: Kind::Ok } }
    fn err(text: impl Into<String>) -> Self { Self { text: text.into(), kind: Kind::Err } }
    pub fn clear(&mut self) { *self = Self::default(); }
}

pub fn loading() -> Msg { Msg::info("loading...") }
pub fn saving() -> Msg { Msg::info("saving...") }
pub fn deleting() -> Msg { Msg::info("deleting...") }
pub fn creating() -> Msg { Msg::info("creating...") }
pub fn incrementing() -> Msg { Msg::info("incrementing...") }
pub fn publishing() -> Msg { Msg::info("publishing...") }
pub fn restoring() -> Msg { Msg::info("restoring...") }
pub fn fetching() -> Msg { Msg::info("fetching...") }
pub fn opening_browser() -> Msg { Msg::info("opening browser for login...") }

pub fn saved() -> Msg { Msg::ok("saved") }
pub fn deleted() -> Msg { Msg::ok("deleted") }
pub fn created() -> Msg { Msg::ok("created") }
pub fn incremented() -> Msg { Msg::ok("incremented") }
pub fn logged_in() -> Msg { Msg::ok("logged in") }
pub fn logged_out() -> Msg { Msg::ok("logged out") }
pub fn yanked() -> Msg { Msg::ok("yanked") }
pub fn pasted() -> Msg { Msg::ok("pasted") }
pub fn store_deleted() -> Msg { Msg::ok("data store scheduled for deletion") }
pub fn store_restored() -> Msg { Msg::ok("data store restored") }
pub fn no_changes() -> Msg { Msg::ok("no changes") }

pub fn invalid_number() -> Msg { Msg::err("invalid number") }
pub fn invalid_ttl() -> Msg { Msg::err("invalid TTL") }
pub fn no_entries_to_delete() -> Msg { Msg::err("no entries to delete") }
pub fn no_items_to_delete() -> Msg { Msg::err("no items to delete") }
pub fn conflict() -> Msg { Msg::err("conflict: value changed on server — reloaded latest, your edit was discarded") }
pub fn id_empty() -> Msg { Msg::err("entry id cannot be empty") }
pub fn id_too_long() -> Msg { Msg::err("id must be 1-63 characters") }
pub fn topic_empty() -> Msg { Msg::err("topic cannot be empty") }
pub fn no_universes() -> Msg { Msg::err("no authorized universes found for this token") }
pub fn oauth_not_configured() -> Msg { Msg::err("OAuth not configured: set ROFORGE_OAUTH_CLIENT_ID/ROFORGE_OAUTH_CLIENT_SECRET") }
pub fn datastore_needs_api_key() -> Msg { Msg::err("Data Stores need an API key — OAuth tokens aren't accepted here. Set ROFORGE_API_KEY and restart.") }
pub fn expect_single_key_obj() -> Msg { Msg::err("expected a single-key JSON object") }
pub fn id_field_required() -> Msg { Msg::err("\"id\" field must be a non-empty string") }
pub fn value_field_number() -> Msg { Msg::err("\"value\" field must be a number") }

pub fn api_error(err: impl std::fmt::Display) -> Msg { Msg::err(format!("{err}")) }
pub fn json_error(err: impl std::fmt::Display) -> Msg { Msg::err(format!("invalid JSON: {err}")) }
pub fn published(topic: &str) -> Msg { Msg::ok(format!("published to '{topic}'")) }
pub fn store_count(n: usize) -> Msg { Msg::info(format!("{n} data stores")) }
pub fn bulk_progress(done: usize, total: usize, verb: &str) -> Msg { Msg::info(format!("{verb} {done}/{total}...")) }
pub fn bulk_result(count: usize, failed: usize, noun: &str, verb: &str) -> Msg {
    if failed == 0 {
        Msg::ok(format!("{verb} {count} {noun}"))
    } else {
        Msg::err(format!("{verb} {count} {noun}, {failed} failed"))
    }
}
pub fn page_count(n: usize, noun: &str, page: usize) -> Msg { Msg::info(format!("{n} {noun} (page {page})")) }
pub fn search_count(n: usize, noun: &str) -> Msg { Msg::info(format!("{n} {noun} (search across whole store)")) }
pub fn loading_search(noun: &str) -> Msg { Msg::info(format!("loading all {noun} for search...")) }
pub fn editor_launch_error(editor: &str, err: impl std::fmt::Display) -> Msg { Msg::err(format!("failed to launch '{editor}': {err}")) }
pub fn editor_exit_error(editor: &str, code: impl std::fmt::Display) -> Msg { Msg::err(format!("'{editor}' exited with {code}")) }
