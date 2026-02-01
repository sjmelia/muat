//! Network-backed PDS implementation via XRPC.
//!
//! This module provides a network implementation of the PDS backend,
//! using the XRPC protocol to communicate with a remote PDS.

use async_trait::async_trait;
use tracing::{debug, instrument};

use super::{CreateAccountOutput, PdsBackend};
use crate::Result;
use crate::error::AuthError;
use crate::repo::{ListRecordsOutput, Record, RecordValue};
use crate::types::{AtUri, Did, Nsid, PdsUrl};
use crate::xrpc::{
    CREATE_RECORD, CreateRecordRequest, CreateRecordResponse, DELETE_RECORD, DeleteRecordRequest,
    GET_RECORD, GetRecordQuery, GetRecordResponse, LIST_RECORDS, ListRecordsQuery,
    ListRecordsResponse, XrpcClient,
};

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
///
/// This backend communicates with a remote PDS server using the XRPC protocol.
/// All authenticated operations require a valid access token.
#[derive(Debug, Clone)]
pub struct XrpcPdsBackend {
    client: XrpcClient,
}

impl XrpcPdsBackend {
    /// Create a new XRPC backend for the given PDS URL.
    pub fn new(pds: PdsUrl) -> Self {
        Self {
            client: XrpcClient::new(pds),
        }
    }

    /// Returns the underlying XRPC client.
    ///
    /// This is useful for advanced operations not covered by the `PdsBackend` trait.
    pub fn client(&self) -> &XrpcClient {
        &self.client
    }
}

#[async_trait]
impl PdsBackend for XrpcPdsBackend {
    #[instrument(skip(self, value, token))]
    async fn create_record(
        &self,
        repo: &Did,
        collection: &Nsid,
        value: &RecordValue,
        rkey: Option<&str>,
        token: Option<&str>,
    ) -> Result<AtUri> {
        let token = token.ok_or(AuthError::SessionExpired)?;

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
    async fn get_record(&self, uri: &AtUri, token: Option<&str>) -> Result<Record> {
        let token = token.ok_or(AuthError::SessionExpired)?;

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
    async fn list_records(
        &self,
        repo: &Did,
        collection: &Nsid,
        limit: Option<u32>,
        cursor: Option<&str>,
        token: Option<&str>,
    ) -> Result<ListRecordsOutput> {
        let token = token.ok_or(AuthError::SessionExpired)?;

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
    async fn delete_record(&self, uri: &AtUri, token: Option<&str>) -> Result<()> {
        let token = token.ok_or(AuthError::SessionExpired)?;

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

    #[instrument(skip(self, password))]
    async fn create_account(
        &self,
        handle: &str,
        password: Option<&str>,
        email: Option<&str>,
        invite_code: Option<&str>,
    ) -> Result<CreateAccountOutput> {
        debug!(handle = %handle, "Creating account via XRPC");

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

    #[instrument(skip(self, token, password))]
    async fn delete_account(
        &self,
        did: &Did,
        token: Option<&str>,
        password: Option<&str>,
    ) -> Result<()> {
        let token = token.ok_or(AuthError::SessionExpired)?;
        let password = password.ok_or(AuthError::InvalidCredentials)?;

        debug!(did = %did, "Deleting account via XRPC");

        let request = DeleteAccountRequest {
            did: did.as_str(),
            password,
            token,
        };

        self.client
            .procedure_authed_no_response(DELETE_ACCOUNT, &request, token)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backend_creation() {
        let pds = PdsUrl::new("https://bsky.social").unwrap();
        let backend = XrpcPdsBackend::new(pds);
        assert!(backend.client().pds().as_str().contains("bsky.social"));
    }
}
