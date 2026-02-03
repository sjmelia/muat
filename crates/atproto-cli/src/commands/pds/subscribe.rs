//! Subscribe command implementation.

use anyhow::{Context, Result};
use clap::Args;
use colored::Colorize;
use futures_util::StreamExt;

use muat::Pds;
use muat::repo::RepoEvent;

use crate::session::storage;

#[derive(Args, Debug)]
pub struct SubscribeArgs {
    /// Starting cursor position
    #[arg(long)]
    pub cursor: Option<i64>,

    /// Output events as JSON
    #[arg(long)]
    pub json: bool,

    /// Filter events by collection prefix (e.g., "app.bsky.")
    #[arg(long)]
    pub filter: Option<String>,
}

pub async fn run(args: SubscribeArgs) -> Result<()> {
    let session = storage::load_session()
        .await
        .context("Failed to load session")?
        .context("No active session. Run 'atproto pds login' first.")?;

    eprintln!("{}", "Connecting to firehose...".dimmed());
    eprintln!("{}", "Press Ctrl+C to stop.".dimmed());
    eprintln!();

    let json_output = args.json;
    let filter = args.filter.clone();

    let pds = Pds::open(session.pds().clone());
    let mut stream = pds
        .firehose_from(args.cursor)
        .context("Failed to start subscription")?;

    while let Some(result) = stream.next().await {
        match result {
            Ok(event) => {
                handle_event(&event, json_output, filter.as_deref());
            }
            Err(e) => {
                eprintln!("{} {}", "ERROR".red(), e);
            }
        }
    }

    Ok(())
}

fn handle_event(event: &RepoEvent, json_output: bool, filter: Option<&str>) {
    match event {
        RepoEvent::Commit(commit) => {
            // Apply filter if specified
            if let Some(prefix) = filter {
                let matches = commit.ops.iter().any(|op| op.path.starts_with(prefix));
                if !matches {
                    return; // don't print
                }
            }

            if json_output {
                if let Ok(json) = serde_json::to_string(&commit) {
                    println!("{}", json);
                }
            } else {
                println!(
                    "{} {} {} ops @ seq {}",
                    "COMMIT".green(),
                    commit.repo.dimmed(),
                    commit.ops.len(),
                    commit.seq
                );
                for op in &commit.ops {
                    let action = match op.action.as_str() {
                        "create" => "CREATE".cyan(),
                        "update" => "UPDATE".yellow(),
                        "delete" => "DELETE".red(),
                        other => other.normal(),
                    };
                    println!("  {} {}", action, op.path);
                }
            }
        }
        RepoEvent::Identity(identity) => {
            if json_output {
                if let Ok(json) = serde_json::to_string(&identity) {
                    println!("{}", json);
                }
            } else {
                println!(
                    "{} {} @ seq {}",
                    "IDENTITY".blue(),
                    identity.did.dimmed(),
                    identity.seq
                );
            }
        }
        RepoEvent::Handle(handle) => {
            if json_output {
                if let Ok(json) = serde_json::to_string(&handle) {
                    println!("{}", json);
                }
            } else {
                println!(
                    "{} {} -> {} @ seq {}",
                    "HANDLE".magenta(),
                    handle.did.dimmed(),
                    handle.handle,
                    handle.seq
                );
            }
        }
        RepoEvent::Info(info) => {
            if !json_output {
                eprintln!(
                    "{} {} {}",
                    "INFO".dimmed(),
                    info.name,
                    info.message.as_deref().unwrap_or("")
                );
            }
        }
        RepoEvent::Unknown { kind } => {
            if !json_output {
                eprintln!("{} {}", "UNKNOWN".dimmed(), kind);
            }
        }
    }
}
