//! Authenticated session for repository operations.

use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, instrument};

use crate::Result;
use crate::account::{AccessToken, RefreshToken};
use crate::error::AuthError;
use crate::repo::{ListRecordsOutput, Record, RecordValue};
use crate::types::{AtUri, Did, Nsid, PdsUrl};

use super::file::FilePds;
use super::xrpc::XrpcPds;

/// An authenticated session.
///
/// Sessions represent a logged-in capability for a specific DID on a specific PDS.
/// Authenticated repository operations are methods on this type.
#[derive(Clone)]
pub struct Session {
    inner: Arc<SessionInner>,
}

#[derive(Debug)]
enum SessionKind {
    File(FileSession),
    Xrpc(XrpcSession),
}

#[derive(Debug)]
struct SessionInner {
    did: Did,
    pds: PdsUrl,
    kind: SessionKind,
    tokens: RwLock<SessionTokens>,
}

#[derive(Debug)]
struct SessionTokens {
    access_token: Option<AccessToken>,
    refresh_token: Option<RefreshToken>,
}

#[derive(Debug)]
struct FileSession {
    pds: FilePds,
}

#[derive(Debug)]
struct XrpcSession {
    pds: XrpcPds,
}

impl Session {
    pub(crate) fn new_file(pds: FilePds, did: Did) -> Self {
        Self {
            inner: Arc::new(SessionInner {
                did,
                pds: pds.url().clone(),
                kind: SessionKind::File(FileSession { pds }),
                tokens: RwLock::new(SessionTokens {
                    access_token: None,
                    refresh_token: None,
                }),
            }),
        }
    }

    pub(crate) fn new_xrpc(
        pds: XrpcPds,
        did: Did,
        access_token: String,
        refresh_token: Option<String>,
    ) -> Self {
        Self {
            inner: Arc::new(SessionInner {
                did,
                pds: pds.url().clone(),
                kind: SessionKind::Xrpc(XrpcSession { pds }),
                tokens: RwLock::new(SessionTokens {
                    access_token: Some(AccessToken::new(access_token)),
                    refresh_token: refresh_token.map(RefreshToken::new),
                }),
            }),
        }
    }

    /// Restore a session from persisted tokens.
    ///
    /// This is only valid for network PDS sessions.
    pub fn from_persisted(
        pds: PdsUrl,
        did: Did,
        access_token: String,
        refresh_token: Option<String>,
    ) -> Self {
        Self::new_xrpc(XrpcPds::new(pds), did, access_token, refresh_token)
    }

    /// Returns the DID associated with this session.
    pub fn did(&self) -> &Did {
        &self.inner.did
    }

    /// Returns the PDS URL for this session.
    pub fn pds(&self) -> &PdsUrl {
        &self.inner.pds
    }

    /// Refresh the session tokens.
    ///
    /// This is a no-op for file-backed sessions.
    #[instrument(skip(self), fields(did = %self.inner.did))]
    pub async fn refresh(&self) -> Result<()> {
        match &self.inner.kind {
            SessionKind::File(_) => Ok(()),
            SessionKind::Xrpc(session) => {
                info!("Refreshing session");

                let refresh_token = {
                    let tokens = self.inner.tokens.read().await;
                    tokens
                        .refresh_token
                        .as_ref()
                        .map(|t| t.as_str().to_string())
                };

                let refresh_token = refresh_token.ok_or(AuthError::RefreshTokenInvalid)?;

                let response = session.pds.refresh_session(&refresh_token).await?;

                {
                    let mut tokens = self.inner.tokens.write().await;
                    tokens.access_token = Some(AccessToken::new(response.access_jwt));
                    tokens.refresh_token = Some(RefreshToken::new(response.refresh_jwt));
                }

                debug!("Session refreshed successfully");
                Ok(())
            }
        }
    }

