use anyhow::{Context, Result};
use std::collections::HashSet;
use std::process::Stdio;
use tokio::process::Command;

const DOCS_URL: &str = "https://luau-lsp.pages.dev/api-docs/en-us.json";
const TYPES_URL: &str =
    "https://luau-lsp.pages.dev/type-definitions/globalTypes.PluginSecurity.d.luau";
const FFLAGS_URL: &str =
    "https://clientsettingscdn.roblox.com/v1/settings/application?applicationName=PCStudioApp";

const FLAG_PREFIXES: &[&str] = &["FFlag", "FInt", "DFFlag", "DFInt"];

pub async fn fetch_docs(client: &reqwest::Client) -> Result<Vec<u8>> {
    Ok(client
        .get(DOCS_URL)
        .send()
        .await?
        .error_for_status()?
        .bytes()
        .await?
        .to_vec())
}

pub async fn fetch_types(client: &reqwest::Client) -> Result<Vec<u8>> {
    Ok(client
        .get(TYPES_URL)
        .send()
        .await?
        .error_for_status()?
        .bytes()
        .await?
        .to_vec())
}

pub async fn fetch_fflags(client: &reqwest::Client) -> Result<Vec<String>> {
    let resp: serde_json::Value = client
        .get(FFLAGS_URL)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    let settings = resp["applicationSettings"]
        .as_object()
        .ok_or_else(|| anyhow::anyhow!("unexpected fflags response shape"))?;

    let flag_args: Vec<String> = settings
        .iter()
        .filter_map(|(key, val)| {
            let stripped = FLAG_PREFIXES.iter().find_map(|prefix| {
                let after = key.strip_prefix(prefix)?;
                after.starts_with("Luau").then_some(after)
            })?;
            let value = val.as_str()?;
            Some(format!("--flag:{}={}", stripped, value))
        })
        .collect();

    validate_with_luau_lsp(flag_args).await
}

async fn validate_with_luau_lsp(flag_args: Vec<String>) -> Result<Vec<String>> {
    let temp = std::env::temp_dir().join("_rofc_fflag_probe.luau");
    tokio::fs::write(&temp, "")
        .await
        .context("failed to write temp probe file")?;

    let mut cmd = Command::new("luau-lsp");
    cmd.arg("analyze").arg("--no-flags-enabled");
    for arg in &flag_args {
        cmd.arg(arg);
    }
    cmd.arg(&temp);
    cmd.stderr(Stdio::piped());
    cmd.stdout(Stdio::null());

    let output = cmd
        .output()
        .await
        .context("failed to run luau-lsp, is it on PATH?")?;
    let _ = tokio::fs::remove_file(&temp).await;

    let stderr = String::from_utf8_lossy(&output.stderr);
    let bad: HashSet<&str> = stderr
        .lines()
        .filter_map(|line| {
            let pos = line.find("Unknown FFlag: ")?;
            Some(line[pos + "Unknown FFlag: ".len()..].trim())
        })
        .collect();

    Ok(flag_args
        .into_iter()
        .filter(|arg| {
            let name = arg
                .strip_prefix("--flag:")
                .and_then(|s| s.split('=').next())
                .unwrap_or("");
            !bad.contains(name)
        })
        .collect())
}
