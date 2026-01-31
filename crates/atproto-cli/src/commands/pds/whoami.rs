//! Whoami command implementation.

use anyhow::{Context, Result};
use clap::Args;

use crate::output;
use crate::session::storage;

#[derive(Args, Debug)]
pub struct WhoamiArgs {}

pub async fn run(_args: WhoamiArgs) -> Result<()> {
    let session = storage::load_session()
        .await
        .context("Failed to load session")?
        .context("No active session. Run 'atproto pds login' first.")?;

    output::field("DID", session.did().as_str());
    output::field("PDS", session.pds().as_str());

    Ok(())
}
