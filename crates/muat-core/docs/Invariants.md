# muat-core — Core Types & Invariants

## Purpose

This document defines the **normative core types** and **invariants** for the `muat-core` protocol library.

These rules are intended to:

- eliminate stringly-typed protocol boundaries,
- make invalid states unrepresentable where practical,
- keep the protocol layer schema-agnostic while still strongly typed.

Any code or downstream crate using `muat-core` must respect these invariants.

---

## Design Principles

1. **Session-scoped authentication**
   - Authenticated operations require a `Session` value.
   - No free functions that accept raw tokens.

2. **Strong types at API boundaries**
   - Use `Nsid`, `AtUri`, `Did`, etc., not `String`.

3. **Schema-agnostic record values**
   - Untyped record values use `serde_json::Value`.
   - The protocol layer does not interpret lexicon payloads.

4. **Explicitness over magic**
   - No hidden global state.
   - No silent retries.
   - No implicit environment-dependent defaults.

5. **Opaque, first-class tokens**
   - Tokens are opaque values that can be persisted and restored.
   - Core does not parse or interpret token contents.

---

## Core Types

### `Did`

Represents a decentralized identifier.

**Invariant**

- Always a syntactically valid DID string (eg `did:plc:...`, `did:web:...`).

---

### `Nsid`

Represents an AT Protocol NSID.

**Invariant**

- Always a syntactically valid NSID (reverse-DNS style).
- Validation occurs at construction (`FromStr`/`try_from`), never at call sites.

---

### `AtUri`

Represents an `at://` URI.

**Invariant**

- Always parseable and valid: `at://<repo>/<collection>/<rkey>`
- Provides structured accessors for repo, collection, rkey.

---

### `PdsUrl`

Represents the base URL of a PDS.

**Invariant**

- Absolute URL with scheme (`https://...` or `file://...`).
- Normalized such that joining `/xrpc/...` is well-defined.

---

### `Credentials`

Represents login inputs.

**Fields**

- `identifier`: handle or DID
- `secret`: password/app-password token

**Invariant**

- The library must not log secrets.

---

### `AccessToken` / `RefreshToken`

Bearer tokens (JWTs or opaque strings).

**Invariant**

- Treated as opaque.
- Never logged.
- Exposed for persistence and resumable sessions.

---

### `RecordValue`

A validated record payload.

**Invariants**

- Must be a JSON object.
- Must contain a `$type` field.
- `$type` must be a string.

These invariants are enforced at construction and deserialization.

---

## Traits

### `Pds`

The PDS trait represents a concrete PDS implementation (file-backed or XRPC-backed).

**Invariants**

- `login()` returns a `Session`.
- `create_account()` and `delete_account()` are PDS-scoped operations.
- `firehose()` returns a `Firehose` stream.
- Backend selection (file vs network) happens **outside** `muat-core`.

---

### `Session`

The session trait represents authenticated repository access.

**Invariants**

- All authenticated repo operations are methods on `Session`.
- `access_token()` and `refresh_token()` expose opaque tokens for persistence.
- No automatic token refresh in core.

---

### `Firehose`

The firehose trait is a stream of repository events.

**Invariants**

- `Firehose` implements `Stream<Item = Result<RepoEvent, Error>>`.
- Works uniformly across backends.

---

## Error Model

`muat-core` exposes a single public error type with variants including:

- Transport (network, DNS, TLS, timeout)
- Auth (invalid credentials, expired session)
- Protocol (non-2xx responses, XRPC error envelopes)
- InvalidInput (NSID/URI parse failures)

**Invariant**

- Errors include structured detail without leaking secrets.

---

## Logging & Tracing

- `muat-core` does not initialize a subscriber.
- Sensitive material (tokens, passwords) must never be logged.
