//! Output formatting helpers.

use anyhow::Result;
use colored::Colorize;
use serde::Serialize;

/// Print a success message.
pub fn success(msg: &str) {
    println!("{} {}", "✓".green(), msg);
}

/// Print an error message.
pub fn error(msg: &str) {
    eprintln!("{} {}", "✗".red(), msg);
}

/// Print a labeled field.
pub fn field(label: &str, value: &str) {
    println!("{}: {}", label.dimmed(), value);
}

/// Print a value as compact JSON.
pub fn json<T: Serialize>(value: &T) -> Result<()> {
    let json = serde_json::to_string(value)?;
    println!("{}", json);
    Ok(())
}

/// Print a value as pretty-printed JSON.
pub fn json_pretty<T: Serialize>(value: &T) -> Result<()> {
    let json = serde_json::to_string_pretty(value)?;
    println!("{}", json);
    Ok(())
}
