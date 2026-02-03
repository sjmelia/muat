//! XRPC-backed session implementation.

use std::sync::{Arc, RwLock};

use async_trait::async_trait;
use tracing::{debug, info, instrument};

use muat_core::error::AuthError;
use muat_core::repo::{ListRecordsOutput, Record, RecordValue};
use muat_core::traits::Session as SessionTrait;
use muat_core::types::{AtUri, Did, Nsid, PdsUrl};
use muat_core::{AccessToken, RefreshToken, Result};

use crate::pds::XrpcPds;

/// Session for an XRPC-backed PDS.
#[derive(Clone)]
pub struct XrpcSession {
    inner: Arc<SessionInner>,
}

#[derive(Debug)]
struct SessionInner {
    did: Did,
    pds: PdsUrl,
    pds_impl: XrpcPds,
    tokens: RwLock<SessionTokens>,
}

#[derive(Debug)]
struct SessionTokens {
    access_token: AccessToken,
    refresh_token: Option<RefreshToken>,
}

impl XrpcSession {
    pub(crate) fn new(
        pds_impl: XrpcPds,
        did: Did,
        access_token: AccessToken,
        refresh_token: Option<RefreshToken>,
    ) -> Self {
        Self {
            inner: Arc::new(SessionInner {
                did,
                pds: pds_impl.url().clone(),
                pds_impl,
                tokens: RwLock::new(SessionTokens {
                    access_token,
                    refresh_token,
                }),
            }),
        }
    }

    /// Restore a session from persisted tokens.
    pub fn from_persisted(
        pds: PdsUrl,
        did: Did,
        access_token: AccessToken,
        refresh_token: Option<RefreshToken>,
    ) -> Self {
        Self::new(XrpcPds::new(pds), did, access_token, refresh_token)
    }

    /// Refresh the session tokens.
    #[instrument(skip(self), fields(did = %self.inner.did))]
    pub async fn refresh(&self) -> Result<()> {
        info!("Refreshing session");

        let refresh_token = {
            let tokens = self.inner.tokens.read().unwrap();
            tokens
                .refresh_token
                .as_ref()
                .map(|t| t.as_str().to_string())
        };

        let refresh_token = refresh_token.ok_or(AuthError::RefreshTokenInvalid)?;

        let response = self.inner.pds_impl.refresh_session(&refresh_token).await?;

        {
            let mut tokens = self.inner.tokens.write().unwrap();
            tokens.access_token = AccessToken::new(response.access_jwt);
            tokens.refresh_token = Some(RefreshToken::new(response.refresh_jwt));
        }

        debug!("Session refreshed successfully");
        Ok(())
    }

    fn access_token_string(&self) -> Result<String> {
        let tokens = self.inner.tokens.read().unwrap();
        Ok(tokens.access_token.as_str().to_string())
    }
}

#[async_trait]
impl SessionTrait for XrpcSession {
    fn did(&self) -> &Did {
        &self.inner.did
    }

    fn pds(&self) -> &PdsUrl {
        &self.inner.pds
    }

    fn access_token(&self) -> AccessToken {
        // Clone the current access token snapshot.
        let tokens = self.inner.tokens.read().unwrap();
        AccessToken::new(tokens.access_token.as_str().to_string())
    }

    fn refresh_token(&self) -> Option<RefreshToken> {
        let tokens = self.inner.tokens.read().unwrap();
        tokens
            .refresh_token
            .as_ref()
            .map(|t| RefreshToken::new(t.as_str().to_string()))
    }

    #[instrument(skip(self), fields(did = %self.inner.did, %collection))]
    async fn list_records(
        &self,
        repo: &Did,
        collection: &Nsid,
        limit: Option<u32>,
        cursor: Option<&str>,
    ) -> Result<ListRecordsOutput> {
        debug!("Listing records");
        let token = self.access_token_string()?;
        self.inner
            .pds_impl
            .list_records(repo, collection, limit, cursor, &token)
            .await
    }

    #[instrument(skip(self), fields(did = %self.inner.did, %uri))]
    async fn get_record(&self, uri: &AtUri) -> Result<Record> {
        debug!("Getting record");
        let token = self.access_token_string()?;
        self.inner.pds_impl.get_record(uri, &token).await
    }

    #[instrument(skip(self, value), fields(did = %self.inner.did, %collection))]
    async fn create_record(&self, collection: &Nsid, value: &RecordValue) -> Result<AtUri> {
        debug!("Creating record");
        let token = self.access_token_string()?;
        self.inner
            .pds_impl
            .create_record(&self.inner.did, collection, value, None, &token)
            .await
    }

    #[instrument(skip(self), fields(did = %self.inner.did, %uri))]
    async fn delete_record(&self, uri: &AtUri) -> Result<()> {
        debug!("Deleting record");
        let token = self.access_token_string()?;
        self.inner.pds_impl.delete_record(uri, &token).await
    }
}

impl XrpcSession {
    /// Export the current access token for persistence.
    pub async fn export_access_token(&self) -> AccessToken {
        let tokens = self.inner.tokens.read().unwrap();
        AccessToken::new(tokens.access_token.as_str().to_string())
    }

    /// Export the current refresh token for persistence.
    pub async fn export_refresh_token(&self) -> Option<RefreshToken> {
        let tokens = self.inner.tokens.read().unwrap();
        tokens
            .refresh_token
            .as_ref()
            .map(|t| RefreshToken::new(t.as_str().to_string()))
    }
}

impl std::fmt::Debug for XrpcSession {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("XrpcSession")
            .field("did", &self.inner.did)
            .field("pds", &self.inner.pds)
            .field("tokens", &"[REDACTED]")
            .finish()
    }
}
