//! atproto - CLI tool for AT Protocol PDS exploration.
//!
//! This is a thin wrapper over the `muat` library, intended for manual
//! protocol exploration and debugging against a PDS.

mod cli;
mod commands;
mod output;
mod session;

use anyhow::Result;
use clap::Parser;
use tracing_subscriber::{EnvFilter, fmt, prelude::*};

use cli::{Cli, Commands};
use commands::pds;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize logging
    init_logging(cli.verbose, cli.json_logs);

    match cli.command {
        Commands::Pds(pds_cmd) => pds::handle(pds_cmd).await,
    }
}

fn init_logging(verbosity: u8, json: bool) {
    let filter = match verbosity {
        0 => "warn",
        1 => "info",
        2 => "debug",
        _ => "trace",
    };

    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(filter));

    if json {
        tracing_subscriber::registry()
            .with(filter)
            .with(fmt::layer().json())
            .init();
    } else {
        tracing_subscriber::registry()
            .with(filter)
            .with(fmt::layer().with_target(false))
            .init();
    }
}
