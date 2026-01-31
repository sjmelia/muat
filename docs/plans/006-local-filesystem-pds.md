# Implementation Plan: PRD-006 Local Filesystem PDS Backend

## Overview

This plan details the implementation of PRD-006, which adds a `file://` PDS URL mode to `muat` enabling local-only development and testing without a network PDS. Records are stored on the filesystem with an append-only firehose log.

## Goals Summary

| Goal | Description |
|------|-------------|
| G1 | PDS backend abstraction trait |
| G2 | `file://` URL support with filesystem storage |
| G3 | Append-only firehose with cross-process locking |
| G4 | CLI `create-account` and `remove-account` commands |
| G5 | Integration tests including concurrency |

---

## G1: PDS Backend Abstraction

### Design

Introduce a trait that abstracts PDS operations, allowing both HTTP and filesystem backends.

**File:** `crates/muat/src/backend/mod.rs`

```rust
pub mod http;
pub mod file;

use async_trait::async_trait;
use crate::error::Result;
use crate::types::{AtUri, Did, Nsid, PdsUrl};
use crate::repo::{ListRecordsOutput, Record, RecordValue};

/// Backend for PDS operations.
#[async_trait]
pub trait PdsBackend: Send + Sync {
    /// Create a record in the repository.
    async fn create_record(
        &self,
        repo: &Did,
        collection: &Nsid,
        value: &RecordValue,
        rkey: Option<&str>,
    ) -> Result<AtUri>;

    /// Get a record from the repository.
    async fn get_record(&self, uri: &AtUri) -> Result<Record>;

    /// List records in a collection.
    async fn list_records(
        &self,
        repo: &Did,
        collection: &Nsid,
        limit: Option<u32>,
        cursor: Option<&str>,
    ) -> Result<ListRecordsOutput>;

    /// Delete a record from the repository.
    async fn delete_record(&self, uri: &AtUri) -> Result<()>;
}

/// Select backend based on PDS URL scheme.
pub fn select_backend(pds: &PdsUrl) -> Box<dyn PdsBackend> {
    match pds.scheme() {
        "file" => Box::new(file::FilePdsBackend::new(pds.path())),
        _ => Box::new(http::HttpPdsBackend::new(pds.clone())),
    }
}
```

---

## G2: Filesystem Backend

### Directory Structure

```
$ROOT/pds/
├── accounts/
│   └── <did>/
│       └── account.json
├── collections/
│   └── <collection>/
│       └── <did>/
│           └── <rkey>.json
└── firehose.jsonl
```

### Implementation

**File:** `crates/muat/src/backend/file.rs`

