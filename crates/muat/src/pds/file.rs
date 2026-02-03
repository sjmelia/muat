//! Filesystem-backed PDS implementation.
//!
//! This module provides a local filesystem implementation of the PDS,
//! enabling local-only development and testing without a network PDS.
//!
//! ## Directory Structure
//!
//! The layout is **repo-centric**: each DID owns a repository containing its collections.
//!
//! ```text
//! $ROOT/pds/
//! ├── accounts/
//! │   └── <did>/
//! │       └── account.json
//! ├── repos/
//! │   └── <did>/
//! │       └── collections/
//! │           └── <collection>/
//! │               └── <rkey>.json
//! └── firehose.jsonl
//! ```
//!
//! This mirrors the AT Protocol data model where repositories belong to users (DIDs),
//! and each repository contains collections of records.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use chrono::Utc;
use fs2::FileExt;
use serde::{Deserialize, Serialize};
use tracing::{debug, instrument};
use uuid::Uuid;

use super::{CreateAccountOutput, RepoEventStream, Session};
use crate::Result;
use crate::account::Credentials;
use crate::error::AuthError;
use crate::error::{Error, InvalidInputError, ProtocolError};
use crate::pds::firehose::{FirehoseLogEvent, FirehoseLogOp};
use crate::repo::{ListRecordsOutput, Record, RecordValue};
use crate::types::{AtUri, Did, Nsid, PdsUrl, Rkey};

/// A filesystem-backed PDS implementation.
///
/// This PDS stores records as JSON files in a directory structure,
/// and maintains an append-only firehose log for event streaming.
#[derive(Debug, Clone)]
pub struct FilePds {
    root: PathBuf,
    url: PdsUrl,
}

/// Account metadata stored in the local PDS.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalAccount {
    /// The DID of the account.
    pub did: String,
    /// The handle (username) of the account.
    pub handle: String,
    /// When the account was created.
    pub created_at: String,
}

impl FilePds {
    /// Create a new filesystem PDS with the given root directory.
    pub fn new(root: impl AsRef<Path>, url: PdsUrl) -> Self {
        Self {
            root: root.as_ref().to_path_buf(),
            url,
        }
    }

    /// Returns the PDS URL for this instance.
    pub fn url(&self) -> &PdsUrl {
        &self.url
    }

    /// Get the root directory path.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Get the PDS data directory.
    fn pds_dir(&self) -> PathBuf {
        self.root.join("pds")
    }

    /// Get the accounts directory.
    fn accounts_dir(&self) -> PathBuf {
        self.pds_dir().join("accounts")
    }

    /// Get the repos directory.
    fn repos_dir(&self) -> PathBuf {
        self.pds_dir().join("repos")
    }

    /// Get the path for a specific account.
    fn account_path(&self, did: &Did) -> PathBuf {
        self.accounts_dir().join(did.as_str()).join("account.json")
    }

    /// Get the collections directory for a specific repo (DID).
    fn repo_collections_dir(&self, did: &Did) -> PathBuf {
        self.repos_dir().join(did.as_str()).join("collections")
    }

    /// Get the path for a specific record.
    fn record_path(&self, collection: &Nsid, did: &Did, rkey: &str) -> PathBuf {
        self.repo_collections_dir(did)
            .join(collection.as_str())
            .join(format!("{}.json", rkey))
    }

    /// Get the firehose log path.
    fn firehose_path(&self) -> PathBuf {
        self.pds_dir().join("firehose.jsonl")
    }

    /// Get the firehose lock file path.
    fn firehose_lock_path(&self) -> PathBuf {
        self.pds_dir().join("firehose.lock")
    }

    /// Generate a new record key (TID-style).
    fn generate_rkey(&self) -> String {
        // Use current timestamp in microseconds as a simple TID
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_micros();
        format!("{:x}", now)
    }

    /// Generate a simple CID for a record.
    fn generate_cid(&self, content: &str) -> String {
        // Simple hash-based CID for local use
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        content.hash(&mut hasher);
        format!("bafylocal{:016x}", hasher.finish())
    }

