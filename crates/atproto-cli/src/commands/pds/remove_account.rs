//! Remove account command implementation.
//!
//! This command removes an account from a local filesystem-backed PDS.
//! It is not supported for remote PDS instances.

use std::io::{self, Write};

use anyhow::{Context, Result, bail};
use clap::Args;

use muat::backend::file::FilePdsBackend;
use muat::{Did, PdsUrl};

use crate::output;

#[derive(Args, Debug)]
pub struct RemoveAccountArgs {
    /// DID of the account to remove
    pub did: String,

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

    let backend = FilePdsBackend::new(&path);

    // Check account exists
    let account = backend
        .get_account(&did)
        .context("Failed to check account")?;

    if account.is_none() {
        bail!("Account {} not found", args.did);
    }

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
        .remove_account(&did, args.delete_records)
        .context("Failed to remove account")?;

    output::success(&format!("Account {} removed", args.did));

    Ok(())
}
