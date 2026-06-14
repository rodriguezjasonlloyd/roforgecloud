use std::collections::HashMap;

use serde::Deserialize;

const USERS_URL: &str = "https://users.roblox.com/v1/users";
const CHUNK_SIZE: usize = 100;

#[derive(Debug, Deserialize)]
struct UsersResponse {
    data: Vec<UserInfo>,
}

#[derive(Debug, Deserialize)]
struct UserInfo {
    id: u64,
    name: String,
}

/// Extracts the longest run of consecutive digits from `s`, if any. DataStore
/// entry IDs are often something like `Player_12345` or `PlayerData-12345-v2`,
/// so this picks out the most likely user ID embedded in the string.
pub fn extract_id(s: &str) -> Option<u64> {
    let mut best: Option<&str> = None;
    let mut start = None;

    for (i, c) in s.char_indices() {
        if c.is_ascii_digit() {
            if start.is_none() {
                start = Some(i);
            }
        } else if let Some(begin) = start.take() {
            let run = &s[begin..i];
            if best.is_none_or(|b| run.len() > b.len()) {
                best = Some(run);
            }
        }
    }
    if let Some(begin) = start {
        let run = &s[begin..];
        if best.is_none_or(|b| run.len() > b.len()) {
            best = Some(run);
        }
    }

    best.and_then(|run| run.parse().ok())
}

/// Resolves user IDs to usernames via the public Roblox Users API.
pub async fn resolve_usernames(
    client: &reqwest::Client,
    ids: &[u64],
) -> reqwest::Result<HashMap<u64, String>> {
    let mut result = HashMap::new();

    for chunk in ids.chunks(CHUNK_SIZE) {
        let response: UsersResponse = client
            .post(USERS_URL)
            .json(&serde_json::json!({
                "userIds": chunk,
                "excludeBannedUsers": false,
            }))
            .send()
            .await?
            .json()
            .await?;

        for user in response.data {
            result.insert(user.id, user.name);
        }
    }

    Ok(result)
}