    /// Export the current access token for persistence.
    pub async fn export_access_token(&self) -> Option<String> {
        let tokens = self.inner.tokens.read().await;
        tokens.access_token.as_ref().map(|t| t.as_str().to_string())
    }

    /// Export the current refresh token for persistence.
    pub async fn export_refresh_token(&self) -> Option<String> {
        let tokens = self.inner.tokens.read().await;
        tokens
            .refresh_token
            .as_ref()
            .map(|t| t.as_str().to_string())
    }

    async fn access_token(&self) -> Option<String> {
        let tokens = self.inner.tokens.read().await;
        tokens.access_token.as_ref().map(|t| t.as_str().to_string())
    }

    // ========================================================================
    // Repository Operations
    // ========================================================================

    /// List records in a collection.
    #[instrument(skip(self), fields(did = %self.inner.did, %collection))]
    pub async fn list_records(
        &self,
        repo: &Did,
        collection: &Nsid,
        limit: Option<u32>,
        cursor: Option<&str>,
    ) -> Result<ListRecordsOutput> {
        debug!("Listing records");
        match &self.inner.kind {
            SessionKind::File(session) => {
                session
                    .pds
                    .list_records(repo, collection, limit, cursor)
                    .await
            }
            SessionKind::Xrpc(session) => {
                let token = self.access_token().await.ok_or(AuthError::SessionExpired)?;
                session
                    .pds
                    .list_records(repo, collection, limit, cursor, &token)
                    .await
            }
        }
    }

    /// Get a single record by its AT URI.
    #[instrument(skip(self), fields(did = %self.inner.did, %uri))]
    pub async fn get_record(&self, uri: &AtUri) -> Result<Record> {
        debug!("Getting record");
        match &self.inner.kind {
            SessionKind::File(session) => session.pds.get_record(uri).await,
            SessionKind::Xrpc(session) => {
                let token = self.access_token().await.ok_or(AuthError::SessionExpired)?;
                session.pds.get_record(uri, &token).await
            }
        }
    }

    /// Create a new record in a collection with a validated [`RecordValue`].
    #[instrument(skip(self, value), fields(did = %self.inner.did, %collection))]
    pub async fn create_record(&self, collection: &Nsid, value: &RecordValue) -> Result<AtUri> {
        debug!("Creating record");
        match &self.inner.kind {
            SessionKind::File(session) => {
                session
                    .pds
                    .create_record(&self.inner.did, collection, value, None)
                    .await
            }
            SessionKind::Xrpc(session) => {
                let token = self.access_token().await.ok_or(AuthError::SessionExpired)?;
                session
                    .pds
                    .create_record(&self.inner.did, collection, value, None, &token)
                    .await
            }
        }
    }

    /// Create a new record in a collection from raw JSON.
    #[instrument(skip(self, value), fields(did = %self.inner.did, %collection))]
    pub async fn create_record_raw(
        &self,
        collection: &Nsid,
        value: &serde_json::Value,
    ) -> Result<AtUri> {
        debug!("Creating record (raw)");
        let record_value = RecordValue::new(value.clone())?;
        self.create_record(collection, &record_value).await
    }

    /// Delete a record by its AT URI.
    #[instrument(skip(self), fields(did = %self.inner.did, %uri))]
    pub async fn delete_record(&self, uri: &AtUri) -> Result<()> {
        debug!("Deleting record");
        match &self.inner.kind {
            SessionKind::File(session) => session.pds.delete_record(uri).await,
            SessionKind::Xrpc(session) => {
                let token = self.access_token().await.ok_or(AuthError::SessionExpired)?;
                session.pds.delete_record(uri, &token).await
            }
        }
    }
}

// Custom Debug impl that hides sensitive data
impl std::fmt::Debug for Session {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Session")
            .field("did", &self.inner.did)
            .field("pds", &self.inner.pds)
            .field("kind", &"[REDACTED]")
            .field("tokens", &"[REDACTED]")
            .finish()
    }
}