```rust
use std::path::{Path, PathBuf};
use std::fs;
use crate::error::{Error, Result};
use crate::types::{AtUri, Did, Nsid, Rkey};
use crate::repo::{ListRecordsOutput, Record, RecordValue};
use super::PdsBackend;

pub struct FilePdsBackend {
    root: PathBuf,
}

impl FilePdsBackend {
    pub fn new(root: impl AsRef<Path>) -> Self {
        Self {
            root: root.as_ref().to_path_buf(),
        }
    }

    fn collections_dir(&self) -> PathBuf {
        self.root.join("pds").join("collections")
    }

    fn record_path(&self, collection: &Nsid, did: &Did, rkey: &str) -> PathBuf {
        self.collections_dir()
            .join(collection.as_str())
            .join(did.as_str())
            .join(format!("{}.json", rkey))
    }

    fn accounts_dir(&self) -> PathBuf {
        self.root.join("pds").join("accounts")
    }

    fn account_path(&self, did: &Did) -> PathBuf {
        self.accounts_dir().join(did.as_str()).join("account.json")
    }

    fn firehose_path(&self) -> PathBuf {
        self.root.join("pds").join("firehose.jsonl")
    }

    fn firehose_lock_path(&self) -> PathBuf {
        self.root.join("pds").join("firehose.lock")
    }

    fn generate_rkey(&self) -> String {
        // Generate TID-style rkey
        use std::time::{SystemTime, UNIX_EPOCH};
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_micros();
        format!("{:x}", now)
    }
}

#[async_trait]
impl PdsBackend for FilePdsBackend {
    async fn create_record(
        &self,
        repo: &Did,
        collection: &Nsid,
        value: &RecordValue,
        rkey: Option<&str>,
    ) -> Result<AtUri> {
        let rkey = rkey.map(|s| s.to_string())
            .unwrap_or_else(|| self.generate_rkey());

        let path = self.record_path(collection, repo, &rkey);

        // Create directories
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        // Write record atomically (write to temp, then rename)
        let temp_path = path.with_extension("tmp");
        let content = serde_json::to_string_pretty(value.as_value())?;
        fs::write(&temp_path, &content)?;
        fs::rename(&temp_path, &path)?;

        // Build URI
        let uri = AtUri::new(repo, collection, &rkey)?;

        // Append to firehose
        self.append_firehose(&uri, value).await?;

        Ok(uri)
    }

    async fn get_record(&self, uri: &AtUri) -> Result<Record> {
        let path = self.record_path(uri.collection(), uri.repo(), uri.rkey());

        if !path.exists() {
            return Err(Error::Protocol(/* record not found */));
        }

        let content = fs::read_to_string(&path)?;
        let value: RecordValue = serde_json::from_str(&content)?;

        // Generate CID (simplified - just hash the content)
        let cid = format!("bafylocal{:x}", hash(&content));

        Ok(Record {
            uri: uri.clone(),
            cid,
            value,
        })
    }

    async fn list_records(
        &self,
        repo: &Did,
        collection: &Nsid,
        limit: Option<u32>,
        cursor: Option<&str>,
    ) -> Result<ListRecordsOutput> {
        let dir = self.collections_dir()
            .join(collection.as_str())
            .join(repo.as_str());

        let mut records = Vec::new();

        if dir.exists() {
            let mut entries: Vec<_> = fs::read_dir(&dir)?
                .filter_map(|e| e.ok())
                .filter(|e| e.path().extension() == Some("json".as_ref()))
                .collect();

            // Sort by filename (rkey)
            entries.sort_by_key(|e| e.file_name());

            // Apply cursor (skip entries before cursor)
            let start_idx = cursor
                .and_then(|c| entries.iter().position(|e| {
                    e.path().file_stem()
                        .and_then(|s| s.to_str())
                        .map(|s| s > c)
                        .unwrap_or(false)
                }))
                .unwrap_or(0);

            let limit = limit.unwrap_or(50) as usize;

            for entry in entries.iter().skip(start_idx).take(limit) {
                let rkey = entry.path()
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("")
                    .to_string();

                let uri = AtUri::new(repo, collection, &rkey)?;
                let record = self.get_record(&uri).await?;
                records.push(record);
            }
        }

        let cursor = if records.len() == limit.unwrap_or(50) as usize {
            records.last().map(|r| r.uri.rkey().to_string())
        } else {
            None
        };

        Ok(ListRecordsOutput { records, cursor })
    }

    async fn delete_record(&self, uri: &AtUri) -> Result<()> {
        let path = self.record_path(uri.collection(), uri.repo(), uri.rkey());

        if path.exists() {
            fs::remove_file(&path)?;
        }

        Ok(())
    }
}
```

---

## G3: Firehose with Cross-Process Locking

### Implementation

**File:** `crates/muat/src/backend/file.rs` (continued)

```rust
use fs2::FileExt;  // Cross-platform file locking
use std::io::{Write, BufWriter};

impl FilePdsBackend {
    async fn append_firehose(&self, uri: &AtUri, value: &RecordValue) -> Result<()> {
        let firehose_path = self.firehose_path();
        let lock_path = self.firehose_lock_path();

        // Ensure parent directory exists
        if let Some(parent) = firehose_path.parent() {
            fs::create_dir_all(parent)?;
        }

        // Acquire exclusive lock
        let lock_file = fs::OpenOptions::new()
            .create(true)
            .write(true)
            .open(&lock_path)?;
        lock_file.lock_exclusive()?;

        // Append event
        let event = FirehoseEvent {
            uri: uri.to_string(),
            time: chrono::Utc::now().to_rfc3339(),
            value: value.as_value().clone(),
        };

        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&firehose_path)?;

        let line = serde_json::to_string(&event)?;
        writeln!(file, "{}", line)?;
        file.sync_data()?;

        // Release lock (automatic on drop, but explicit for clarity)
        lock_file.unlock()?;

        Ok(())
    }
}

#[derive(Debug, Serialize)]
struct FirehoseEvent {
    uri: String,
    time: String,
    value: serde_json::Value,
}
```

### Dependencies

Add to `crates/muat/Cargo.toml`:

```toml
[dependencies]
fs2 = "0.4"  # Cross-platform file locking
```

