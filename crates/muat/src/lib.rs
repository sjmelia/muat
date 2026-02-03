//! muat - Core AT Protocol Library
//!
//! This library provides foundational AT Protocol primitives with a session-centric API.
//! All authenticated operations flow through a [`Session`] object.
//!
//! # Example
//!
//! ```no_run
//! use muat::{Credentials, Pds, PdsUrl, Nsid};
//!
//! # async fn example() -> Result<(), muat::Error> {
//! let pds_url = PdsUrl::new("https://bsky.social")?;
//! let pds = Pds::open(pds_url);
//! let credentials = Credentials::new("alice.bsky.social", "app-password");
//! let session = pds.login(credentials).await?;
//!
//! let collection = Nsid::new("app.bsky.feed.post")?;
//! let records = session.list_records(&session.did(), &collection, None, None).await?;
//!
//! for record in records.records {
//!     println!("{}: {:?}", record.uri, record.value);
//! }
//! # Ok(())
//! # }
//! ```

pub mod account;
pub mod error;
pub mod pds;
pub mod repo;
pub mod types;

// Re-export primary types at crate root for convenience
pub use account::Credentials;
pub use error::Error;
pub use pds::{Pds, RepoEventStream, Session};
pub use repo::RecordValue;
pub use types::{AtUri, Did, Nsid, PdsUrl, Rkey};

/// Result type alias using the crate's Error type.
pub type Result<T> = std::result::Result<T, Error>;
