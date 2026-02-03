//! Repository event types for the firehose stream.

use serde::{Deserialize, Serialize};

/// A repository event from the subscription stream.
#[derive(Debug, Clone)]
pub enum RepoEvent {
    /// A commit event containing repository changes.
    Commit(CommitEvent),

    /// An identity update event.
    Identity(IdentityEvent),

    /// A handle update event.
    Handle(HandleEvent),

    /// The stream info event (sent at connection start).
    Info(InfoEvent),

    /// An unknown event type.
    Unknown { kind: String },
}

/// A commit event from the repository.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitEvent {
    /// The repository DID.
    pub repo: String,

    /// The commit revision.
    pub rev: String,

    /// Sequence number.
    pub seq: i64,

    /// Timestamp of the commit.
    pub time: String,

    /// Operations in this commit.
    #[serde(default)]
    pub ops: Vec<CommitOperation>,
}

/// An operation within a commit.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitOperation {
    /// The path (collection/rkey).
    pub path: String,

    /// The operation action ("create", "update", or "delete").
    pub action: String,

    /// The CID of the record (for creates/updates).
    pub cid: Option<String>,
}

/// An identity update event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentityEvent {
    /// The DID.
    pub did: String,

    /// Sequence number.
    pub seq: i64,

    /// Timestamp.
    pub time: String,
}

/// A handle update event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandleEvent {
    /// The DID.
    pub did: String,

    /// The new handle.
    pub handle: String,

    /// Sequence number.
    pub seq: i64,

    /// Timestamp.
    pub time: String,
}

/// Stream info event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InfoEvent {
    /// The name of the stream.
    pub name: String,

    /// Optional message.
    pub message: Option<String>,
}
