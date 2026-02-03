//! Remove account command implementation.
//!
//! This command removes an account from a local filesystem-backed PDS.
//! It is not supported for remote PDS instances.

use std::io::{self, Write};

use anyhow::{Context, Result, bail};
use clap::Args;

use muat_core::traits::{Pds, Session};
use muat_core::{Credentials, Did, PdsUrl};
use muat_file::FilePds;

use crate::output;

#[derive(Args, Debug)]
pub struct RemoveAccountArgs {
    /// DID of the account to remove
    pub did: String,

    /// Account password
    #[arg(long)]
    pub password: String,

    /// Also delete all records for this account
    #[arg(long)]
    pub delete_records: bool,

    /// Skip confirmation prompt
    #[arg(long, short = 'f')]
    pub force: bool,

    /// PDS URL (must be file://)
    #[arg(long, default_value = "file://./pds")]
    pub pds: String,
}

pub async fn run(args: RemoveAccountArgs) -> Result<()> {
    let pds_url = PdsUrl::new(&args.pds).context("Invalid PDS URL")?;

    if !pds_url.is_local() {
        bail!(
            "Remote PDS account removal is not supported by this CLI.\n\
             Use the PDS web interface or official tools instead."
        );
    }

    let path = pds_url
        .to_file_path()
        .context("Failed to convert file:// URL to path")?;

    let did = Did::new(&args.did).context("Invalid DID")?;

    let backend = FilePds::new(&path, pds_url);

    // Check account exists by attempting login
    let session = backend
        .login(Credentials::new(did.as_str(), &args.password))
        .await
        .context("Failed to authenticate account")?;

    // Confirm unless --force
    if !args.force {
        eprint!(
            "This will remove account {}{}. Continue? [y/N] ",
            args.did,
            if args.delete_records {
                " and all its records"
            } else {
                ""
            }
        );
        io::stderr().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        if !input.trim().eq_ignore_ascii_case("y") {
            eprintln!("Aborted.");
            return Ok(());
        }
    }

    backend
        .remove_account(
            &did,
            &session.access_token(),
            args.delete_records,
            Some(&args.password),
        )
        .await
        .context("Failed to remove account")?;

    output::success(&format!("Account {} removed", args.did));

    Ok(())
}
