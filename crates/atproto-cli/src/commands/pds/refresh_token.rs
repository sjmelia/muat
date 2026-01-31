//! Refresh token command implementation.

use anyhow::{Context, Result};
use clap::Args;
use colored::Colorize;

use crate::output;
use crate::session::storage;

#[derive(Args, Debug)]
pub struct RefreshTokenArgs {}

pub async fn run(_args: RefreshTokenArgs) -> Result<()> {
    let session = storage::load_session()
        .await
        .context("Failed to load session")?
        .context("No active session. Run 'atproto pds login' first.")?;

    eprintln!("{}", "Refreshing session...".dimmed());

    session
        .refresh()
        .await
        .context("Failed to refresh session")?;

    // Save the updated session with new tokens
    storage::save_session(&session)
        .await
        .context("Failed to save refreshed session")?;

    output::success("Session refreshed successfully");
    output::field("DID", session.did().as_str());

    Ok(())
}
