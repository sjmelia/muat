# Implementation Plan: muat Core Library

> Note: This document reflects an earlier design. The current implementation uses `Pds`/`Session`/`RepoEventStream` with firehose on `Pds`. See `crates/muat/README.md` and `crates/muat/docs/Invariants.md` for current rules.


## Overview

This plan details the implementation of `muat`, the core AT Protocol library. The library provides foundational protocol primitives with a session-centric API design.

## Architecture

```
crates/muat/
├── Cargo.toml
├── src/
│   ├── lib.rs              # Public API re-exports
│   ├── types/
│   │   ├── mod.rs
│   │   ├── did.rs          # Did newtype
│   │   ├── nsid.rs         # Nsid newtype
│   │   ├── at_uri.rs       # AtUri with structured accessors
│   │   ├── pds_url.rs      # PdsUrl newtype
│   │   └── rkey.rs         # Record key type
│   ├── auth/
│   │   ├── mod.rs
│   │   ├── credentials.rs  # Login inputs
│   │   ├── tokens.rs       # Access/Refresh tokens
│   │   └── session.rs      # Session capability object
│   ├── xrpc/
│   │   ├── mod.rs
│   │   ├── client.rs       # HTTP client wrapper
│   │   └── endpoints.rs    # XRPC endpoint definitions
│   ├── repo/
│   │   ├── mod.rs
│   │   ├── operations.rs   # list_records, get_record, etc.
│   │   └── streaming.rs    # subscribe_repos
│   └── error.rs            # Unified error type
└── docs/
    ├── prd/
    │   └── 001 - Core Library.md
    └── Invariants.md
```

## Implementation Phases

### Phase 1: Core Types

**Files:** `src/types/*.rs`

1. **Did** (`did.rs`)
   - Newtype wrapper: `struct Did(String)`
   - Validation: must start with `did:` and have valid method
   - Implements: `Display`, `FromStr`, `Serialize`, `Deserialize`, `Clone`, `Debug`, `PartialEq`, `Eq`, `Hash`

2. **Nsid** (`nsid.rs`)
   - Newtype wrapper: `struct Nsid(String)`
   - Validation: reverse-DNS format (e.g., `app.bsky.feed.post`)
   - Implements: same as Did

3. **AtUri** (`at_uri.rs`)
   - Struct with parsed components
   - Format: `at://<repo>/<collection>/<rkey>`
   - Accessors: `repo() -> &Did`, `collection() -> &Nsid`, `rkey() -> &str`
   - Implements: `Display`, `FromStr`, `Serialize`, `Deserialize`

4. **PdsUrl** (`pds_url.rs`)
   - Newtype wrapper over `url::Url`
   - Validation: must be absolute HTTPS URL
   - Normalization: trailing slash handling
   - Method: `xrpc_url(&self, method: &str) -> Url`

5. **Rkey** (`rkey.rs`)
   - Record key type (string for now, may add TID parsing later)
   - Newtype: `struct Rkey(String)`

### Phase 2: Authentication Types

**Files:** `src/auth/*.rs`

1. **Credentials** (`credentials.rs`)
   - Struct: `{ identifier: String, password: String }`
   - No `Debug` impl that shows password
   - Builder pattern for construction

2. **Tokens** (`tokens.rs`)
   - `AccessToken(String)` - opaque, no Debug
   - `RefreshToken(String)` - opaque, no Debug
   - Both implement `Clone` (for Session sharing)

3. **Session** (`session.rs`)
   - Core capability object
   - Fields: `did`, `pds`, `access_token`, `refresh_token`, `expires_at`
   - Inner struct wrapped in `Arc` for cheap cloning
   - Token refresh via interior mutability (`RwLock`)
   - Constructor: `Session::login(pds: &PdsUrl, credentials: Credentials) -> Result<Session>`
   - Method: `Session::refresh(&self) -> Result<()>`

### Phase 3: Error Types

**File:** `src/error.rs`

Single unified error enum:

```rust
pub enum Error {
    Transport(TransportError),      // Network, DNS, TLS, timeout
    Auth(AuthError),                // Invalid credentials, expired session
    Protocol(ProtocolError),        // XRPC errors, non-2xx responses
    InvalidInput(InvalidInputError), // Parse failures
}
```

Each variant has inner detail types. Never includes secrets in Display/Debug.

### Phase 4: XRPC Client

**Files:** `src/xrpc/*.rs`

1. **Client** (`client.rs`)
   - Wraps `reqwest::Client`
   - Methods for GET/POST XRPC calls
   - Handles authentication header injection
   - Response parsing with error mapping
   - Uses `tracing` for request/response logging (no secrets)

2. **Endpoints** (`endpoints.rs`)
   - Constants for XRPC method names
   - Request/response type definitions for:
     - `com.atproto.server.createSession`
     - `com.atproto.server.refreshSession`
     - `com.atproto.repo.listRecords`
     - `com.atproto.repo.getRecord`
     - `com.atproto.repo.createRecord`
     - `com.atproto.repo.deleteRecord`
     - `com.atproto.sync.subscribeRepos`

### Phase 5: Session Implementation

**File:** `src/auth/session.rs` (extended)

Session methods implemented against XRPC client:

```rust
impl Session {
    pub async fn login(pds: &PdsUrl, credentials: Credentials) -> Result<Self>;
    pub async fn refresh(&self) -> Result<()>;

    // Repo operations delegated from Session
    pub async fn list_records(&self, args: ListRecordsArgs) -> Result<ListRecordsOutput>;
    pub async fn get_record(&self, uri: &AtUri) -> Result<Record>;
    pub async fn create_record_raw(&self, collection: &Nsid, value: Value) -> Result<AtUri>;
    pub async fn delete_record(&self, uri: &AtUri) -> Result<()>;
    pub async fn subscribe_repos(&self, handler: impl RepoEventHandler) -> Result<()>;
}
```

### Phase 6: Repository Operations

**Files:** `src/repo/*.rs`

1. **Operations** (`operations.rs`)
   - `ListRecordsArgs`: repo (optional, defaults to session DID), collection, limit, cursor
   - `ListRecordsOutput`: records vec, cursor option
   - `Record`: uri, cid, value (serde_json::Value)
   - Implementation calls XRPC endpoints via Session's internal client

2. **Streaming** (`streaming.rs`)
   - WebSocket connection to `subscribeRepos`
   - Event types: Commit, Handle, Identity, etc.
   - Handler trait: `RepoEventHandler`
   - Reconnection logic (explicit, not hidden)

## Dependencies

```toml
[dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
reqwest = { version = "0.12", features = ["json", "rustls-tls"] }
tokio = { version = "1", features = ["full"] }
tokio-tungstenite = { version = "0.24", features = ["rustls-tls-webpki-roots"] }
url = { version = "2", features = ["serde"] }
thiserror = "2"
tracing = "0.1"
chrono = { version = "0.4", features = ["serde"] }
futures = "0.3"
parking_lot = "0.12"  # For RwLock in Session
```

## Testing Strategy

1. **Unit tests** for type validation (Did, Nsid, AtUri parsing)
2. **Integration tests** against public Bluesky PDS (with test account)
3. **Mock tests** for XRPC client using `wiremock`

## Success Criteria

- [ ] `Session::login()` successfully authenticates against Bluesky PDS
- [ ] `session.list_records()` returns records
- [ ] `session.get_record()` fetches specific records
- [ ] `session.subscribe_repos()` receives commit events
- [ ] All authenticated operations require `&Session`
- [ ] No secrets appear in logs or error messages
- [ ] All public API types are strongly typed (no raw Strings for Did/Nsid/etc.)