---

## G4: CLI Account Commands

### Create Account

**File:** `crates/atproto-cli/src/commands/pds/create_account.rs`

```
atproto pds create-account <handle> [--pds <url>]
```

When PDS URL is `file://`:
- Creates account directory at `$ROOT/pds/accounts/<did>/`
- Generates a DID (e.g., `did:plc:<random>` for local use)
- Writes `account.json` with handle and metadata
- Sets up session for the new account

When PDS URL is `http(s)://`:
- Fails with clear error: "Remote account creation not supported by this CLI"

```rust
#[derive(Args, Debug)]
pub struct CreateAccountArgs {
    /// Handle for the new account
    pub handle: String,

    /// PDS URL (defaults to configured or file://./pds)
    #[arg(long)]
    pub pds: Option<String>,
}

pub async fn handle(args: CreateAccountArgs) -> Result<()> {
    let pds_url = args.pds
        .map(|s| PdsUrl::new(&s))
        .transpose()?
        .unwrap_or_else(|| PdsUrl::new("file://./pds").unwrap());

    if pds_url.scheme() != "file" {
        anyhow::bail!("Remote PDS account creation is not supported by this CLI. \
                       Use the PDS web interface or official tools instead.");
    }

    let backend = FilePdsBackend::new(pds_url.path());
    let did = backend.create_account(&args.handle).await?;

    eprintln!("Created account: {}", did);
    eprintln!("Handle: {}", args.handle);

    Ok(())
}
```

### Remove Account

**File:** `crates/atproto-cli/src/commands/pds/remove_account.rs`

```
atproto pds remove-account <did> [--pds <url>]
```

When PDS URL is `file://`:
- Removes account directory
- Optionally removes associated records (with `--delete-records` flag)
- Requires confirmation unless `--force`

When PDS URL is `http(s)://`:
- Fails with clear error

```rust
#[derive(Args, Debug)]
pub struct RemoveAccountArgs {
    /// DID of the account to remove
    pub did: String,

    /// Also delete all records for this account
    #[arg(long)]
    pub delete_records: bool,

    /// Skip confirmation prompt
    #[arg(long)]
    pub force: bool,

    /// PDS URL
    #[arg(long)]
    pub pds: Option<String>,
}

pub async fn handle(args: RemoveAccountArgs) -> Result<()> {
    let pds_url = /* ... */;

    if pds_url.scheme() != "file" {
        anyhow::bail!("Remote PDS account removal is not supported by this CLI.");
    }

    if !args.force {
        eprintln!("This will remove account {}. Continue? [y/N]", args.did);
        // Read confirmation
    }

    let backend = FilePdsBackend::new(pds_url.path());
    backend.remove_account(&Did::new(&args.did)?, args.delete_records).await?;

    eprintln!("Account removed: {}", args.did);

    Ok(())
}
```

---

## G5: Integration Tests

### Local Store Basics

**File:** `crates/muat/tests/file_backend_tests.rs`

```rust
use tempfile::TempDir;

#[tokio::test]
async fn test_create_account() {
    let tmp = TempDir::new().unwrap();
    let backend = FilePdsBackend::new(tmp.path());

    let did = backend.create_account("test.local").await.unwrap();
    assert!(did.as_str().starts_with("did:"));

    let account_path = tmp.path()
        .join("pds/accounts")
        .join(did.as_str())
        .join("account.json");
    assert!(account_path.exists());
}

#[tokio::test]
async fn test_create_record_writes_file() {
    let tmp = TempDir::new().unwrap();
    let backend = FilePdsBackend::new(tmp.path());
    let did = Did::new("did:plc:test123").unwrap();
    let collection = Nsid::new("org.test.record").unwrap();

    let value = RecordValue::new(json!({
        "$type": "org.test.record",
        "text": "hello"
    })).unwrap();

    let uri = backend.create_record(&did, &collection, &value, Some("testrkey")).await.unwrap();

    let record_path = tmp.path()
        .join("pds/collections/org.test.record/did:plc:test123/testrkey.json");
    assert!(record_path.exists());

    let content = std::fs::read_to_string(&record_path).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&content).unwrap();
    assert_eq!(parsed["$type"], "org.test.record");
}
```

### Firehose Append

