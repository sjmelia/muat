//! PDS subcommand implementations.

mod create_account;
mod create_record;
mod delete_record;
mod get_record;
mod list_records;
mod login;
mod refresh_token;
mod remove_account;
mod subscribe;
mod whoami;

use anyhow::Result;
use clap::{Args, Subcommand};

#[derive(Args, Debug)]
pub struct PdsCommand {
    #[command(subcommand)]
    pub command: PdsSubcommand,
}

#[derive(Subcommand, Debug)]
pub enum PdsSubcommand {
    /// Create a new session (login)
    Login(login::LoginArgs),

    /// Display the active session
    Whoami(whoami::WhoamiArgs),

    /// Refresh the session tokens
    RefreshToken(refresh_token::RefreshTokenArgs),

    /// Create a new account (local PDS only)
    CreateAccount(create_account::CreateAccountArgs),

    /// Remove an account (local PDS only)
    RemoveAccount(remove_account::RemoveAccountArgs),

    /// Create a new record in a collection
    CreateRecord(create_record::CreateRecordArgs),

    /// List records in a collection
    ListRecords(list_records::ListRecordsArgs),

    /// Fetch a single record
    GetRecord(get_record::GetRecordArgs),

    /// Delete a record
    DeleteRecord(delete_record::DeleteRecordArgs),

    /// Subscribe to repository events
    Subscribe(subscribe::SubscribeArgs),
}

pub async fn handle(cmd: PdsCommand) -> Result<()> {
    match cmd.command {
        PdsSubcommand::Login(args) => login::run(args).await,
        PdsSubcommand::Whoami(args) => whoami::run(args).await,
        PdsSubcommand::RefreshToken(args) => refresh_token::run(args).await,
        PdsSubcommand::CreateAccount(args) => create_account::run(args).await,
        PdsSubcommand::RemoveAccount(args) => remove_account::run(args).await,
        PdsSubcommand::CreateRecord(args) => create_record::run(args).await,
        PdsSubcommand::ListRecords(args) => list_records::run(args).await,
        PdsSubcommand::GetRecord(args) => get_record::run(args).await,
        PdsSubcommand::DeleteRecord(args) => delete_record::run(args).await,
        PdsSubcommand::Subscribe(args) => subscribe::run(args).await,
    }
}
