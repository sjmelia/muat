//! XRPC-backed PDS implementation.

use async_trait::async_trait;
use tracing::{debug, instrument};

use muat_core::error::AuthError;
use muat_core::repo::{ListRecordsOutput, Record, RecordValue};
use muat_core::traits::{CreateAccountOutput, Pds};
use muat_core::types::{AtUri, Did, Nsid, PdsUrl};
use muat_core::{AccessToken, Credentials, RefreshToken, Result};

use crate::firehose::XrpcFirehose;
use crate::session::XrpcSession;
use crate::xrpc::client::XrpcClient;
use crate::xrpc::endpoints::*;

/// Endpoint for account creation.
const CREATE_ACCOUNT: &str = "com.atproto.server.createAccount";

/// Endpoint for account deletion.
const DELETE_ACCOUNT: &str = "com.atproto.server.deleteAccount";

/// Request body for createAccount.
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct CreateAccountRequest<'a> {
    handle: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    password: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    email: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    invite_code: Option<&'a str>,
}

/// Response from createAccount.
#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateAccountResponse {
    did: String,
    handle: String,
    #[allow(dead_code)]
    access_jwt: String,
    #[allow(dead_code)]
    refresh_jwt: String,
}

/// Request body for deleteAccount.
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct DeleteAccountRequest<'a> {
    did: &'a str,
    password: &'a str,
    token: &'a str,
}

/// A network-backed PDS implementation using XRPC.
#[derive(Debug, Clone)]
pub struct XrpcPds {
    pds: PdsUrl,
    client: XrpcClient,
}

impl XrpcPds {
    /// Create a new XRPC PDS for the given PDS URL.
    pub fn new(pds: PdsUrl) -> Self {
        let client = XrpcClient::new(pds.clone());
        Self { pds, client }
    }

    /// Returns the PDS URL for this instance.
    pub fn url(&self) -> &PdsUrl {
        &self.pds
    }

    pub async fn refresh_session(&self, refresh_token: &str) -> Result<RefreshSessionResponse> {
        self.client
            .procedure_authed_no_body(REFRESH_SESSION, refresh_token)
            .await
    }

    #[instrument(skip(self, value, token))]
    pub(crate) async fn create_record(
        &self,
        repo: &Did,
        collection: &Nsid,
        value: &RecordValue,
        rkey: Option<&str>,
        token: &str,
    ) -> Result<AtUri> {
        debug!(repo = %repo, collection = %collection, "Creating record via XRPC");

        let request = CreateRecordRequest {
            repo: repo.as_str(),
            collection: collection.as_str(),
            record: value.as_value(),
            rkey,
            validate: None,
        };

        let response: CreateRecordResponse = self
            .client
            .procedure_authed(CREATE_RECORD, &request, token)
            .await?;

        AtUri::new(&response.uri)
    }

    #[instrument(skip(self, token))]
    pub(crate) async fn get_record(&self, uri: &AtUri, token: &str) -> Result<Record> {
        debug!(uri = %uri, "Getting record via XRPC");

        let query = GetRecordQuery {
            repo: uri.repo().as_str(),
            collection: uri.collection().as_str(),
            rkey: uri.rkey().as_str(),
            cid: None,
        };

        let response: GetRecordResponse =
            self.client.query_authed(GET_RECORD, &query, token).await?;

        Ok(Record {
            uri: AtUri::new(&response.uri)?,
            cid: response.cid,
            value: RecordValue::new(response.value)?,
        })
    }

    #[instrument(skip(self, token))]
    pub(crate) async fn list_records(
        &self,
        repo: &Did,
        collection: &Nsid,
        limit: Option<u32>,
        cursor: Option<&str>,
        token: &str,
    ) -> Result<ListRecordsOutput> {
        debug!(repo = %repo, collection = %collection, "Listing records via XRPC");

        let query = ListRecordsQuery {
            repo: repo.as_str(),
            collection: collection.as_str(),
            limit,
            cursor,
            reverse: None,
        };

        let response: ListRecordsResponse = self
            .client
            .query_authed(LIST_RECORDS, &query, token)
            .await?;

        let records = response
            .records
            .into_iter()
            .map(|r| {
                Ok(Record {
                    uri: AtUri::new(&r.uri)?,
                    cid: r.cid,
                    value: RecordValue::new(r.value)?,
                })
            })
            .collect::<Result<Vec<_>>>()?;

        Ok(ListRecordsOutput {
            records,
            cursor: response.cursor,
        })
    }

    #[instrument(skip(self, token))]
    pub(crate) async fn delete_record(&self, uri: &AtUri, token: &str) -> Result<()> {
        debug!(uri = %uri, "Deleting record via XRPC");

        let request = DeleteRecordRequest {
            repo: uri.repo().as_str(),
            collection: uri.collection().as_str(),
            rkey: uri.rkey().as_str(),
            swap_record: None,
            swap_commit: None,
        };

        self.client
            .procedure_authed_no_response(DELETE_RECORD, &request, token)
            .await
    }
}

#[async_trait]
impl Pds for XrpcPds {
    type Session = XrpcSession;
    type Firehose = XrpcFirehose;

    fn url(&self) -> &PdsUrl {
        &self.pds
    }

    async fn login(&self, credentials: Credentials) -> Result<Self::Session> {
        let request = CreateSessionRequest {
            identifier: credentials.identifier(),
            password: credentials.password(),
        };

        let response: CreateSessionResponse =
            self.client.procedure(CREATE_SESSION, &request).await?;

        let did = Did::new(&response.did)?;

        Ok(XrpcSession::new(
            self.clone(),
            did,
            AccessToken::new(response.access_jwt),
            Some(RefreshToken::new(response.refresh_jwt)),
        ))
    }

    async fn create_account(
        &self,
        handle: &str,
        password: Option<&str>,
        email: Option<&str>,
        invite_code: Option<&str>,
    ) -> Result<CreateAccountOutput> {
        let request = CreateAccountRequest {
            handle,
            password,
            email,
            invite_code,
        };

        let response: CreateAccountResponse =
            self.client.procedure(CREATE_ACCOUNT, &request).await?;

        Ok(CreateAccountOutput {
            did: Did::new(&response.did)?,
            handle: response.handle,
        })
    }

    async fn delete_account(
        &self,
        did: &Did,
        token: &AccessToken,
        password: Option<&str>,
    ) -> Result<()> {
        let password = password.ok_or(AuthError::InvalidCredentials(
            "deleteAccount requires a password".to_string(),
        ))?;

        let request = DeleteAccountRequest {
            did: did.as_str(),
            password,
            token: token.as_str(),
        };

        self.client
            .procedure_authed_no_response(DELETE_ACCOUNT, &request, token.as_str())
            .await
    }

    fn firehose_from(&self, cursor: Option<i64>) -> Result<Self::Firehose> {
        let pds = self.pds.clone();
        let (tx, mut rx) = tokio::sync::mpsc::channel::<Result<muat_core::repo::RepoEvent>>(100);

        tokio::spawn(async move {
            match XrpcFirehose::from_websocket(&pds, cursor).await {
                Ok(mut stream) => {
                    use futures_util::StreamExt;
                    while let Some(event) = stream.next().await {
                        if tx.send(event).await.is_err() {
                            break;
                        }
                    }
                }
                Err(e) => {
                    let _ = tx.send(Err(e)).await;
                }
            }
        });

        let stream = async_stream::stream! {
            while let Some(event) = rx.recv().await {
                yield event;
            }
        };

        Ok(XrpcFirehose::new(stream))
    }
}
