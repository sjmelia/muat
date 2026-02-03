//! Login command implementation.

use anyhow::{Context, Result};
use clap::Args;
use colored::Colorize;

use muat_core::traits::Pds;
use muat_core::{Credentials, PdsUrl};
use muat_file::FilePds;
use muat_xrpc::XrpcPds;

use crate::output;
use crate::session::CliSession;
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
    let pds_url = PdsUrl::new(&args.pds).context("Invalid PDS URL")?;
    let credentials = Credentials::new(&args.identifier, &args.password);

    eprintln!("{}", "Logging in...".dimmed());

    let session = if pds_url.is_local() {
        let path = pds_url
            .to_file_path()
            .context("Failed to convert file:// URL to path")?;
        let pds = FilePds::new(&path, pds_url);
        CliSession::File(pds.login(credentials).await.context("Failed to login")?)
    } else {
        let pds = XrpcPds::new(pds_url.clone());
        CliSession::Xrpc(pds.login(credentials).await.context("Failed to login")?)
    };

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
