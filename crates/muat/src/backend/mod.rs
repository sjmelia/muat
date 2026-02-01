//! PDS backend abstraction.
//!
//! This module provides a unified interface over different PDS implementations,
//! allowing the library to work with both network PDS instances and local
//! filesystem-backed storage.
//!
//! ## Backend Types
//!
//! - [`FilePdsBackend`](file::FilePdsBackend) - Filesystem-backed storage for local development
//! - [`XrpcPdsBackend`](xrpc::XrpcPdsBackend) - Network-backed storage via XRPC
//!
//! ## Backend Selection
//!
//! Use [`create_backend`] to automatically select the appropriate backend based on
//! the PDS URL scheme:
//!
//! - `file://` → `FilePdsBackend`
//! - `http://` / `https://` → `XrpcPdsBackend`
//!
//! ## Example
//!
//! ```no_run
//! use muat::backend::{create_backend, PdsBackend};
//! use muat::{PdsUrl, Did, Nsid};
//!
//! # async fn example() -> Result<(), muat::Error> {
//! // File-based backend for local development
//! let file_pds = PdsUrl::new("file:///tmp/my-pds")?;
//! let file_backend = create_backend(&file_pds);
//!
//! // Network backend for production
//! let network_pds = PdsUrl::new("https://bsky.social")?;
//! let network_backend = create_backend(&network_pds);
//! # Ok(())
//! # }
//! ```

pub mod file;
pub mod xrpc;

use async_trait::async_trait;

use crate::Result;
use crate::repo::{ListRecordsOutput, Record, RecordValue};
use crate::types::{AtUri, Did, Nsid, PdsUrl};

pub use file::FilePdsBackend;
pub use xrpc::XrpcPdsBackend;

/// Output from account creation.
#[derive(Debug, Clone)]
pub struct CreateAccountOutput {
    /// The DID of the created account.
    pub did: Did,
    /// The handle of the created account.
    pub handle: String,
}

/// A backend for PDS operations.
///
/// This trait abstracts the storage and retrieval of AT Protocol records,
/// allowing different implementations (network, filesystem) to be used
/// interchangeably.
///
/// ## Token Parameters
///
/// Methods that require authentication accept an optional token parameter.
/// For network backends, the token is required for authenticated operations.
/// For filesystem backends, the token is ignored (no authentication needed).
#[async_trait]
pub trait PdsBackend: Send + Sync {
    /// Create a record in the repository.
    ///
    /// Returns the AT URI of the created record.
    ///
    /// # Arguments
    ///
    /// * `repo` - The repository DID
    /// * `collection` - The collection NSID
    /// * `value` - The record value
    /// * `rkey` - Optional record key (auto-generated if not provided)
    /// * `token` - Authentication token (required for network backends)
    async fn create_record(
        &self,
        repo: &Did,
        collection: &Nsid,
        value: &RecordValue,
        rkey: Option<&str>,
        token: Option<&str>,
    ) -> Result<AtUri>;

    /// Get a record from the repository.
    ///
    /// # Arguments
    ///
    /// * `uri` - The AT URI of the record
    /// * `token` - Authentication token (required for network backends)
    async fn get_record(&self, uri: &AtUri, token: Option<&str>) -> Result<Record>;

    /// List records in a collection.
    ///
    /// # Arguments
    ///
    /// * `repo` - The repository DID
    /// * `collection` - The collection NSID
    /// * `limit` - Maximum number of records to return
    /// * `cursor` - Pagination cursor from a previous response
    /// * `token` - Authentication token (required for network backends)
    async fn list_records(
        &self,
        repo: &Did,
        collection: &Nsid,
        limit: Option<u32>,
        cursor: Option<&str>,
        token: Option<&str>,
    ) -> Result<ListRecordsOutput>;

    /// Delete a record from the repository.
    ///
    /// # Arguments
    ///
    /// * `uri` - The AT URI of the record to delete
    /// * `token` - Authentication token (required for network backends)
    async fn delete_record(&self, uri: &AtUri, token: Option<&str>) -> Result<()>;

    /// Create a new account.
    ///
    /// For filesystem backends, this creates a local account with a generated DID.
    /// For network backends, this calls the createAccount XRPC endpoint.
    ///
    /// # Arguments
    ///
    /// * `handle` - The desired handle for the account
    /// * `password` - The account password (ignored for filesystem backends)
    /// * `email` - Optional email address
    /// * `invite_code` - Optional invite code
    async fn create_account(
        &self,
        handle: &str,
        password: Option<&str>,
        email: Option<&str>,
        invite_code: Option<&str>,
    ) -> Result<CreateAccountOutput>;

