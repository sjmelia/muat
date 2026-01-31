//! Repository operation types.

use crate::types::AtUri;
use serde::{Deserialize, Serialize};

use super::RecordValue;

/// A record from the repository.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Record {
    /// The AT URI of this record.
    pub uri: AtUri,

    /// The CID (content identifier) of this record.
    pub cid: String,

    /// The record value.
    ///
    /// Guaranteed to be a JSON object with a `$type` field.
    /// This is schema-agnostic beyond that; interpretation is left to higher layers.
    pub value: RecordValue,
}

/// Output from listing records in a collection.
#[derive(Debug, Clone)]
pub struct ListRecordsOutput {
    /// The records in this page.
    pub records: Vec<Record>,

    /// Cursor for the next page, if more records exist.
    pub cursor: Option<String>,
}
