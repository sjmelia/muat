//! CLI session wrapper.

use async_trait::async_trait;

use muat_core::repo::{ListRecordsOutput, Record, RecordValue};
use muat_core::traits::Session;
use muat_core::types::{AtUri, Did, Nsid, PdsUrl};
use muat_core::{AccessToken, RefreshToken, Result};
use muat_file::FileSession;
use muat_xrpc::XrpcSession;

/// Session wrapper for CLI use.
#[derive(Debug)]
pub enum CliSession {
    File(FileSession),
    Xrpc(XrpcSession),
}

impl CliSession {
    pub fn did(&self) -> &Did {
        match self {
            CliSession::File(session) => session.did(),
            CliSession::Xrpc(session) => session.did(),
        }
    }

    pub fn pds(&self) -> &PdsUrl {
        match self {
            CliSession::File(session) => session.pds(),
            CliSession::Xrpc(session) => session.pds(),
        }
    }

    pub fn access_token(&self) -> AccessToken {
        match self {
            CliSession::File(session) => session.access_token(),
            CliSession::Xrpc(session) => session.access_token(),
        }
    }

    pub fn refresh_token(&self) -> Option<RefreshToken> {
        match self {
            CliSession::File(session) => session.refresh_token(),
            CliSession::Xrpc(session) => session.refresh_token(),
        }
    }

    pub fn as_xrpc(&self) -> Option<&XrpcSession> {
        match self {
            CliSession::Xrpc(session) => Some(session),
            _ => None,
        }
    }
}

#[async_trait]
impl Session for CliSession {
    fn did(&self) -> &Did {
        match self {
            CliSession::File(session) => session.did(),
            CliSession::Xrpc(session) => session.did(),
        }
    }

    fn pds(&self) -> &PdsUrl {
        match self {
            CliSession::File(session) => session.pds(),
            CliSession::Xrpc(session) => session.pds(),
        }
    }

    fn access_token(&self) -> AccessToken {
        match self {
            CliSession::File(session) => session.access_token(),
            CliSession::Xrpc(session) => session.access_token(),
        }
    }

    fn refresh_token(&self) -> Option<RefreshToken> {
        match self {
            CliSession::File(session) => session.refresh_token(),
            CliSession::Xrpc(session) => session.refresh_token(),
        }
    }

    async fn list_records(
        &self,
        repo: &Did,
        collection: &Nsid,
        limit: Option<u32>,
        cursor: Option<&str>,
    ) -> Result<ListRecordsOutput> {
        match self {
            CliSession::File(session) => {
                session.list_records(repo, collection, limit, cursor).await
            }
            CliSession::Xrpc(session) => {
                session.list_records(repo, collection, limit, cursor).await
            }
        }
    }

    async fn get_record(&self, uri: &AtUri) -> Result<Record> {
        match self {
            CliSession::File(session) => session.get_record(uri).await,
            CliSession::Xrpc(session) => session.get_record(uri).await,
        }
    }

    async fn create_record(&self, collection: &Nsid, value: &RecordValue) -> Result<AtUri> {
        match self {
            CliSession::File(session) => session.create_record(collection, value).await,
            CliSession::Xrpc(session) => session.create_record(collection, value).await,
        }
    }

    async fn delete_record(&self, uri: &AtUri) -> Result<()> {
        match self {
            CliSession::File(session) => session.delete_record(uri).await,
            CliSession::Xrpc(session) => session.delete_record(uri).await,
        }
    }
}
