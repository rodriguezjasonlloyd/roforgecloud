use anyhow::Result;
use colored::Colorize;
use colored_json::to_colored_json_auto;
use comfy_table::{presets::UTF8_FULL_CONDENSED, Attribute, Cell, Table};
use serde::Serialize;

pub fn print_json<T: Serialize>(value: &T) -> Result<()> {
    println!("{}", to_colored_json_auto(&serde_json::to_value(value)?)?);
    Ok(())
}

pub fn print_table(header: &[&str], rows: Vec<Vec<String>>) {
    let mut table = Table::new();
    table.load_preset(UTF8_FULL_CONDENSED);
    table.set_header(
        header
            .iter()
            .map(|h| Cell::new(h).add_attribute(Attribute::Bold)),
    );
    for row in rows {
        table.add_row(row);
    }
    println!("{table}");
}

pub fn print_error(err: &anyhow::Error) {
    if let Some(roforgecloud_core::error::Error::Api { status, body }) =
        err.downcast_ref::<roforgecloud_core::error::Error>()
    {
        eprintln!("{} {}", "error:".red().bold(), status.to_string().red());
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(body) {
            if let Ok(pretty) = to_colored_json_auto(&json) {
                eprintln!("{pretty}");
                return;
            }
        }
        eprintln!("{body}");
        return;
    }

    eprintln!("{} {err}", "error:".red().bold());
}

pub fn print_ok() {
    println!("{}", "ok".green());
}

pub fn parse_value(raw: &str) -> Result<serde_json::Value> {
    Ok(serde_json::from_str(raw)?)
}
