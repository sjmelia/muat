//! PDS backend abstraction.
//!
//! This module provides an abstraction over different PDS implementations,
//! allowing the library to work with both network PDS instances and local
//! filesystem-backed storage.

pub mod file;

use async_trait::async_trait;

use crate::Result;
use crate::repo::{ListRecordsOutput, Record, RecordValue};
use crate::types::{AtUri, Did, Nsid, PdsUrl};

/// A backend for PDS operations.
///
/// This trait abstracts the storage and retrieval of AT Protocol records,
/// allowing different implementations (network, filesystem) to be used
/// interchangeably.
#[async_trait]
pub trait PdsBackend: Send + Sync {
    /// Create a record in the repository.
    ///
    /// Returns the AT URI of the created record.
    async fn create_record(
        &self,
        repo: &Did,
        collection: &Nsid,
        value: &RecordValue,
        rkey: Option<&str>,
    ) -> Result<AtUri>;

    /// Get a record from the repository.
    async fn get_record(&self, uri: &AtUri) -> Result<Record>;

    /// List records in a collection.
    async fn list_records(
        &self,
        repo: &Did,
        collection: &Nsid,
        limit: Option<u32>,
        cursor: Option<&str>,
    ) -> Result<ListRecordsOutput>;

    /// Delete a record from the repository.
    async fn delete_record(&self, uri: &AtUri) -> Result<()>;
}

/// Create a backend based on the PDS URL scheme.
///
/// - `file://` URLs create a [`file::FilePdsBackend`]
/// - `http://` and `https://` URLs are not directly supported by this function;
///   use the existing [`Session`](crate::Session) API for network operations.
pub fn create_file_backend(pds: &PdsUrl) -> Option<file::FilePdsBackend> {
    if pds.is_local() {
        pds.to_file_path().map(file::FilePdsBackend::new)
    } else {
        None
    }
}
