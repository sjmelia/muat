//! List records command implementation.

use anyhow::{Context, Result};
use clap::Args;
use colored::Colorize;

use muat::{Did, Nsid};

use crate::output;
use crate::session::storage;

#[derive(Args, Debug)]
pub struct ListRecordsArgs {
    /// Repository DID (defaults to session DID)
    #[arg(long)]
    pub repo: Option<String>,

    /// Collection NSID
    #[arg(long)]
    pub collection: String,

    /// Maximum number of records to return
    #[arg(long)]
    pub limit: Option<u32>,

    /// Pagination cursor
    #[arg(long)]
    pub cursor: Option<String>,

    /// Pretty-print JSON output
    #[arg(long)]
    pub pretty: bool,
}

pub async fn run(args: ListRecordsArgs) -> Result<()> {
    let session = storage::load_session()
        .await
        .context("Failed to load session")?
        .context("No active session. Run 'atproto pds login' first.")?;

    let repo = match &args.repo {
        Some(r) => Did::new(r).context("Invalid repo DID")?,
        None => session.did().clone(),
    };

    let collection = Nsid::new(&args.collection).context("Invalid collection NSID")?;

    let result = session
        .list_records(&repo, &collection, args.limit, args.cursor.as_deref())
        .await
        .context("Failed to list records")?;

    if result.records.is_empty() {
        eprintln!("{}", "No records found.".dimmed());
        return Ok(());
    }

    for record in &result.records {
        if args.pretty {
            output::json_pretty(&record.value)?;
        } else {
            output::json(&record)?;
        }
        println!();
    }

    if let Some(cursor) = &result.cursor {
        eprintln!();
        eprintln!("{}: {}", "Next cursor".dimmed(), cursor);
    }

    Ok(())
}
