# muat

Core AT Protocol library for Rust.

## Overview

`muat` provides foundational AT Protocol primitives with a session-centric API. All authenticated operations flow through a `Session` object, enforcing proper capability-based access control.

## Features

- **Strong typing** - Protocol types (`Did`, `Nsid`, `AtUri`, `PdsUrl`, `Rkey`, `RecordValue`) are validated at construction
- **Session-scoped auth** - All authenticated operations require a `Session`
- **RecordValue type** - Guarantees record payloads are valid JSON objects with `$type` field
- **PDS abstraction** - `Pds` provides a uniform interface over file and network PDS instances
- **Local PDS** - Use `file://` URLs for offline development without a network PDS
- **Network PDS** - Uses XRPC for remote PDS instances
- **Thread-safe** - `Session` uses `Arc<RwLock<...>>` internally for safe sharing
- **Uniform firehose** - `Pds::firehose()` returns an async `Stream` for both file:// and https:// PDS URLs

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
muat = { path = "../muat" }  # Or from crates.io when published
```

## Quick Start

```rust
use muat::{Credentials, Pds, PdsUrl, Nsid};

#[tokio::main]
async fn main() -> Result<(), muat::Error> {
    // Connect to a PDS
    let pds_url = PdsUrl::new("https://bsky.social")?;
    let pds = Pds::open(pds_url);
    let credentials = Credentials::new("alice.bsky.social", "app-password");

    // Create a session
    let session = pds.login(credentials).await?;
    println!("Logged in as: {}", session.did());

    // List records
    let collection = Nsid::new("app.bsky.feed.post")?;
    let records = session.list_records(session.did(), &collection, Some(10), None).await?;

    for record in records.records {
        println!("{}: {:?}", record.uri, record.value);
    }

    Ok(())
}
```

## Core Types

| Type | Description | Example |
|------|-------------|---------|
| `Did` | Decentralized Identifier | `did:plc:z72i7hdynmk6r22z27h6tvur` |
| `Nsid` | Namespaced Identifier (collection) | `app.bsky.feed.post` |
| `AtUri` | AT Protocol URI | `at://did:plc:.../app.bsky.feed.post/...` |
| `PdsUrl` | PDS URL (network or local) | `https://bsky.social`, `file:///tmp/pds` |
| `Pds` | PDS handle (file or network) | - |
| `Rkey` | Record key | `3jui7kd54zh2y` |
| `RecordValue` | Validated record payload | `{"$type": "app.bsky.feed.post", ...}` |
| `Session` | Authenticated session | - |
| `Credentials` | Login identifier + password | - |

## API Reference

### PDS Operations

```rust
// Open a PDS handle
let pds = Pds::open(pds_url);

// Login
let session = pds.login(credentials).await?;

// Firehose (PDS-scoped)
let stream = pds.firehose()?;
```

### Session Operations

```rust
// Refresh tokens (network sessions)
session.refresh().await?;

// Access session info
session.did()  // Returns &Did
session.pds()  // Returns &PdsUrl
```

### Repository Operations

```rust
// List records in a collection
let result = session.list_records(&did, &nsid, Some(limit), cursor).await?;

// Get a specific record
let record = session.get_record(&at_uri).await?;

// Create a record with RecordValue (preferred)
use muat::RecordValue;
use serde_json::json;

let value = RecordValue::with_type("org.example.record", json!({
    "text": "Hello, world!"
}))?;
let uri = session.create_record(&nsid, &value).await?;

// Create a record (raw JSON, for advanced use)
let response = session.create_record_raw(&nsid, record_value).await?;

// Delete a record
session.delete_record(&at_uri).await?;
```

## PDS Architecture

`muat` uses a `Pds` handle for PDS-scoped operations. It selects the
implementation based on the URL scheme:

- `file://` → `FilePds`
- `http://` / `https://` → `XrpcPds`

### Local Filesystem PDS

For development and testing, you can use a local filesystem-backed PDS:

```rust
use muat::{Credentials, Pds, PdsUrl, Nsid, RecordValue};
use serde_json::json;

let pds_url = PdsUrl::new("file:///tmp/test-pds")?;
let pds = Pds::open(pds_url);

let output = pds.create_account("alice.local", None, None, None).await?;
let session = pds.login(Credentials::new("alice.local", "unused")).await?;

let collection = Nsid::new("org.test.record")?;
let value = RecordValue::new(json!({
    "$type": "org.test.record",
    "text": "test"
}))?;

let uri = session.create_record(&collection, &value).await?;
```

Directory structure (repo-centric):
```
$ROOT/pds/
├── accounts/<did>/account.json
├── repos/<did>/collections/<collection>/<rkey>.json
└── firehose.jsonl
```

### Firehose Streaming

Subscribe to repository events using the uniform firehose API:

```rust
use futures_util::StreamExt;
use muat::{Pds, PdsUrl};
use muat::repo::RepoEvent;

let pds = Pds::open(PdsUrl::new("https://bsky.social")?);
let mut stream = pds.firehose()?;

while let Some(result) = stream.next().await {
    match result {
        Ok(RepoEvent::Commit(commit)) => {
            for op in commit.ops {
                println!("{}:{} {}", commit.repo, op.path, op.action);
            }
        }
        Ok(event) => println!("Event: {:?}", event),
        Err(e) => eprintln!("Error: {}", e),
    }
}
```

## Error Handling

`muat` provides a unified `Error` type with variants for:

- `Transport` - Network, DNS, TLS, timeout errors
- `Auth` - Invalid credentials, expired session
- `Protocol` - Non-2xx responses, XRPC error envelopes
- `InvalidInput` - NSID/URI parse failures

```rust
use muat::Error;

match session.get_record(&uri).await {
    Ok(record) => println!("{:?}", record.value),
    Err(Error::Auth(msg)) => eprintln!("Auth error: {}", msg),
    Err(Error::Protocol { status, message, .. }) => {
        eprintln!("Protocol error {}: {}", status, message);
    }
    Err(e) => eprintln!("Error: {}", e),
}
```

## Design Principles

1. **Session-scoped auth** - Authenticated operations require a `Session`; no free functions for authenticated endpoints
2. **PDS abstraction** - `Pds` provides a uniform entry point for file and network PDS instances
3. **Strong types at API boundaries** - Use `Nsid`, `AtUri`, `Did`, etc., not `String`
4. **Schema-agnostic record values** - The protocol layer does not interpret lexicon payloads
5. **Explicitness over magic** - No hidden global state, no silent retries, no implicit defaults
6. **Secrets are never logged** - Custom `Debug` implementations redact sensitive fields

## Testing

```bash
# Run unit tests
cargo test -p muat

# Run mock PDS tests
cargo test -p muat --test mock_pds
```

## License

MIT OR Apache-2.0

## See Also

- [Muat workspace README](../../README.md)
- [Invariants documentation](docs/Invariants.md)
- [AT Protocol Specification](https://atproto.com/specs/atp)
