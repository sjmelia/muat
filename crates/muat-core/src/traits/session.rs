//! Authenticated session trait.

use async_trait::async_trait;

use crate::repo::{ListRecordsOutput, Record, RecordValue};
use crate::types::{AtUri, Did, Nsid, PdsUrl};
use crate::{AccessToken, RefreshToken, Result};

/// An authenticated session for repository operations.
#[async_trait]
pub trait Session: Send + Sync {
    /// Returns the DID associated with this session.
    fn did(&self) -> &Did;

    /// Returns the PDS URL associated with this session.
    fn pds(&self) -> &PdsUrl;

    /// Returns the access token for this session.
    fn access_token(&self) -> AccessToken;

    /// Returns the refresh token for this session, if any.
    fn refresh_token(&self) -> Option<RefreshToken>;

    /// List records in a collection.
    async fn list_records(
        &self,
        repo: &Did,
        collection: &Nsid,
        limit: Option<u32>,
        cursor: Option<&str>,
    ) -> Result<ListRecordsOutput>;

    /// Get a single record by its AT URI.
    async fn get_record(&self, uri: &AtUri) -> Result<Record>;

    /// Create a new record in a collection with a validated [`RecordValue`].
    async fn create_record(&self, collection: &Nsid, value: &RecordValue) -> Result<AtUri>;

    /// Create a new record in a collection from raw JSON.
    async fn create_record_raw(
        &self,
        collection: &Nsid,
        value: &serde_json::Value,
    ) -> Result<AtUri> {
        let record_value = RecordValue::new(value.clone())?;
        self.create_record(collection, &record_value).await
    }

    /// Delete a record by its AT URI.
    async fn delete_record(&self, uri: &AtUri) -> Result<()>;
}
