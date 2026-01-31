//! Repository operations and types.
//!
//! This module defines the types used for repository operations.
//! The actual operations are methods on [`Session`](crate::Session).

mod streaming;
mod types;

pub use streaming::{RepoEvent, RepoEventHandler, RepoSubscription};
pub use types::{ListRecordsOutput, Record};
