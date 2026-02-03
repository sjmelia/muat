//! File-backed PDS implementation.

use async_trait::async_trait;
use bcrypt::{DEFAULT_COST, hash, verify};
use serde_json::json;

use muat_core::error::{AuthError, Error, InvalidInputError};
use muat_core::traits::{CreateAccountOutput, Pds};
use muat_core::types::{Did, PdsUrl};
use muat_core::{AccessToken, Credentials, Result};

use crate::firehose::FileFirehose;
use crate::session::FileSession;
use crate::store::{FileStore, LocalAccount};

/// Filesystem-backed PDS implementation.
#[derive(Debug, Clone)]
pub struct FilePds {
    store: FileStore,
    url: PdsUrl,
}

impl FilePds {
    /// Create a new file-backed PDS at the given root directory.
    pub fn new(root: impl AsRef<std::path::Path>, url: PdsUrl) -> Self {
        Self {
            store: FileStore::new(root),
            url,
        }
    }

    /// Returns the PDS URL for this instance.
    pub fn url(&self) -> &PdsUrl {
        &self.url
    }

    /// Access the underlying file store.
    pub(crate) fn store(&self) -> &FileStore {
        &self.store
    }

    fn make_token(did: &Did, password_hash: &str) -> AccessToken {
        let token = json!({
            "did": did.as_str(),
            "password_hash": password_hash,
        })
        .to_string();
        AccessToken::new(token)
    }

    pub(crate) fn parse_token(token: &AccessToken) -> Result<(Did, String)> {
        let value: serde_json::Value = serde_json::from_str(token.as_str()).map_err(|e| {
            Error::InvalidInput(InvalidInputError::Other {
                message: format!("Invalid token JSON: {}", e),
            })
        })?;

        let did = value.get("did").and_then(|v| v.as_str()).ok_or_else(|| {
            Error::InvalidInput(InvalidInputError::Other {
                message: "Token missing 'did'".to_string(),
            })
        })?;

        let password_hash = value
            .get("password_hash")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                Error::InvalidInput(InvalidInputError::Other {
                    message: "Token missing 'password_hash'".to_string(),
                })
            })?;

        Ok((Did::new(did)?, password_hash.to_string()))
    }

    pub(crate) fn validate_token(&self, token: &AccessToken) -> Result<LocalAccount> {
        let (did, password_hash) = Self::parse_token(token)?;
        let account = self
            .store
            .get_account(&did)?
            .ok_or_else(|| AuthError::InvalidCredentials("Account not found".to_string()))?;

        if account.password_hash != password_hash {
            return Err(AuthError::InvalidCredentials("Invalid token".to_string()).into());
        }

        Ok(account)
    }

    pub(crate) fn ensure_repo_access(&self, token: &AccessToken, repo: &Did) -> Result<()> {
        let account = self.validate_token(token)?;
        let did = Did::new(&account.did)?;

        if &did != repo {
            return Err(AuthError::InvalidCredentials("Access denied".to_string()).into());
        }

        Ok(())
    }

    /// Remove an account with optional record deletion.
    pub async fn remove_account(
        &self,
        did: &Did,
        token: &AccessToken,
        delete_records: bool,
        password: Option<&str>,
    ) -> Result<()> {
        let account = self.validate_token(token)?;
        let token_did = Did::new(&account.did)?;

        if &token_did != did {
            return Err(AuthError::InvalidCredentials("Access denied".to_string()).into());
        }

        if let Some(password) = password {
            let ok = verify(password, &account.password_hash).map_err(|e| {
                Error::InvalidInput(InvalidInputError::Other {
                    message: e.to_string(),
                })
            })?;

            if !ok {
                return Err(AuthError::InvalidCredentials("Invalid password".to_string()).into());
            }
        }

        self.store.remove_account(did, delete_records)
    }
}

#[async_trait]
impl Pds for FilePds {
    type Session = FileSession;
    type Firehose = FileFirehose;

    fn url(&self) -> &PdsUrl {
        self.url()
    }

    async fn login(&self, credentials: Credentials) -> Result<Self::Session> {
        let identifier = credentials.identifier();

        let account = if identifier.starts_with("did:") {
            let did = Did::new(identifier)?;
            self.store.get_account(&did)?
        } else {
            self.store.find_account_by_handle(identifier)?
        }
        .ok_or_else(|| AuthError::InvalidCredentials("Account not found".to_string()))?;

        let ok = verify(credentials.password(), &account.password_hash).map_err(|e| {
            Error::InvalidInput(InvalidInputError::Other {
                message: e.to_string(),
            })
        })?;

        if !ok {
            return Err(AuthError::InvalidCredentials("Invalid password".to_string()).into());
        }

        let did = Did::new(&account.did)?;
        let token = Self::make_token(&did, &account.password_hash);

        Ok(FileSession::new(self.clone(), did, token))
    }

    async fn create_account(
        &self,
        handle: &str,
        password: Option<&str>,
        _email: Option<&str>,
        _invite_code: Option<&str>,
    ) -> Result<CreateAccountOutput> {
        let password = password.ok_or_else(|| {
            Error::InvalidInput(InvalidInputError::Other {
                message: "Password is required for file PDS accounts".to_string(),
            })
        })?;

        let password_hash = hash(password, DEFAULT_COST).map_err(|e| {
            Error::InvalidInput(InvalidInputError::Other {
                message: e.to_string(),
            })
        })?;

        let did = self.store.create_account(handle, &password_hash)?;

        Ok(CreateAccountOutput {
            did,
            handle: handle.to_string(),
        })
    }

    async fn delete_account(
        &self,
        did: &Did,
        token: &AccessToken,
        password: Option<&str>,
    ) -> Result<()> {
        self.remove_account(did, token, true, password).await
    }

    fn firehose_from(&self, _cursor: Option<i64>) -> Result<Self::Firehose> {
        FileFirehose::from_store(self.store.clone())
    }
}
