# muat — Core Types & Invariants

## Status
Draft (Normative)

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

### Untyped records
For endpoints that return record bodies, `muat` uses:

- `serde_json::Value`

**Invariant**
- Public APIs must not accept/return record payloads as `String` or raw bytes.
- Parsing into typed lexicon structs is done *outside* `muat` (later layer).

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

## Definition of Done

- All public API boundaries use strong types (`Did`, `Nsid`, `AtUri`, `PdsUrl`, `Session`)
- Untyped record payloads are `serde_json::Value`
- All authenticated operations are methods on `Session`
- Error type is unified and does not leak secrets
