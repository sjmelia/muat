//! muat-core - Core AT Protocol types and traits.

pub mod credentials;
pub mod error;
pub mod repo;
pub mod tokens;
pub mod traits;
pub mod types;

pub use credentials::Credentials;
pub use error::Error;
pub use repo::{
    CommitEvent, CommitOperation, HandleEvent, IdentityEvent, InfoEvent, Record, RecordValue,
    RepoEvent,
};
pub use tokens::{AccessToken, RefreshToken};
pub use traits::{CreateAccountOutput, Firehose, Pds, Session};
pub use types::{AtUri, Did, Nsid, PdsUrl, Rkey};

/// Result type alias using the crate's Error type.
pub type Result<T> = std::result::Result<T, Error>;
