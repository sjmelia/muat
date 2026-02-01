//! Create account command implementation.
//!
//! This command creates a new account in a local filesystem-backed PDS.
//! It is not supported for remote PDS instances.

use anyhow::{Context, Result, bail};
use clap::Args;

use muat::PdsUrl;
use muat::backend::file::FilePdsBackend;

use crate::output;

#[derive(Args, Debug)]
pub struct CreateAccountArgs {
    /// Handle for the new account (e.g., alice.local)
    pub handle: String,

    /// PDS URL (must be file://)
    #[arg(long, default_value = "file://./pds")]
    pub pds: String,
}

pub async fn run(args: CreateAccountArgs) -> Result<()> {
    let pds_url = PdsUrl::new(&args.pds).context("Invalid PDS URL")?;

    if !pds_url.is_local() {
        bail!(
            "Remote PDS account creation is not supported by this CLI.\n\
             Use the PDS web interface or official tools instead.\n\
             For local development, use a file:// URL (e.g., file://./pds)"
        );
    }

    let path = pds_url
        .to_file_path()
        .context("Failed to convert file:// URL to path")?;

    let backend = FilePdsBackend::new(&path);
    let did = backend
        .create_account_local(&args.handle)
        .context("Failed to create account")?;

    output::field("DID", did.as_str());
    output::field("Handle", &args.handle);
    output::field("PDS", &args.pds);
    output::success("Account created successfully");

    Ok(())
}
