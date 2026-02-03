//! Login command implementation.

use anyhow::{Context, Result};
use clap::Args;
use colored::Colorize;

use muat::{Credentials, Pds, PdsUrl};

use crate::output;
use crate::session::storage;

#[derive(Args, Debug)]
pub struct LoginArgs {
    /// Handle or DID to authenticate with
    #[arg(long)]
    pub identifier: String,

    /// Account password or app password
    #[arg(long)]
    pub password: String,

    /// PDS base URL
    #[arg(long, default_value = "https://bsky.social")]
    pub pds: String,
}

pub async fn run(args: LoginArgs) -> Result<()> {
    let pds = PdsUrl::new(&args.pds).context("Invalid PDS URL")?;
    let credentials = Credentials::new(&args.identifier, &args.password);

    eprintln!("{}", "Logging in...".dimmed());

    let session = Pds::open(pds)
        .login(credentials)
        .await
        .context("Failed to login")?;

    // Save session
    storage::save_session(&session)
        .await
        .context("Failed to save session")?;

    // Print success
    output::success("Logged in successfully");
    println!();
    output::field("DID", session.did().as_str());
    output::field("PDS", session.pds().as_str());

    Ok(())
}
