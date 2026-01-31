//! Repository operation types.

use crate::types::AtUri;
use serde::{Deserialize, Serialize};

/// A record from the repository.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Record {
    /// The AT URI of this record.
    pub uri: AtUri,

    /// The CID (content identifier) of this record.
    pub cid: String,

    /// The record value as JSON.
    ///
    /// This is schema-agnostic; interpretation is left to higher layers.
    pub value: serde_json::Value,
}

/// Output from listing records in a collection.
#[derive(Debug, Clone)]
pub struct ListRecordsOutput {
    /// The records in this page.
    pub records: Vec<Record>,

    /// Cursor for the next page, if more records exist.
    pub cursor: Option<String>,
}
