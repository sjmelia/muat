//! Filesystem storage for the file-backed PDS.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use chrono::Utc;
use fs2::FileExt;
use serde::{Deserialize, Serialize};
use tracing::{debug, instrument};
use uuid::Uuid;

use muat_core::Result;
use muat_core::error::{Error, InvalidInputError, ProtocolError, TransportError};
use muat_core::repo::{ListRecordsOutput, Record, RecordValue};
use muat_core::types::{AtUri, Did, Nsid, Rkey};

fn map_io(err: std::io::Error) -> Error {
    Error::Transport(TransportError::Http {
        message: format!("IO error: {}", err),
    })
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
    /// Password hash (bcrypt).
    pub password_hash: String,
}

/// An event in the firehose log.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct FirehoseLogEvent {
    /// The AT URI of the affected record.
    pub uri: String,
    /// ISO 8601 timestamp.
    pub time: String,
    /// The operation type.
    pub op: FirehoseLogOp,
}

/// The type of firehose operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum FirehoseLogOp {
    /// A record was created.
    Create,
    /// A record was deleted.
    Delete,
}

/// Filesystem-backed storage for a local PDS.
#[derive(Debug, Clone)]
pub struct FileStore {
    root: PathBuf,
}

impl FileStore {
    /// Create a new file store at the given root directory.
    pub fn new(root: impl AsRef<Path>) -> Self {
        Self {
            root: root.as_ref().to_path_buf(),
        }
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

    /// Convert a DID into a filesystem-safe directory name.
    fn did_dir_name(did: &Did) -> String {
        // Windows does not allow ':' in path segments.
        did.as_str().replace(':', "_")
    }

    /// Get the path for a specific account.
    fn account_path(&self, did: &Did) -> PathBuf {
        self.accounts_dir()
            .join(Self::did_dir_name(did))
            .join("account.json")
    }

    /// Get the collections directory for a specific repo (DID).
    fn repo_collections_dir(&self, did: &Did) -> PathBuf {
        self.repos_dir()
            .join(Self::did_dir_name(did))
            .join("collections")
    }

    /// Get the path for a specific record.
    fn record_path(&self, collection: &Nsid, did: &Did, rkey: &str) -> PathBuf {
        self.repo_collections_dir(did)
            .join(collection.as_str())
            .join(format!("{}.json", rkey))
    }

    /// Get the firehose log path.
    pub(crate) fn firehose_path(&self) -> PathBuf {
        self.pds_dir().join("firehose.jsonl")
    }

    /// Get the firehose lock file path.
    fn firehose_lock_path(&self) -> PathBuf {
        self.pds_dir().join("firehose.lock")
    }

    /// Generate a new record key (TID-style).
    fn generate_rkey(&self) -> String {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_micros();
        format!("{:x}", now)
    }

    /// Generate a simple CID for a record.
    fn generate_cid(&self, content: &str) -> String {
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

        if let Some(parent) = firehose_path.parent() {
            fs::create_dir_all(parent).map_err(map_io)?;
        }

        let lock_file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(false)
            .open(&lock_path)
            .map_err(map_io)?;

        lock_file.lock_exclusive().map_err(map_io)?;

        let event = FirehoseLogEvent {
            uri: uri.to_string(),
            time: Utc::now().to_rfc3339(),
            op,
        };

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&firehose_path)
            .map_err(map_io)?;

        let line = serde_json::to_string(&event).map_err(|e| {
            Error::InvalidInput(InvalidInputError::Other {
                message: e.to_string(),
            })
        })?;

        writeln!(file, "{}", line).map_err(map_io)?;
        file.sync_data().map_err(map_io)?;

        lock_file.unlock().map_err(map_io)?;

        Ok(())
    }

    // ========================================================================
    // Account Management
    // ========================================================================

    #[instrument(skip(self, password_hash))]
    pub fn create_account(&self, handle: &str, password_hash: &str) -> Result<Did> {
        let uuid_str = Uuid::new_v4().to_string().replace("-", "");
        let did_str = format!("did:plc:{}", &uuid_str[..24]);
        let did = Did::new(&did_str)?;

        let account = LocalAccount {
            did: did_str.clone(),
            handle: handle.to_string(),
            created_at: Utc::now().to_rfc3339(),
            password_hash: password_hash.to_string(),
        };

        let account_path = self.account_path(&did);

        if let Some(parent) = account_path.parent() {
            fs::create_dir_all(parent).map_err(map_io)?;
        }

        let content = serde_json::to_string_pretty(&account).map_err(|e| {
            Error::InvalidInput(InvalidInputError::Other {
                message: e.to_string(),
            })
        })?;
        fs::write(&account_path, content).map_err(map_io)?;

        debug!(did = %did, handle = %handle, "Created local account");

        Ok(did)
    }

