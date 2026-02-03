//! PDS access and sessions.
//!
//! This module provides a PDS abstraction with concrete implementations for
//! file-backed and XRPC-backed servers. Authentication produces a `Session`,
//! which is the capability object for authenticated repository operations.

mod firehose;
mod session;

pub mod file;
pub mod xrpc;

use crate::Result;
use crate::account::Credentials;
use crate::error::InvalidInputError;
use crate::types::PdsUrl;

pub use firehose::RepoEventStream;
pub use session::Session;

pub use file::FilePds;
pub use xrpc::XrpcPds;

/// Output from account creation.
#[derive(Debug, Clone)]
pub struct CreateAccountOutput {
    /// The DID of the created account.
    pub did: crate::types::Did,
    /// The handle of the created account.
    pub handle: String,
}

/// A PDS handle, either file-backed or XRPC-backed.
#[derive(Debug, Clone)]
pub struct Pds {
    inner: PdsKind,
}

#[derive(Debug, Clone)]
enum PdsKind {
    File(FilePds),
    Xrpc(XrpcPds),
}

impl Pds {
    /// Open a PDS from a URL.
    ///
    /// - `file://` URLs create a [`FilePds`]
    /// - `http://` and `https://` URLs create an [`XrpcPds`]
    pub fn open(pds: PdsUrl) -> Self {
        if pds.is_local()
            && let Some(path) = pds.to_file_path()
        {
            return Self {
                inner: PdsKind::File(FilePds::new(path, pds)),
            };
        }

        Self {
            inner: PdsKind::Xrpc(XrpcPds::new(pds)),
        }
    }

    /// Returns the PDS URL for this handle.
    pub fn url(&self) -> &PdsUrl {
        match &self.inner {
            PdsKind::File(pds) => pds.url(),
            PdsKind::Xrpc(pds) => pds.url(),
        }
    }

    /// Login to the PDS with credentials, returning an authenticated session.
    pub async fn login(&self, credentials: Credentials) -> Result<Session> {
        match &self.inner {
            PdsKind::File(pds) => pds.login(credentials).await,
            PdsKind::Xrpc(pds) => pds.login(credentials).await,
        }
    }

    /// Create a new account on the PDS.
    pub async fn create_account(
        &self,
        handle: &str,
        password: Option<&str>,
        email: Option<&str>,
        invite_code: Option<&str>,
    ) -> Result<CreateAccountOutput> {
        match &self.inner {
            PdsKind::File(pds) => {
                pds.create_account(handle, password, email, invite_code)
                    .await
            }
            PdsKind::Xrpc(pds) => {
                pds.create_account(handle, password, email, invite_code)
                    .await
            }
        }
    }

    /// Delete an account from the PDS.
    pub async fn delete_account(
        &self,
        did: &crate::types::Did,
        token: Option<&str>,
        password: Option<&str>,
    ) -> Result<()> {
        match &self.inner {
            PdsKind::File(pds) => pds.delete_account(did, token, password).await,
            PdsKind::Xrpc(pds) => pds.delete_account(did, token, password).await,
        }
    }

    /// Subscribe to the firehose stream.
    pub fn firehose(&self) -> Result<RepoEventStream> {
        self.firehose_from(None)
    }

    /// Subscribe to the firehose stream from a given cursor.
    pub fn firehose_from(&self, cursor: Option<i64>) -> Result<RepoEventStream> {
        match &self.inner {
            PdsKind::File(pds) => pds.firehose_from(cursor),
            PdsKind::Xrpc(pds) => pds.firehose_from(cursor),
        }
    }
}

impl TryFrom<PdsUrl> for Pds {
    type Error = crate::Error;

    fn try_from(value: PdsUrl) -> Result<Self> {
        if value.is_network() || value.is_local() {
            Ok(Pds::open(value))
        } else {
            Err(InvalidInputError::PdsUrl {
                value: value.to_string(),
                reason: "unsupported scheme".to_string(),
            }
            .into())
        }
    }
}
