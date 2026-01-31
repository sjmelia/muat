//! Session management for authenticated AT Protocol operations.

use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, instrument};

use crate::error::{AuthError, Error};
use crate::repo::{ListRecordsOutput, Record};
use crate::types::{AtUri, Did, Nsid, PdsUrl};
use crate::xrpc::{
    CREATE_RECORD, CREATE_SESSION, CreateRecordRequest, CreateRecordResponse, CreateSessionRequest,
    DELETE_RECORD, DeleteRecordRequest, GET_RECORD, GetRecordQuery, GetRecordResponse,
    LIST_RECORDS, ListRecordsQuery, ListRecordsResponse, REFRESH_SESSION, RefreshSessionResponse,
    XrpcClient,
};

use super::credentials::Credentials;
use super::tokens::{AccessToken, RefreshToken};

/// A session representing an authenticated connection to a PDS.
///
/// All authenticated AT Protocol operations require a `Session`.
/// Sessions are obtained via [`Session::login()`] and can be refreshed
/// via [`Session::refresh()`].
///
/// # Thread Safety
///
/// Sessions are cheap to clone (they use internal `Arc`) and are safe
/// to share across threads. Token refresh is handled internally with
/// appropriate synchronization.
///
/// # Example
///
/// ```no_run
/// use muat::{Session, Credentials, PdsUrl, Nsid};
///
/// # async fn example() -> Result<(), muat::Error> {
/// let pds = PdsUrl::new("https://bsky.social")?;
/// let creds = Credentials::new("alice.bsky.social", "app-password");
/// let session = Session::login(&pds, creds).await?;
///
/// println!("Logged in as: {}", session.did());
/// # Ok(())
/// # }
/// ```
#[derive(Clone)]
pub struct Session {
    inner: Arc<SessionInner>,
}

struct SessionInner {
    did: Did,
    pds: PdsUrl,
    client: XrpcClient,
    tokens: RwLock<SessionTokens>,
}

struct SessionTokens {
    access_token: AccessToken,
    refresh_token: Option<RefreshToken>,
}

impl Session {
    /// Authenticate with a PDS and create a new session.
    ///
    /// # Arguments
    ///
    /// * `pds` - The PDS to authenticate with
    /// * `credentials` - Login credentials (identifier and password)
    ///
    /// # Errors
    ///
    /// Returns an error if authentication fails or the PDS is unreachable.
    #[instrument(skip(credentials), fields(pds = %pds, identifier = %credentials.identifier()))]
    pub async fn login(pds: &PdsUrl, credentials: Credentials) -> Result<Self, Error> {
        info!("Creating new session");

        let client = XrpcClient::new(pds.clone());

        let request = CreateSessionRequest {
            identifier: credentials.identifier(),
            password: credentials.password(),
        };

        let response: crate::xrpc::CreateSessionResponse =
            client.procedure(CREATE_SESSION, &request).await?;

        let did = Did::new(&response.did)?;
        let access_token = AccessToken::new(response.access_jwt);
        let refresh_token = Some(RefreshToken::new(response.refresh_jwt));

        debug!(did = %did, "Session created successfully");

        Ok(Self {
            inner: Arc::new(SessionInner {
                did,
                pds: pds.clone(),
                client,
                tokens: RwLock::new(SessionTokens {
                    access_token,
                    refresh_token,
                }),
            }),
        })
    }

    /// Create a session from persisted tokens.
    ///
    /// This allows restoring a session without re-authenticating.
    /// The caller is responsible for ensuring the tokens are valid.
    ///
    /// # Arguments
    ///
    /// * `pds` - The PDS URL
    /// * `did` - The DID associated with the session
    /// * `access_token` - The access JWT
    /// * `refresh_token` - The refresh JWT (optional)
    pub fn from_persisted(
        pds: PdsUrl,
        did: Did,
        access_token: String,
        refresh_token: Option<String>,
    ) -> Self {
        let client = XrpcClient::new(pds.clone());

        Self {
            inner: Arc::new(SessionInner {
                did,
                pds,
                client,
                tokens: RwLock::new(SessionTokens {
                    access_token: AccessToken::new(access_token),
                    refresh_token: refresh_token.map(RefreshToken::new),
                }),
            }),
        }
    }

    /// Refresh the session tokens.
    ///
    /// This obtains new access and refresh tokens using the current refresh token.
    /// The session is updated in-place.
    ///
    /// # Errors
    ///
    /// Returns an error if the refresh token is invalid or expired.
    #[instrument(skip(self), fields(did = %self.inner.did))]
    pub async fn refresh(&self) -> Result<(), Error> {
        info!("Refreshing session");

        let refresh_token = {
            let tokens = self.inner.tokens.read().await;
            tokens
                .refresh_token
                .as_ref()
                .map(|t| t.as_str().to_string())
        };

        let refresh_token = refresh_token.ok_or(AuthError::RefreshTokenInvalid)?;

        let response: RefreshSessionResponse = self
            .inner
            .client
            .procedure_authed_no_body(REFRESH_SESSION, &refresh_token)
            .await?;

        // Update tokens
        {
            let mut tokens = self.inner.tokens.write().await;
            tokens.access_token = AccessToken::new(response.access_jwt);
            tokens.refresh_token = Some(RefreshToken::new(response.refresh_jwt));
        }

        debug!("Session refreshed successfully");
        Ok(())
    }

