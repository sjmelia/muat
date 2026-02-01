# muat — Core Types & Invariants

## Purpose

This document defines the **normative core types** and **invariants** for the `muat` protocol library.

These rules are intended to:
- eliminate stringly-typed protocol boundaries,
- make invalid states unrepresentable where practical,
- keep the protocol layer schema-agnostic while still strongly typed.

Any code or downstream crate using `muat` must respect these invariants.

---

## Design Principles

1. **Session-first capability**
   - Authenticated operations require a `Session`.
   - No free functions for authenticated endpoints.

2. **Strong types at API boundaries**
   - Use `Nsid`, `AtUri`, `Did`, etc., not `String`.

3. **Schema-agnostic record values**
   - Untyped record values use `serde_json::Value`.
   - The protocol layer does not interpret lexicon payloads.

4. **Explicitness over magic**
   - No hidden global state.
   - No silent retries.
   - No implicit environment-dependent defaults.

---

## Core Types

### `Did`
Represents a decentralized identifier.

**Invariant**
- Always a syntactically valid DID string (eg `did:plc:...`, `did:web:...`).

**Notes**
- Keep as a newtype (`struct Did(String);`) with validating constructor.
- Prefer `Display` and `FromStr` implementations.

---

### `Nsid`
Represents an AT Protocol NSID.

**Invariant**
- Always a syntactically valid NSID (reverse-DNS style).
- Validation occurs at construction (`FromStr`/`try_from`), never at call sites.

**Usage**
- Collections are identified by `Nsid`.

---

### `AtUri`
Represents an `at://` URI.

**Invariant**
- Always parseable and valid:
  - `at://<repo>/<collection>/<rkey>`
- Provides structured accessors:
  - `repo(): Did`
  - `collection(): Nsid`
  - `rkey(): Rkey` (or `String` until typed)

---

### `PdsUrl`
Represents the base URL of a PDS (XRPC server).

**Invariant**
- Absolute URL with scheme (`https://...`).
- Normalized such that joining `/xrpc/...` is well-defined.

---

### `Credentials`
Represents login inputs.

**Fields**
- `identifier`: handle or DID (keep as a string/newtype; parsing may be deferred)
- `secret`: password/app-password token

**Invariant**
- The library must not log secrets.
- The CLI must avoid printing secrets on failure.

---

### `AccessToken` / `RefreshToken`
Bearer tokens (JWTs or opaque strings).

**Invariant**
- Treated as opaque.
- Never logged.
- Carried only inside `Session` unless explicitly exported for persistence.

---

### `Session`
The central capability object for authenticated operations.

**Holds**
- `did: Did`
- `pds: PdsUrl`
- `access_token: AccessToken`
- `refresh_token: Option<RefreshToken>`
- `expires_at: Option<DateTime<Utc>>` (if known)

**Invariants**
1. A `Session` always refers to exactly one DID.
2. A `Session` always targets exactly one PDS.
3. All authenticated endpoint calls require a `&Session`.
4. Session construction is only via:
   - `Session::login(...)`
   - `Session::from_persisted(...)` (if implemented)
   - `Session::refresh(...)` (returns a new/updated session)

**Concurrency**
- `Session` must be cheap to clone or share (eg `Arc<SessionInner>`), OR be explicitly non-clone with clear sharing guidance.
- Any internal mutability must be deliberate (eg for token refresh) and thread-safe if enabled.

---

## Record Value Representation

### RecordValue Type

For endpoints that return or accept record bodies, `muat` uses `RecordValue`:

```rust
pub struct RecordValue(serde_json::Value);
```

