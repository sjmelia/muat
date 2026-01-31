//! CLI argument definitions.

use clap::{Parser, Subcommand};

use crate::commands::pds::PdsCommand;

/// AT Protocol CLI tool for PDS exploration.
#[derive(Parser, Debug)]
#[command(name = "atproto")]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    /// Increase verbosity (-v, -vv, -vvv)
    #[arg(short, long, action = clap::ArgAction::Count, global = true)]
    pub verbose: u8,

    /// Output logs as JSON
    #[arg(long, global = true)]
    pub json_logs: bool,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// PDS (Personal Data Server) operations
    Pds(PdsCommand),
}