    /// Delete an account.
    ///
    /// For filesystem backends, this removes the account and optionally its records.
    /// For network backends, this calls the deleteAccount XRPC endpoint.
    ///
    /// # Arguments
    ///
    /// * `did` - The DID of the account to delete
    /// * `token` - Authentication token (access or refresh token)
    /// * `password` - Account password (required for network backends)
    async fn delete_account(
        &self,
        did: &Did,
        token: Option<&str>,
        password: Option<&str>,
    ) -> Result<()>;
}

/// Concrete backend storage for use in Session.
///
/// This enum provides a closed set of backend implementations,
/// avoiding dynamic dispatch and keeping types explicit.
#[derive(Debug, Clone)]
pub enum BackendKind {
    /// Filesystem-backed PDS for local development.
    File(FilePdsBackend),
    /// Network-backed PDS via XRPC.
    Xrpc(XrpcPdsBackend),
}

impl BackendKind {
    /// Returns true if this is a file-based backend.
    pub fn is_file(&self) -> bool {
        matches!(self, BackendKind::File(_))
    }

    /// Returns true if this is a network-based backend.
    pub fn is_xrpc(&self) -> bool {
        matches!(self, BackendKind::Xrpc(_))
    }
}

#[async_trait]
impl PdsBackend for BackendKind {
    async fn create_record(
        &self,
        repo: &Did,
        collection: &Nsid,
        value: &RecordValue,
        rkey: Option<&str>,
        token: Option<&str>,
    ) -> Result<AtUri> {
        match self {
            BackendKind::File(backend) => {
                backend
                    .create_record(repo, collection, value, rkey, token)
                    .await
            }
            BackendKind::Xrpc(backend) => {
                backend
                    .create_record(repo, collection, value, rkey, token)
                    .await
            }
        }
    }

    async fn get_record(&self, uri: &AtUri, token: Option<&str>) -> Result<Record> {
        match self {
            BackendKind::File(backend) => backend.get_record(uri, token).await,
            BackendKind::Xrpc(backend) => backend.get_record(uri, token).await,
        }
    }

    async fn list_records(
        &self,
        repo: &Did,
        collection: &Nsid,
        limit: Option<u32>,
        cursor: Option<&str>,
        token: Option<&str>,
    ) -> Result<ListRecordsOutput> {
        match self {
            BackendKind::File(backend) => {
                backend
                    .list_records(repo, collection, limit, cursor, token)
                    .await
            }
            BackendKind::Xrpc(backend) => {
                backend
                    .list_records(repo, collection, limit, cursor, token)
                    .await
            }
        }
    }

    async fn delete_record(&self, uri: &AtUri, token: Option<&str>) -> Result<()> {
        match self {
            BackendKind::File(backend) => backend.delete_record(uri, token).await,
            BackendKind::Xrpc(backend) => backend.delete_record(uri, token).await,
        }
    }

    async fn create_account(
        &self,
        handle: &str,
        password: Option<&str>,
        email: Option<&str>,
        invite_code: Option<&str>,
    ) -> Result<CreateAccountOutput> {
        match self {
            BackendKind::File(backend) => {
                backend
                    .create_account(handle, password, email, invite_code)
                    .await
            }
            BackendKind::Xrpc(backend) => {
                backend
                    .create_account(handle, password, email, invite_code)
                    .await
            }
        }
    }

    async fn delete_account(
        &self,
        did: &Did,
        token: Option<&str>,
        password: Option<&str>,
    ) -> Result<()> {
        match self {
            BackendKind::File(backend) => backend.delete_account(did, token, password).await,
            BackendKind::Xrpc(backend) => backend.delete_account(did, token, password).await,
        }
    }
}

/// Create a backend based on the PDS URL scheme.
///
/// - `file://` URLs create a [`FilePdsBackend`]
/// - `http://` and `https://` URLs create an [`XrpcPdsBackend`]
///
/// # Example
///
/// ```no_run
/// use muat::backend::create_backend;
/// use muat::PdsUrl;
///
/// let file_pds = PdsUrl::new("file:///tmp/my-pds").unwrap();
/// let backend = create_backend(&file_pds);
/// assert!(backend.is_file());
///
/// let network_pds = PdsUrl::new("https://bsky.social").unwrap();
/// let backend = create_backend(&network_pds);
/// assert!(backend.is_xrpc());
/// ```
pub fn create_backend(pds: &PdsUrl) -> BackendKind {
    if pds.is_local()
        && let Some(path) = pds.to_file_path()
    {
        return BackendKind::File(FilePdsBackend::new(path));
    }
    BackendKind::Xrpc(XrpcPdsBackend::new(pds.clone()))
}

/// Create a file backend if the PDS URL is a file:// URL.
///
/// Returns `None` for network URLs.
#[deprecated(since = "0.3.0", note = "Use create_backend() instead")]
pub fn create_file_backend(pds: &PdsUrl) -> Option<file::FilePdsBackend> {
    if pds.is_local() {
        pds.to_file_path().map(file::FilePdsBackend::new)
    } else {
        None
    }
}
