//! Create record command implementation.

use std::io::{self, Read};

use anyhow::{Context, Result};
use clap::Args;
use serde_json::Value;

use muat::{Nsid, RecordValue};

use crate::output;
use crate::session::storage;

#[derive(Args, Debug)]
pub struct CreateRecordArgs {
    /// Collection NSID (e.g., org.example.record)
    pub collection: String,

    /// Record type ($type field value)
    #[arg(long = "type", short = 't')]
    pub record_type: String,

    /// JSON file with record data (use - for stdin)
    #[arg(long)]
    pub json: Option<String>,
}

pub async fn run(args: CreateRecordArgs) -> Result<()> {
    let session = storage::load_session()
        .await
        .context("Failed to load session")?
        .context("No active session. Run 'atproto pds login' first.")?;

    let collection = Nsid::new(&args.collection).context("Invalid collection NSID")?;

    // Read base JSON if provided
    let base_value: Value = if let Some(ref path) = args.json {
        if path == "-" {
            let mut buf = String::new();
            io::stdin()
                .read_to_string(&mut buf)
                .context("Failed to read from stdin")?;
            serde_json::from_str(&buf).context("Invalid JSON from stdin")?
        } else {
            let content =
                std::fs::read_to_string(path).context("Failed to read JSON file")?;
            serde_json::from_str(&content).context("Invalid JSON in file")?
        }
    } else {
        Value::Object(serde_json::Map::new())
    };

    // Construct RecordValue with the specified type
    let record_value = RecordValue::with_type(&args.record_type, base_value)
        .context("Invalid record value")?;

    // Create the record
    let uri = session
        .create_record(&collection, &record_value)
        .await
        .context("Failed to create record")?;

    // Output the created record's URI
    println!("{}", uri);
    output::success(&format!("Created record: {}", uri));

    Ok(())
}