    /// Append an event to the firehose log.
    fn append_firehose(&self, uri: &AtUri, op: FirehoseLogOp) -> Result<()> {
        let firehose_path = self.firehose_path();
        let lock_path = self.firehose_lock_path();

        // Ensure directories exist
        if let Some(parent) = firehose_path.parent() {
            fs::create_dir_all(parent).map_err(|e| Error::Transport(e.into()))?;
        }

        // Acquire exclusive lock
        let lock_file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(false)
            .open(&lock_path)
            .map_err(|e| Error::Transport(e.into()))?;

        lock_file
            .lock_exclusive()
            .map_err(|e| Error::Transport(e.into()))?;

        // Append event
        let event = FirehoseLogEvent {
            uri: uri.to_string(),
            time: Utc::now().to_rfc3339(),
            op,
        };

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&firehose_path)
            .map_err(|e| Error::Transport(e.into()))?;

        let line = serde_json::to_string(&event).map_err(|e| {
            Error::InvalidInput(InvalidInputError::Other {
                message: e.to_string(),
            })
        })?;

        writeln!(file, "{}", line).map_err(|e| Error::Transport(e.into()))?;
        file.sync_data().map_err(|e| Error::Transport(e.into()))?;

        // Release lock (implicit on drop, but explicit for clarity)
        lock_file.unlock().map_err(|e| Error::Transport(e.into()))?;

        Ok(())
    }

    // ========================================================================
    // Account Management (Direct Methods)
    // ========================================================================

    /// Create a new account in the local PDS.
    ///
    /// Returns the generated DID for the new account.
    ///
    /// This is a convenience method for the file-backed PDS.
    #[instrument(skip(self))]
    pub fn create_account_local(&self, handle: &str) -> Result<Did> {
        // Generate a local DID
        let uuid_str = Uuid::new_v4().to_string().replace("-", "");
        let did_str = format!("did:plc:{}", &uuid_str[..24]);
        let did = Did::new(&did_str)?;

        let account = LocalAccount {
            did: did_str.clone(),
            handle: handle.to_string(),
            created_at: Utc::now().to_rfc3339(),
        };

        let account_path = self.account_path(&did);

        // Create account directory
        if let Some(parent) = account_path.parent() {
            fs::create_dir_all(parent).map_err(|e| Error::Transport(e.into()))?;
        }

        // Write account file
        let content = serde_json::to_string_pretty(&account).map_err(|e| {
            Error::InvalidInput(InvalidInputError::Other {
                message: e.to_string(),
            })
        })?;
        fs::write(&account_path, content).map_err(|e| Error::Transport(e.into()))?;

        debug!(did = %did, handle = %handle, "Created local account");

        Ok(did)
    }

    /// Get an account by DID.
    pub fn get_account(&self, did: &Did) -> Result<Option<LocalAccount>> {
        let account_path = self.account_path(did);

        if !account_path.exists() {
            return Ok(None);
        }

        let content = fs::read_to_string(&account_path).map_err(|e| Error::Transport(e.into()))?;
        let account: LocalAccount = serde_json::from_str(&content).map_err(|e| {
            Error::InvalidInput(InvalidInputError::Other {
                message: e.to_string(),
            })
        })?;

        Ok(Some(account))
    }

    /// Remove an account from the local PDS.
    ///
    /// If `delete_records` is true, also removes all records for this account.
    #[instrument(skip(self))]
    pub fn remove_account(&self, did: &Did, delete_records: bool) -> Result<()> {
        let account_dir = self.accounts_dir().join(did.as_str());

        if !account_dir.exists() {
            return Err(Error::Protocol(ProtocolError::new(
                404,
                Some("AccountNotFound".to_string()),
                Some(format!("Account {} not found", did)),
            )));
        }

        // Remove account directory
        fs::remove_dir_all(&account_dir).map_err(|e| Error::Transport(e.into()))?;

        // Optionally remove records (the entire repo directory for this DID)
        if delete_records {
            let repo_dir = self.repos_dir().join(did.as_str());
            if repo_dir.exists() {
                fs::remove_dir_all(&repo_dir).map_err(|e| Error::Transport(e.into()))?;
            }
        }

        debug!(did = %did, "Removed local account");

        Ok(())
    }

    /// List all accounts in the local PDS.
    pub fn list_accounts(&self) -> Result<Vec<LocalAccount>> {
        let accounts_dir = self.accounts_dir();

        if !accounts_dir.exists() {
            return Ok(Vec::new());
        }

        let mut accounts = Vec::new();

        for entry in fs::read_dir(&accounts_dir).map_err(|e| Error::Transport(e.into()))? {
            let entry = entry.map_err(|e| Error::Transport(e.into()))?;
            let account_file = entry.path().join("account.json");

            if account_file.exists() {
                let content =
                    fs::read_to_string(&account_file).map_err(|e| Error::Transport(e.into()))?;
                if let Ok(account) = serde_json::from_str::<LocalAccount>(&content) {
                    accounts.push(account);
                }
            }
        }

        Ok(accounts)
    }

    /// Find an account by handle.
    ///
    /// Returns the account if found, or None if no account with the given handle exists.
    pub fn find_account_by_handle(&self, handle: &str) -> Result<Option<LocalAccount>> {
        let accounts = self.list_accounts()?;
        Ok(accounts.into_iter().find(|a| a.handle == handle))
    }

    /// Helper to get a record without token (for internal use in list_records).
    async fn get_record_internal(&self, uri: &AtUri) -> Result<Record> {
        let path = self.record_path(uri.collection(), uri.repo(), uri.rkey().as_str());

        if !path.exists() {
            return Err(Error::Protocol(ProtocolError::new(
                404,
                Some("RecordNotFound".to_string()),
                Some(format!("Record {} not found", uri)),
            )));
        }

        let content = fs::read_to_string(&path).map_err(|e| Error::Transport(e.into()))?;
        let value: RecordValue = serde_json::from_str(&content).map_err(|e| {
            Error::InvalidInput(InvalidInputError::Other {
                message: e.to_string(),
            })
        })?;

        let cid = self.generate_cid(&content);

        Ok(Record {
            uri: uri.clone(),
            cid,
            value,
        })
    }

    // ========================================================================
    // Firehose
    // ========================================================================

    pub fn firehose_from(&self, _cursor: Option<i64>) -> Result<RepoEventStream> {
        RepoEventStream::from_file(self.root.clone())
    }
}