    pub fn get_account(&self, did: &Did) -> Result<Option<LocalAccount>> {
        let account_path = self.account_path(did);

        if !account_path.exists() {
            return Ok(None);
        }

        let content = fs::read_to_string(&account_path).map_err(map_io)?;
        let account: LocalAccount = serde_json::from_str(&content).map_err(|e| {
            Error::InvalidInput(InvalidInputError::Other {
                message: e.to_string(),
            })
        })?;

        Ok(Some(account))
    }

    #[instrument(skip(self))]
    pub fn remove_account(&self, did: &Did, delete_records: bool) -> Result<()> {
        let account_dir = self.accounts_dir().join(Self::did_dir_name(did));

        if !account_dir.exists() {
            return Err(Error::Protocol(ProtocolError::new(
                404,
                Some("AccountNotFound".to_string()),
                Some(format!("Account {} not found", did)),
            )));
        }

        fs::remove_dir_all(&account_dir).map_err(map_io)?;

        if delete_records {
            let repo_dir = self.repos_dir().join(Self::did_dir_name(did));
            if repo_dir.exists() {
                fs::remove_dir_all(&repo_dir).map_err(map_io)?;
            }
        }

        debug!(did = %did, "Removed local account");

        Ok(())
    }

    pub fn list_accounts(&self) -> Result<Vec<LocalAccount>> {
        let accounts_dir = self.accounts_dir();

        if !accounts_dir.exists() {
            return Ok(Vec::new());
        }

        let mut accounts = Vec::new();

        for entry in fs::read_dir(&accounts_dir).map_err(map_io)? {
            let entry = entry.map_err(map_io)?;
            let account_file = entry.path().join("account.json");

            if account_file.exists() {
                let content = fs::read_to_string(&account_file).map_err(map_io)?;
                if let Ok(account) = serde_json::from_str::<LocalAccount>(&content) {
                    accounts.push(account);
                }
            }
        }

        Ok(accounts)
    }

    pub fn find_account_by_handle(&self, handle: &str) -> Result<Option<LocalAccount>> {
        let accounts = self.list_accounts()?;
        Ok(accounts.into_iter().find(|a| a.handle == handle))
    }

    // ========================================================================
    // Record Operations
    // ========================================================================

    async fn get_record_internal(&self, uri: &AtUri) -> Result<Record> {
        let path = self.record_path(uri.collection(), uri.repo(), uri.rkey().as_str());

        if !path.exists() {
            return Err(Error::Protocol(ProtocolError::new(
                404,
                Some("RecordNotFound".to_string()),
                Some(format!("Record {} not found", uri)),
            )));
        }

        let content = fs::read_to_string(&path).map_err(map_io)?;
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

    #[instrument(skip(self, value))]
    pub async fn create_record(
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

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(map_io)?;
        }

        let content = serde_json::to_string_pretty(value.as_value()).map_err(|e| {
            Error::InvalidInput(InvalidInputError::Other {
                message: e.to_string(),
            })
        })?;

        let temp_path = path.with_extension("tmp");
        fs::write(&temp_path, &content).map_err(map_io)?;
        fs::rename(&temp_path, &path).map_err(map_io)?;

        let uri = AtUri::from_parts(repo.clone(), collection.clone(), rkey_validated);

        self.append_firehose(&uri, FirehoseLogOp::Create)?;

        debug!(uri = %uri, "Created record");

        Ok(uri)
    }

    #[instrument(skip(self))]
    pub async fn get_record(&self, uri: &AtUri) -> Result<Record> {
        self.get_record_internal(uri).await
    }

    #[instrument(skip(self))]
    pub async fn list_records(
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
                .map_err(map_io)?
                .filter_map(|e| e.ok())
                .filter(|e| e.path().extension().is_some_and(|ext| ext == "json"))
                .collect();

            entries.sort_by_key(|e| e.file_name());

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
                    Err(_) => continue,
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
    pub async fn delete_record(&self, uri: &AtUri) -> Result<()> {
        let path = self.record_path(uri.collection(), uri.repo(), uri.rkey().as_str());

        if path.exists() {
            fs::remove_file(&path).map_err(map_io)?;

            self.append_firehose(uri, FirehoseLogOp::Delete)?;

            debug!(uri = %uri, "Deleted record");
        }

        Ok(())
    }
}