```rust
#[tokio::test]
async fn test_firehose_append() {
    let tmp = TempDir::new().unwrap();
    let backend = FilePdsBackend::new(tmp.path());
    let did = Did::new("did:plc:test123").unwrap();
    let collection = Nsid::new("org.test.record").unwrap();

    let value = RecordValue::new(json!({
        "$type": "org.test.record"
    })).unwrap();

    backend.create_record(&did, &collection, &value, None).await.unwrap();

    let firehose_path = tmp.path().join("pds/firehose.jsonl");
    assert!(firehose_path.exists());

    let content = std::fs::read_to_string(&firehose_path).unwrap();
    let lines: Vec<&str> = content.lines().collect();
    assert_eq!(lines.len(), 1);

    let event: serde_json::Value = serde_json::from_str(lines[0]).unwrap();
    assert!(event["uri"].as_str().unwrap().starts_with("at://"));
}
```

### Concurrency Tests

```rust
#[tokio::test]
async fn test_concurrent_firehose_writes() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path().to_path_buf();

    let handles: Vec<_> = (0..10).map(|i| {
        let root = root.clone();
        tokio::spawn(async move {
            let backend = FilePdsBackend::new(&root);
            let did = Did::new(&format!("did:plc:test{}", i)).unwrap();
            let collection = Nsid::new("org.test.record").unwrap();

            for j in 0..10 {
                let value = RecordValue::new(json!({
                    "$type": "org.test.record",
                    "index": j
                })).unwrap();
                backend.create_record(&did, &collection, &value, None).await.unwrap();
            }
        })
    }).collect();

    for handle in handles {
        handle.await.unwrap();
    }

    // Verify firehose integrity
    let firehose_path = tmp.path().join("pds/firehose.jsonl");
    let content = std::fs::read_to_string(&firehose_path).unwrap();
    let lines: Vec<&str> = content.lines().collect();

    // 10 threads * 10 records = 100 lines
    assert_eq!(lines.len(), 100);

    // Every line must be valid JSON
    for line in &lines {
        let parsed: Result<serde_json::Value, _> = serde_json::from_str(line);
        assert!(parsed.is_ok(), "Invalid JSON line: {}", line);
    }
}
```

---

## Files to Create/Modify

| File | Action | Description |
|------|--------|-------------|
| `crates/muat/src/backend/mod.rs` | Create | Backend trait + selection |
| `crates/muat/src/backend/http.rs` | Create | Wrap existing HTTP code |
| `crates/muat/src/backend/file.rs` | Create | Filesystem backend |
| `crates/muat/src/types/pds_url.rs` | Modify | Support `file://` scheme |
| `crates/muat/src/lib.rs` | Modify | Export backend module |
| `crates/muat/Cargo.toml` | Modify | Add fs2 dependency |
| `crates/atproto-cli/src/commands/pds/create_account.rs` | Create | CLI command |
| `crates/atproto-cli/src/commands/pds/remove_account.rs` | Create | CLI command |
| `crates/atproto-cli/src/commands/pds/mod.rs` | Modify | Add new commands |
| `crates/muat/tests/file_backend_tests.rs` | Create | Backend unit tests |

---

## Implementation Order

1. **G1** - Backend trait abstraction (enables G2)
2. **G2** - Filesystem backend implementation
3. **G3** - Firehose locking (critical for correctness)
4. **G4** - CLI account commands
5. **G5** - Integration tests

---

## PdsUrl Changes

Update `PdsUrl` to support `file://` scheme:

```rust
impl PdsUrl {
    pub fn new(url: &str) -> Result<Self, Error> {
        let url = Url::parse(url)?;

        match url.scheme() {
            "https" | "http" | "file" => Ok(Self(url)),
            _ => Err(Error::InvalidInput(/* ... */)),
        }
    }

    pub fn scheme(&self) -> &str {
        self.0.scheme()
    }

    pub fn path(&self) -> &Path {
        Path::new(self.0.path())
    }
}
```

---

## Success Criteria

- [ ] `PdsBackend` trait exists with HTTP and File implementations
- [ ] `file://` URLs route to `FilePdsBackend`
- [ ] Records stored at `$ROOT/pds/collections/<collection>/<did>/<rkey>.json`
- [ ] Firehose appends atomically with cross-process locking
- [ ] `atproto pds create-account` works for `file://` URLs
- [ ] `atproto pds remove-account` works for `file://` URLs
- [ ] Both commands fail clearly for `http(s)://` URLs
- [ ] Concurrent firehose writes don't corrupt the file
- [ ] Integration tests pass for local store and concurrency
