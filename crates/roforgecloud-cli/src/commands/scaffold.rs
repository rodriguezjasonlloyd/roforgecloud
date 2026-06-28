use anyhow::{Context, Result};
use clap::Subcommand;
use roforgecloud_core::scaffold;
use std::path::Path;
use tokio::fs;

#[derive(Subcommand)]
pub enum ScaffoldCommand {
    #[command(subcommand)]
    Helix(HelixCommand),
}

#[derive(Subcommand)]
pub enum HelixCommand {
    Lsp,
}

pub async fn run(cmd: ScaffoldCommand) -> Result<()> {
    match cmd {
        ScaffoldCommand::Helix(HelixCommand::Lsp) => run_helix().await,
    }
}

async fn run_helix() -> Result<()> {
    let helix_dir = Path::new(".helix");
    fs::create_dir_all(helix_dir)
        .await
        .context("failed to create .helix/")?;

    let client = reqwest::Client::new();

    let (docs, types, flags) = tokio::try_join!(
        scaffold::fetch_docs(&client),
        scaffold::fetch_types(&client),
        scaffold::fetch_fflags(&client),
    )?;

    fs::write(helix_dir.join("docs.json"), docs)
        .await
        .context("failed to write .helix/docs.json")?;
    fs::write(helix_dir.join("types.d.luau"), types)
        .await
        .context("failed to write .helix/types.d.luau")?;

    update_languages_toml(helix_dir, &flags)
        .await
        .context("failed to update .helix/languages.toml")?;

    fs::write(helix_dir.join(".gitignore"), "*")
        .await
        .context("failed to write .helix/.gitignore")?;

    Ok(())
}

fn multiline_array(items: impl IntoIterator<Item = impl Into<String>>) -> toml_edit::Array {
    let mut arr = toml_edit::Array::new();
    for item in items {
        let mut val = toml_edit::Value::from(item.into());
        val.decor_mut().set_prefix("\n  ");
        arr.push_formatted(val);
    }
    arr.set_trailing("\n");
    arr.set_trailing_comma(true);
    arr
}

async fn update_languages_toml(helix_dir: &Path, flags: &[String]) -> Result<()> {
    let mut items = vec![
        "lsp".to_string(),
        "--definitions=.helix/types.d.luau".to_string(),
        "--docs=.helix/docs.json".to_string(),
        "--no-flags-enabled".to_string(),
        "--flag:LuauSolverV2=true".to_string(),
    ];
    items.extend(flags.iter().cloned());

    let args = multiline_array(items);

    let mut lsp_table = toml_edit::Table::new();
    lsp_table.insert("args", toml_edit::value(args));

    let mut ls_table = toml_edit::Table::new();
    ls_table.set_implicit(true);
    ls_table.insert("luau", toml_edit::Item::Table(lsp_table));

    let mut doc = toml_edit::DocumentMut::new();
    doc.insert("language-server", toml_edit::Item::Table(ls_table));

    fs::write(helix_dir.join("languages.toml"), doc.to_string()).await?;
    Ok(())
}
