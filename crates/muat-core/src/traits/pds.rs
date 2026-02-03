//! PDS trait.

use async_trait::async_trait;

use crate::types::{Did, PdsUrl};
use crate::{AccessToken, Credentials, Result};

use super::{Firehose, Session};

/// Output from account creation.
#[derive(Debug, Clone)]
pub struct CreateAccountOutput {
    /// The DID of the created account.
    pub did: Did,
    /// The handle of the created account.
    pub handle: String,
}

/// A PDS implementation.
#[async_trait]
pub trait Pds: Send + Sync {
    /// Session type for this PDS.
    type Session: Session;
    /// Firehose stream type for this PDS.
    type Firehose: Firehose;

    /// Returns the PDS URL for this instance.
    fn url(&self) -> &PdsUrl;

    /// Authenticate with the PDS and create a new session.
    async fn login(&self, credentials: Credentials) -> Result<Self::Session>;

    /// Create a new account.
    async fn create_account(
        &self,
        handle: &str,
        password: Option<&str>,
        email: Option<&str>,
        invite_code: Option<&str>,
    ) -> Result<CreateAccountOutput>;

    /// Delete an account.
    async fn delete_account(
        &self,
        did: &Did,
        token: &AccessToken,
        password: Option<&str>,
    ) -> Result<()>;

    /// Subscribe to the firehose stream.
    fn firehose(&self) -> Result<Self::Firehose> {
        self.firehose_from(None)
    }

    /// Subscribe to the firehose stream from an optional cursor.
    fn firehose_from(&self, cursor: Option<i64>) -> Result<Self::Firehose>;
}