**Invariants**
- `RecordValue` MUST be a JSON object (not array, string, null, etc.)
- `RecordValue` MUST contain a `$type` field
- The `$type` field MUST be a string (the record's lexicon NSID)
- These invariants are enforced at:
  - Construction time (`RecordValue::new()`)
  - Deserialization time (custom `Deserialize` impl)
- It is impossible to create an invalid `RecordValue`

**API**
- `RecordValue::new(value: Value) -> Result<Self>` - validate and wrap
- `RecordValue::with_type(record_type: &str, value: Value) -> Result<Self>` - set $type and wrap
- `record_type() -> &str` - access the $type field (infallible)
- `as_value() -> &Value` - access inner value
- `get(key: &str) -> Option<&Value>` - access fields

**Rationale**
- AT Protocol requires all records to have a `$type` field
- Enforcing at the type level makes invalid states unrepresentable
- Parsing into typed lexicon structs is done *outside* `muat` (later layer)

---

## Method Surface Invariants (Normative)

### Authenticated endpoints
All authenticated endpoints are methods on `Session`, including:

- `list_records(...)`
- `get_record(...)`
- `create_record_raw(...)`
- `delete_record(...)`
- `subscribe_repos(...)`

**Forbidden**
- `muat::repo::list_records(access_token: ..., ...)` (token plumbing outside session)

---

## Error Model

`muat` exposes a single public error type, with variants including:

- Transport (network, DNS, TLS, timeout)
- Auth (invalid credentials, expired session)
- Protocol (non-2xx responses, XRPC error envelopes)
- InvalidInput (NSID/URI parse failures)

**Invariant**
- Endpoint methods do not return ad-hoc errors.
- Errors include enough structured detail for debugging without leaking secrets.

---

## Logging & Tracing

- `muat` emits `tracing` events only.
- No subscriber initialization in `muat`.
- Sensitive material (tokens, passwords) must never be logged.

---

## Compatibility Notes

- `muat` should target the AT Protocol XRPC surface as defined by the specs.
- Bluesky-specific endpoints may exist elsewhere, but are out of scope for `muat` core.

---

## Emergent Invariants (Implementation)

The following invariants emerged during implementation and are now normative:

### `Rkey` (Record Key)

**Invariant**
- Valid characters: `a-z`, `A-Z`, `0-9`, `.`, `-`, `_`, `~`
- Length: 1-512 characters
- Cannot be `.` or `..`
- Typically a TID (timestamp identifier) but not validated as such

---

### Session Architecture

**Implementation**
- `Session` wraps `Arc<SessionInner>` for cheap cloning
- Token storage uses `RwLock` for thread-safe refresh
- Clone is shallow (reference counted)

**Token Export**
- `export_access_token()` and `export_refresh_token()` exist for persistence
- These are async due to internal `RwLock`
- Callers are responsible for secure storage

---

### PdsUrl Normalization

**Invariant**
- Trailing slashes are removed during construction
- HTTP allowed only for localhost/127.0.0.1/::1
- HTTPS required for all other hosts
- `file://` URLs are allowed for local filesystem PDS

**URL Schemes**
- `https://` - Network PDS (production)
- `http://` - Network PDS (localhost only)
- `file://` - Local filesystem PDS

**file:// Specific**
- `file://` URLs must have a path component
- `is_local()` returns true for `file://` URLs
- `to_file_path()` converts to `PathBuf` for file:// URLs

---

### XRPC Client Internal

**Architecture**
- `XrpcClient` is internal to `muat` (not public)
- All authenticated methods require token parameter
- Response parsing handles XRPC error envelope format

**Request Types**
- `query`: GET request with query parameters
- `procedure`: POST request with JSON body
- Auth headers use `Bearer` scheme

---

### Streaming/Subscription

**WebSocket URL Construction**
- `https://` prefix converts to `wss://`
- `http://` prefix converts to `ws://` (localhost only)
- Path is `/xrpc/com.atproto.sync.subscribeRepos`

**Event Types**
- All event types (`CommitEvent`, `IdentityEvent`, etc.) derive both `Serialize` and `Deserialize`
- This enables JSON output in CLI and future re-serialization needs

**Handler Pattern**
- Handler returns `bool` to continue/stop
- Returning `false` gracefully terminates the subscription

---

### Debug Implementation

**Invariant**
- Types containing secrets (`Credentials`, `AccessToken`, `RefreshToken`, `Session`) MUST have custom `Debug` impls
- Secret fields display as `[REDACTED]`
- This prevents accidental logging via `{:?}` formatting

---

### Serde Patterns

**Field Naming**
- XRPC uses `camelCase` (e.g., `accessJwt`, `refreshJwt`)
- Rust types use `snake_case`
- `#[serde(rename_all = "camelCase")]` bridges the gap

**Optional Fields**
- Use `#[serde(default)]` for optional response fields
- Use `#[serde(skip_serializing_if = "Option::is_none")]` for optional request fields

---

### XRPC Request Bodies

**Invariant**
- Some endpoints require no request body at all (not even `{}`)
- The `procedure_authed_no_body` method sends a POST with no body
- Regular endpoints use `procedure_authed` with a JSON body

**No-Body Endpoints**
- `com.atproto.server.refreshSession` - expects no body, rejects even `{}`

**Example**
```rust
// Correct: no body at all
client.procedure_authed_no_body(REFRESH_SESSION, token).await?;

// Wrong: sends {}, rejected by some PDS implementations
client.procedure_authed(REFRESH_SESSION, &EmptyStruct{}, token).await?;
```

---

### Token Refresh Protocol

**Invariant**
- Token refresh uses the refresh token in the `Authorization: Bearer` header
- The request body MUST be empty (no body, not even `{}`)
- The PDS returns new `accessJwt` and `refreshJwt` values
- Both tokens must be updated atomically

---

### URL Construction

**Invariant**
- XRPC URLs must not contain double slashes (e.g., `//xrpc/`)
- The `PdsUrl::xrpc_url()` method trims trailing slashes before constructing the URL
- The `url` crate normalizes URLs to include trailing slashes, so explicit trimming is required

**Example**
```rust
pub fn xrpc_url(&self, method: &str) -> String {
    let base = self.0.as_str().trim_end_matches('/');
    format!("{}/xrpc/{}", base, method)
}
```

---

### Filesystem PDS Backend

**Directory Structure**
```
$ROOT/pds/
├── accounts/<did>/account.json
├── collections/<collection>/<did>/<rkey>.json
└── firehose.jsonl
```

**Record Storage Invariants**
- Records are stored as UTF-8 JSON files
- File contains only the record value (not an envelope)
- Path: `$ROOT/pds/collections/<collection>/<did>/<rkey>.json`
- Parent directories are created as needed
- Writes use atomic temp file + rename pattern

**Firehose Invariants**
- Firehose is append-only: `$ROOT/pds/firehose.jsonl`
- Each line is a single JSON object with `uri`, `time`, `op` fields
- Lines end with `\n`
- Cross-process safety via exclusive file lock on `firehose.lock`
- Lock scope is "append one line" only
- Writes are followed by `fsync` for durability

**Account Management Invariants**
- Accounts have generated DIDs: `did:plc:<uuid-based>`
- Account metadata stored in `$ROOT/pds/accounts/<did>/account.json`
- Account removal can optionally delete associated records

---

### Version Reporting

**Invariant**
- `atproto --version` reports version derived from git state
- Tagged releases: `atproto 0.2.0`
- Development builds: `atproto 0.2.0-5-gabc1234` or `atproto abc1234`
- Version is captured at compile time via build.rs

---

### PDS Backend Unification (PRD-007)

**Overview**
`PdsBackend` is the unified interface for record operations. Both network (XRPC) and filesystem implementations use this trait.

**Backend Types**
- `FilePdsBackend`: Filesystem-backed storage for local development
- `XrpcPdsBackend`: Network-backed storage via XRPC protocol
- `BackendKind`: Concrete enum holding either backend type (avoids dynamic dispatch)

**Token Handling Invariants**
- Backend methods accept an optional `token` parameter for authenticated operations
- Network backends (`XrpcPdsBackend`) REQUIRE a token for authenticated operations
- Filesystem backends (`FilePdsBackend`) IGNORE the token parameter
- `Session` supplies tokens automatically when delegating to the backend

**Backend Selection Invariants**
- `create_backend(&PdsUrl)` selects the appropriate backend based on URL scheme
- `file://` URLs → `FilePdsBackend`
- `http://` and `https://` URLs → `XrpcPdsBackend`
- Selection is deterministic and based solely on the URL scheme

**Session Integration**
- `Session` holds a `BackendKind` internally
- Record operations on `Session` delegate to the backend with the access token
- `Session::backend()` exposes the underlying backend for advanced use
- `Session` remains the public entry point for authenticated operations

**Account Management**
- `PdsBackend::create_account()` creates accounts (no token required for local, varies for network)
- `PdsBackend::delete_account()` deletes accounts (requires token and password for network)
- Filesystem backend generates `did:plc:<uuid>` identifiers locally
- Network backend calls the XRPC `createAccount`/`deleteAccount` endpoints

**Concrete Backend Storage**
- Session uses a concrete enum (`BackendKind`) rather than `dyn PdsBackend`
- This avoids dynamic dispatch and keeps types explicit
- The set of backends is closed and known at compile time

---

## Definition of Done

- All public API boundaries use strong types (`Did`, `Nsid`, `AtUri`, `PdsUrl`, `RecordValue`, `Session`)
- Record payloads use `RecordValue` which guarantees `$type` field
- All authenticated operations are methods on `Session`
- Error type is unified and does not leak secrets
- `file://` URLs enable local development without network PDS
- `PdsBackend` trait provides unified interface for both file and network backends
- `Session` delegates record operations to the backend internally