// Implement conversion from io::Error to TransportError
impl From<std::io::Error> for crate::error::TransportError {
    fn from(err: std::io::Error) -> Self {
        crate::error::TransportError::Http {
            message: format!("IO error: {}", err),
        }
    }
}

impl FilePds {
    #[instrument(skip(self, value))]
    pub(crate) async fn create_record(
        &self,
        repo: &Did,
        collection: &Nsid,
        value: &RecordValue,
        rkey: Option<&str>,
    ) -> Result<AtUri> {
        let rkey = rkey
            .map(|s| s.to_string())
            .unwrap_or_else(|| self.generate_rkey());

        let rkey_validated = Rkey::new(&rkey)?;
        let path = self.record_path(collection, repo, &rkey);

        // Create directories
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| Error::Transport(e.into()))?;
        }

        // Serialize the record value
        let content = serde_json::to_string_pretty(value.as_value()).map_err(|e| {
            Error::InvalidInput(InvalidInputError::Other {
                message: e.to_string(),
            })
        })?;

        // Write atomically (temp file + rename)
        let temp_path = path.with_extension("tmp");
        fs::write(&temp_path, &content).map_err(|e| Error::Transport(e.into()))?;
        fs::rename(&temp_path, &path).map_err(|e| Error::Transport(e.into()))?;

        // Build URI
        let uri = AtUri::from_parts(repo.clone(), collection.clone(), rkey_validated);

        // Append to firehose
        self.append_firehose(&uri, FirehoseLogOp::Create)?;

        debug!(uri = %uri, "Created record");

        Ok(uri)
    }

    #[instrument(skip(self))]
    pub(crate) async fn get_record(&self, uri: &AtUri) -> Result<Record> {
        self.get_record_internal(uri).await
    }

    #[instrument(skip(self))]
    pub(crate) async fn list_records(
        &self,
        repo: &Did,
        collection: &Nsid,
        limit: Option<u32>,
        cursor: Option<&str>,
    ) -> Result<ListRecordsOutput> {
        let dir = self.repo_collections_dir(repo).join(collection.as_str());

        let mut records = Vec::new();
        let limit = limit.unwrap_or(50) as usize;

        if dir.exists() {
            let mut entries: Vec<_> = fs::read_dir(&dir)
                .map_err(|e| Error::Transport(e.into()))?
                .filter_map(|e| e.ok())
                .filter(|e| e.path().extension().is_some_and(|ext| ext == "json"))
                .collect();

            // Sort by filename (rkey)
            entries.sort_by_key(|e| e.file_name());

            // Apply cursor (skip entries before cursor)
            let start_idx = if let Some(cursor) = cursor {
                entries
                    .iter()
                    .position(|e| {
                        e.path()
                            .file_stem()
                            .and_then(|s| s.to_str())
                            .is_some_and(|s| s > cursor)
                    })
                    .unwrap_or(0)
            } else {
                0
            };

            for entry in entries.iter().skip(start_idx).take(limit) {
                let rkey = entry
                    .path()
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("")
                    .to_string();

                let rkey_validated = match Rkey::new(&rkey) {
                    Ok(r) => r,
                    Err(_) => continue, // Skip invalid rkeys
                };

                let uri = AtUri::from_parts(repo.clone(), collection.clone(), rkey_validated);
                if let Ok(record) = self.get_record_internal(&uri).await {
                    records.push(record);
                }
            }
        }

        let cursor = if records.len() == limit {
            records.last().map(|r| r.uri.rkey().as_str().to_string())
        } else {
            None
        };

        Ok(ListRecordsOutput { records, cursor })
    }

    #[instrument(skip(self))]
    pub(crate) async fn delete_record(&self, uri: &AtUri) -> Result<()> {
        let path = self.record_path(uri.collection(), uri.repo(), uri.rkey().as_str());

        if path.exists() {
            fs::remove_file(&path).map_err(|e| Error::Transport(e.into()))?;

            // Append to firehose
            self.append_firehose(uri, FirehoseLogOp::Delete)?;

            debug!(uri = %uri, "Deleted record");
        }

        Ok(())
    }

    #[instrument(skip(self, _password))]
    pub async fn create_account(
        &self,
        handle: &str,
        _password: Option<&str>,
        _email: Option<&str>,
        _invite_code: Option<&str>,
    ) -> Result<CreateAccountOutput> {
        let did = self.create_account_local(handle)?;
        Ok(CreateAccountOutput {
            did,
            handle: handle.to_string(),
        })
    }

    #[instrument(skip(self, _token, _password))]
    pub async fn delete_account(
        &self,
        did: &Did,
        _token: Option<&str>,
        _password: Option<&str>,
    ) -> Result<()> {
        // For file backend, delete the account and all associated records
        self.remove_account(did, true)
    }

    pub async fn login(&self, credentials: Credentials) -> Result<Session> {
        let identifier = credentials.identifier();

        let account = if identifier.starts_with("did:") {
            let did = Did::new(identifier)?;
            self.get_account(&did)?
        } else {
            self.find_account_by_handle(identifier)?
        }
        .ok_or_else(|| {
            AuthError::InvalidCredentials(format!("Account '{}' not found", identifier))
        })?;

        let did = Did::new(&account.did)?;

        Ok(Session::new_file(self.clone(), did))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tempfile::TempDir;

    fn create_test_backend() -> (TempDir, FilePds) {
        let tmp = TempDir::new().unwrap();
        let url = PdsUrl::new(format!("file://{}", tmp.path().display())).unwrap();
        let backend = FilePds::new(tmp.path(), url);
        (tmp, backend)
    }

    #[test]
    fn test_create_account() {
        let (_tmp, backend) = create_test_backend();

        let did = backend.create_account_local("test.local").unwrap();
        assert!(did.as_str().starts_with("did:plc:"));

        let account = backend.get_account(&did).unwrap().unwrap();
        assert_eq!(account.handle, "test.local");
    }

    #[test]
    fn test_remove_account() {
        let (_tmp, backend) = create_test_backend();

        let did = backend.create_account_local("test.local").unwrap();
        assert!(backend.get_account(&did).unwrap().is_some());

        backend.remove_account(&did, false).unwrap();
        assert!(backend.get_account(&did).unwrap().is_none());
    }

    #[test]
    fn test_list_accounts() {
        let (_tmp, backend) = create_test_backend();

        backend.create_account_local("alice.local").unwrap();
        backend.create_account_local("bob.local").unwrap();

        let accounts = backend.list_accounts().unwrap();
        assert_eq!(accounts.len(), 2);
    }

    #[tokio::test]
    async fn test_create_and_get_record() {
        let (_tmp, backend) = create_test_backend();

        let did = Did::new("did:plc:test123").unwrap();
        let collection = Nsid::new("org.test.record").unwrap();
        let value = RecordValue::new(json!({
            "$type": "org.test.record",
            "text": "hello"
        }))
        .unwrap();

        let uri = backend
            .create_record(&did, &collection, &value, Some("testrkey"))
            .await
            .unwrap();

        assert_eq!(uri.rkey().as_str(), "testrkey");

        let record = backend.get_record(&uri).await.unwrap();
        assert_eq!(record.value.record_type(), "org.test.record");
        assert_eq!(record.value.get("text").unwrap(), "hello");
    }

    #[tokio::test]
    async fn test_list_records() {
        let (_tmp, backend) = create_test_backend();

        let did = Did::new("did:plc:test123").unwrap();
        let collection = Nsid::new("org.test.record").unwrap();

        // Create a few records
        for i in 0..5 {
            let value = RecordValue::new(json!({
                "$type": "org.test.record",
                "index": i
            }))
            .unwrap();
            backend
                .create_record(&did, &collection, &value, Some(&format!("rec{:03}", i)))
                .await
                .unwrap();
        }

        let result = backend
            .list_records(&did, &collection, Some(3), None)
            .await
            .unwrap();
        assert_eq!(result.records.len(), 3);
        assert!(result.cursor.is_some());
    }

    #[tokio::test]
    async fn test_delete_record() {
        let (_tmp, backend) = create_test_backend();

        let did = Did::new("did:plc:test123").unwrap();
        let collection = Nsid::new("org.test.record").unwrap();
        let value = RecordValue::new(json!({
            "$type": "org.test.record",
            "text": "to delete"
        }))
        .unwrap();

        let uri = backend
            .create_record(&did, &collection, &value, Some("todelete"))
            .await
            .unwrap();

        // Record exists
        assert!(backend.get_record(&uri).await.is_ok());

        // Delete it
        backend.delete_record(&uri).await.unwrap();

        // Record should be gone
        assert!(backend.get_record(&uri).await.is_err());
    }

    #[tokio::test]
    async fn test_firehose_append() {
        let (tmp, backend) = create_test_backend();

        let did = Did::new("did:plc:test123").unwrap();
        let collection = Nsid::new("org.test.record").unwrap();
        let value = RecordValue::new(json!({
            "$type": "org.test.record"
        }))
        .unwrap();

        backend
            .create_record(&did, &collection, &value, None)
            .await
            .unwrap();

        let firehose_path = tmp.path().join("pds/firehose.jsonl");
        assert!(firehose_path.exists());

        let content = std::fs::read_to_string(&firehose_path).unwrap();
        let lines: Vec<&str> = content.lines().collect();
        assert_eq!(lines.len(), 1);

        // Verify it's valid JSON
        let event: serde_json::Value = serde_json::from_str(lines[0]).unwrap();
        assert!(event["uri"].as_str().unwrap().starts_with("at://"));
    }

    #[tokio::test]
    async fn test_create_account_via_trait() {
        let (_tmp, backend) = create_test_backend();

        let output = backend
            .create_account("test.local", None, None, None)
            .await
            .unwrap();
        assert!(output.did.as_str().starts_with("did:plc:"));
        assert_eq!(output.handle, "test.local");

        let account = backend.get_account(&output.did).unwrap().unwrap();
        assert_eq!(account.handle, "test.local");
    }

    #[tokio::test]
    async fn test_delete_account_via_trait() {
        let (_tmp, backend) = create_test_backend();

        let output = backend
            .create_account("test.local", None, None, None)
            .await
            .unwrap();
        assert!(backend.get_account(&output.did).unwrap().is_some());

        backend
            .delete_account(&output.did, None, None)
            .await
            .unwrap();
        assert!(backend.get_account(&output.did).unwrap().is_none());
    }
}
