# muat

Core AT Protocol library for Rust.

## Overview

`muat` provides foundational AT Protocol primitives with a session-centric API. All authenticated operations flow through a `Session` object, enforcing proper capability-based access control.

## Features

- **Strong typing** - Protocol types (`Did`, `Nsid`, `AtUri`, `PdsUrl`, `Rkey`, `RecordValue`) are validated at construction
- **Session-centric API** - All authenticated operations require a `Session`
- **RecordValue type** - Guarantees record payloads are valid JSON objects with `$type` field
- **Local PDS backend** - Use `file://` URLs for offline development without a network PDS
- **Thread-safe** - `Session` uses `Arc<RwLock<...>>` internally for safe sharing
- **Streaming support** - Subscribe to repository events via WebSocket

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
muat = { path = "../muat" }  # Or from crates.io when published
```

## Quick Start

```rust
use muat::{Session, Credentials, PdsUrl, Nsid};

#[tokio::main]
async fn main() -> Result<(), muat::Error> {
    // Connect to a PDS
    let pds = PdsUrl::new("https://bsky.social")?;
    let credentials = Credentials::new("alice.bsky.social", "app-password");

    // Create a session
    let session = Session::login(&pds, credentials).await?;
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
| `Rkey` | Record key | `3jui7kd54zh2y` |
| `RecordValue` | Validated record payload | `{"$type": "app.bsky.feed.post", ...}` |
| `Session` | Authenticated session | - |
| `Credentials` | Login identifier + password | - |

## API Reference

### Session Operations

```rust
// Login
let session = Session::login(&pds, credentials).await?;

// Refresh tokens
session.refresh().await?;

// Access session info
session.did()      // Returns &Did
session.pds()      // Returns &PdsUrl
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

### Local Filesystem PDS

For development and testing, you can use a local filesystem-backed PDS:

```rust
use muat::backend::file::FilePdsBackend;
use muat::{Did, Nsid, RecordValue};
use serde_json::json;

// Create a backend
let backend = FilePdsBackend::new("/tmp/test-pds");

// Create a local account
let did = backend.create_account("alice.local")?;

// Create records
let collection = Nsid::new("org.test.record")?;
let value = RecordValue::new(json!({
    "$type": "org.test.record",
    "text": "test"
}))?;

let uri = backend.create_record(&did, &collection, &value, None).await?;
```

Directory structure:
```
$ROOT/pds/
├── accounts/<did>/account.json
├── collections/<collection>/<did>/<rkey>.json
└── firehose.jsonl
```

### Streaming

```rust
// Subscribe to repository events
session.subscribe_repos(|event| async move {
    match event {
        RepoEvent::Commit(commit) => println!("Commit: {}", commit.repo),
        RepoEvent::Identity(id) => println!("Identity: {}", id.did),
        RepoEvent::Handle(h) => println!("Handle: {} -> {}", h.did, h.handle),
        RepoEvent::Account(a) => println!("Account: {}", a.did),
        RepoEvent::Tombstone(t) => println!("Tombstone: {}", t.did),
    }
    true // Return false to stop subscription
}).await?;
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

1. **Session-first capability** - Authenticated operations require a `Session`; no free functions for authenticated endpoints
2. **Strong types at API boundaries** - Use `Nsid`, `AtUri`, `Did`, etc., not `String`
3. **Schema-agnostic record values** - The protocol layer does not interpret lexicon payloads
4. **Explicitness over magic** - No hidden global state, no silent retries, no implicit defaults
5. **Secrets are never logged** - Custom `Debug` implementations redact sensitive fields

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

- [Orbit workspace README](../../README.md)
- [Invariants documentation](docs/Invariants.md)
- [AT Protocol Specification](https://atproto.com/specs/atp)