    /// Returns the DID associated with this session.
    pub fn did(&self) -> &Did {
        &self.inner.did
    }

    /// Returns the PDS URL for this session.
    pub fn pds(&self) -> &PdsUrl {
        &self.inner.pds
    }

    /// Export the current access token for persistence.
    ///
    /// # Security
    ///
    /// Handle the returned token securely. It grants access to the account.
    pub async fn export_access_token(&self) -> String {
        let tokens = self.inner.tokens.read().await;
        tokens.access_token.as_str().to_string()
    }

    /// Export the current refresh token for persistence.
    ///
    /// # Security
    ///
    /// Handle the returned token securely. It can be used to obtain new access tokens.
    pub async fn export_refresh_token(&self) -> Option<String> {
        let tokens = self.inner.tokens.read().await;
        tokens
            .refresh_token
            .as_ref()
            .map(|t| t.as_str().to_string())
    }

    // ========================================================================
    // Repository Operations
    // ========================================================================

    /// List records in a collection.
    ///
    /// # Arguments
    ///
    /// * `repo` - The repository DID (usually your own DID)
    /// * `collection` - The collection NSID
    /// * `limit` - Maximum number of records to return (default: 50)
    /// * `cursor` - Pagination cursor from a previous response
    #[instrument(skip(self), fields(did = %self.inner.did, %collection))]
    pub async fn list_records(
        &self,
        repo: &Did,
        collection: &Nsid,
        limit: Option<u32>,
        cursor: Option<&str>,
    ) -> Result<ListRecordsOutput, Error> {
        debug!("Listing records");

        let query = ListRecordsQuery {
            repo: repo.as_str(),
            collection: collection.as_str(),
            limit,
            cursor,
            reverse: None,
        };

        let token = self
            .inner
            .tokens
            .read()
            .await
            .access_token
            .as_str()
            .to_string();

        let response: ListRecordsResponse = self
            .inner
            .client
            .query_authed(LIST_RECORDS, &query, &token)
            .await?;

        let records = response
            .records
            .into_iter()
            .map(|r| {
                Ok(Record {
                    uri: AtUri::new(&r.uri)?,
                    cid: r.cid,
                    value: r.value,
                })
            })
            .collect::<Result<Vec<_>, Error>>()?;

        Ok(ListRecordsOutput {
            records,
            cursor: response.cursor,
        })
    }

    /// Get a single record by its AT URI.
    ///
    /// # Arguments
    ///
    /// * `uri` - The AT URI of the record
    #[instrument(skip(self), fields(did = %self.inner.did, %uri))]
    pub async fn get_record(&self, uri: &AtUri) -> Result<Record, Error> {
        debug!("Getting record");

        let query = GetRecordQuery {
            repo: uri.repo().as_str(),
            collection: uri.collection().as_str(),
            rkey: uri.rkey().as_str(),
            cid: None,
        };

        let token = self
            .inner
            .tokens
            .read()
            .await
            .access_token
            .as_str()
            .to_string();

        let response: GetRecordResponse = self
            .inner
            .client
            .query_authed(GET_RECORD, &query, &token)
            .await?;

        Ok(Record {
            uri: AtUri::new(&response.uri)?,
            cid: response.cid,
            value: response.value,
        })
    }

    /// Create a new record in a collection.
    ///
    /// # Arguments
    ///
    /// * `collection` - The collection NSID
    /// * `value` - The record value as JSON
    ///
    /// # Returns
    ///
    /// The AT URI of the created record.
    #[instrument(skip(self, value), fields(did = %self.inner.did, %collection))]
    pub async fn create_record_raw(
        &self,
        collection: &Nsid,
        value: &serde_json::Value,
    ) -> Result<AtUri, Error> {
        debug!("Creating record");

        let request = CreateRecordRequest {
            repo: self.inner.did.as_str(),
            collection: collection.as_str(),
            record: value,
            rkey: None,
            validate: None,
        };

        let token = self
            .inner
            .tokens
            .read()
            .await
            .access_token
            .as_str()
            .to_string();

        let response: CreateRecordResponse = self
            .inner
            .client
            .procedure_authed(CREATE_RECORD, &request, &token)
            .await?;

        AtUri::new(&response.uri)
    }

    /// Delete a record by its AT URI.
    ///
    /// # Arguments
    ///
    /// * `uri` - The AT URI of the record to delete
    #[instrument(skip(self), fields(did = %self.inner.did, %uri))]
    pub async fn delete_record(&self, uri: &AtUri) -> Result<(), Error> {
        debug!("Deleting record");

        let request = DeleteRecordRequest {
            repo: uri.repo().as_str(),
            collection: uri.collection().as_str(),
            rkey: uri.rkey().as_str(),
            swap_record: None,
            swap_commit: None,
        };

        let token = self
            .inner
            .tokens
            .read()
            .await
            .access_token
            .as_str()
            .to_string();

        self.inner
            .client
            .procedure_authed_no_response(DELETE_RECORD, &request, &token)
            .await
    }
}

// Custom Debug impl that hides sensitive data
impl std::fmt::Debug for Session {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Session")
            .field("did", &self.inner.did)
            .field("pds", &self.inner.pds)
            .field("tokens", &"[REDACTED]")
            .finish()
    }
}
