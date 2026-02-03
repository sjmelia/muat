//! XRPC endpoint definitions and request/response types.

#![allow(dead_code)]

use serde::{Deserialize, Serialize};

// ============================================================================
// Endpoint Names
// ============================================================================

/// com.atproto.server.createSession
pub const CREATE_SESSION: &str = "com.atproto.server.createSession";

/// com.atproto.server.refreshSession
pub const REFRESH_SESSION: &str = "com.atproto.server.refreshSession";

/// com.atproto.server.getSession
pub const GET_SESSION: &str = "com.atproto.server.getSession";

/// com.atproto.repo.listRecords
pub const LIST_RECORDS: &str = "com.atproto.repo.listRecords";

/// com.atproto.repo.getRecord
pub const GET_RECORD: &str = "com.atproto.repo.getRecord";

/// com.atproto.repo.createRecord
pub const CREATE_RECORD: &str = "com.atproto.repo.createRecord";

/// com.atproto.repo.deleteRecord
pub const DELETE_RECORD: &str = "com.atproto.repo.deleteRecord";

/// com.atproto.sync.subscribeRepos
pub const SUBSCRIBE_REPOS: &str = "com.atproto.sync.subscribeRepos";

// ============================================================================
// Request/Response Types
// ============================================================================

/// Request body for createSession.
#[derive(Debug, Serialize)]
pub struct CreateSessionRequest<'a> {
    pub identifier: &'a str,
    pub password: &'a str,
}

/// Response from createSession.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateSessionResponse {
    pub did: String,
    pub handle: String,
    pub access_jwt: String,
    pub refresh_jwt: String,
    #[serde(default)]
    pub email: Option<String>,
    #[serde(default)]
    pub email_confirmed: Option<bool>,
}

/// Response from refreshSession.
/// Note: refreshSession takes no request body; the refresh token is in the Authorization header.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RefreshSessionResponse {
    pub did: String,
    pub handle: String,
    pub access_jwt: String,
    pub refresh_jwt: String,
}

/// Response from getSession.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetSessionResponse {
    pub did: String,
    pub handle: String,
    #[serde(default)]
    pub email: Option<String>,
    #[serde(default)]
    pub email_confirmed: Option<bool>,
}

/// Query parameters for listRecords.
#[derive(Debug, Serialize)]
pub struct ListRecordsQuery<'a> {
    pub repo: &'a str,
    pub collection: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reverse: Option<bool>,
}

/// Response from listRecords.
#[derive(Debug, Deserialize)]
pub struct ListRecordsResponse {
    pub records: Vec<RecordEntry>,
    #[serde(default)]
    pub cursor: Option<String>,
}

/// A single record entry from listRecords.
#[derive(Debug, Deserialize)]
pub struct RecordEntry {
    pub uri: String,
    pub cid: String,
    pub value: serde_json::Value,
}

/// Query parameters for getRecord.
#[derive(Debug, Serialize)]
pub struct GetRecordQuery<'a> {
    pub repo: &'a str,
    pub collection: &'a str,
    pub rkey: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cid: Option<&'a str>,
}

/// Response from getRecord.
#[derive(Debug, Deserialize)]
pub struct GetRecordResponse {
    pub uri: String,
    pub cid: String,
    pub value: serde_json::Value,
}

/// Request body for createRecord.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateRecordRequest<'a> {
    pub repo: &'a str,
    pub collection: &'a str,
    pub record: &'a serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rkey: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub validate: Option<bool>,
}

/// Response from createRecord.
#[derive(Debug, Deserialize)]
pub struct CreateRecordResponse {
    pub uri: String,
    pub cid: String,
}

/// Request body for deleteRecord.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteRecordRequest<'a> {
    pub repo: &'a str,
    pub collection: &'a str,
    pub rkey: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub swap_record: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub swap_commit: Option<&'a str>,
}

/// XRPC error response format.
#[derive(Debug, Deserialize)]
pub struct XrpcErrorResponse {
    pub error: Option<String>,
    pub message: Option<String>,
}
