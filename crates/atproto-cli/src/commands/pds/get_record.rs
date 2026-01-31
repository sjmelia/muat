//! Get record command implementation.

use anyhow::{Context, Result};
use clap::Args;

use muat::{AtUri, Did, Nsid, Rkey};

use crate::output;
use crate::session::storage;

#[derive(Args, Debug)]
pub struct GetRecordArgs {
    /// AT URI of the record (e.g., at://did:plc:.../app.bsky.feed.post/...)
    pub uri: Option<String>,

    /// Repository DID (defaults to session DID)
    #[arg(long)]
    pub repo: Option<String>,

    /// Collection NSID (alternative to URI)
    #[arg(long)]
    pub collection: Option<String>,

    /// Record key (alternative to URI)
    #[arg(long)]
    pub rkey: Option<String>,
}

pub async fn run(args: GetRecordArgs) -> Result<()> {
    let session = storage::load_session()
        .await
        .context("Failed to load session")?
        .context("No active session. Run 'atproto pds login' first.")?;

    let uri = if let Some(uri_str) = &args.uri {
        AtUri::new(uri_str).context("Invalid AT URI")?
    } else {
        // Build from components
        let collection = args
            .collection
            .as_ref()
            .context("Either --uri or --collection is required")?;
        let rkey = args
            .rkey
            .as_ref()
            .context("Either --uri or --rkey is required")?;

        let repo = match &args.repo {
            Some(r) => Did::new(r).context("Invalid repo DID")?,
            None => session.did().clone(),
        };
        let collection = Nsid::new(collection).context("Invalid collection NSID")?;
        let rkey = Rkey::new(rkey).context("Invalid rkey")?;

        AtUri::from_parts(repo, collection, rkey)
    };

    let record = session
        .get_record(&uri)
        .await
        .context("Failed to get record")?;

    output::json_pretty(&record.value)?;

    Ok(())
}
