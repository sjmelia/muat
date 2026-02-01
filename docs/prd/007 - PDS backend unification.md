# PRD-007: PDS Backend Unification (Session + XRPC)

## Status

Ready

## Motivation

`PdsBackend` exists and is implemented by `FilePdsBackend`, but network operations still go through `Session` directly and the XRPC client does not implement the backend trait. This creates two parallel paths for the same operations (create/read/list/delete records) and makes it easy for callers to bypass the backend abstraction.

We want a single backend interface used by all record operations after the PDS URL has been validated. To enable a full API surface behind the backend, the authentication invariant is loosened so that the backend can accept a token parameter when required.

This PRD takes precedence over any other invariants, including those in `/crates/muat/docs/Invariants.md`.

---

## Goals

1. Implement `PdsBackend` for network-backed operations using XRPC.
2. Ensure code paths that operate on records (create/get/list/delete) use `PdsBackend` after PDS URL validation.
3. Allow backend methods to accept auth tokens when needed for network operations.
4. Keep local filesystem mode (`file://`) working and aligned with the same backend interface.
5. Minimize public API churn for existing `Session` and CLI users.

---

## Non-Goals

* Changing the authentication model (sessions remain the only source of auth).
* Introducing lexicon-specific record types.
* Adding new PDS endpoints beyond the existing record operations.
* Adding retry or caching logic in the backend layer.

---

## Proposed Design

### 1) Clarify Backend Roles

`PdsBackend` is the unified interface for record operations:

* create_record
* get_record
* list_records
* delete_record

A new **network-backed implementation** will be introduced:

**`XrpcPdsBackend`**

* Implements `PdsBackend` directly and uses the XRPC client internally.
* Backend methods accept a token parameter where required for authenticated endpoints.
* `Session` owns tokens and calls backend methods with the appropriate token.

`PdsBackend` will also include account management methods:

* `create_account` (no token required)
* `delete_account` (requires a token for the account being deleted)

### 2) Session Integration

`Session` will use the backend internally for record operations:

* `Session` holds a backend instance selected from the PDS URL scheme.
* Record methods on `Session` become thin delegations to the backend, passing auth tokens when required.
* `Session` remains the public entry point for authenticated operations, but the backend now carries the full API surface.

### 3) PDS URL Selection

Backend selection should remain scheme-based:

* `file://` → `FilePdsBackend`
* `http://` / `https://` → `Session`-derived backend

Any API that accepts a `PdsUrl` should validate and then construct the appropriate backend without exposing implementation details.

### 4) Public API Surface (Minimal Changes)

Minimal new public items:

* `backend::create_backend(...)` or similar helper that chooses `FilePdsBackend` vs `XrpcPdsBackend` based on the PDS URL scheme.

Existing `Session::create_record`, `Session::get_record`, etc. remain (they delegate internally to the backend and supply tokens as needed).

### 5) Backend Representation in `Session`

Prefer a concrete enum to store the backend inside `Session`, since the set of backends is closed and known:

* avoids dynamic dispatch
* keeps types explicit and pattern-matchable
* aligns with common Rust practice for small, fixed variants

### 5) Documentation Updates

* Update `crates/muat/README.md` to show backend usage for both file and network cases.
* Update module docs in `crates/muat/src/backend/mod.rs` to reflect that network operations now implement the trait.
* Update `/crates/muat/docs/Invariants.md` with new invariants.

---

## Error Handling

* Preserve existing error types and semantics.
* `PdsBackend` methods continue returning `Result<_, Error>`.
* Network backend should map XRPC errors to `Error::Protocol` consistently with current `Session` behavior.

---

## Testing

1. **Unit tests** for the new network backend wrapper (delegation correctness, no token leaks in debug).
2. **Existing mock PDS tests** continue to validate local backend behavior unchanged.
3. **CLI integration tests** are unchanged but should pass using the unified backend path.

---

## Migration Notes

* Internal code should favor `PdsBackend` once the PDS URL is validated.
* Public `Session` methods remain supported; no breaking changes required.
* This PRD intentionally does not remove any APIs; it consolidates internal paths.

---

## Open Questions

1. Should delete-account tokens be access tokens only, or should refresh tokens be accepted too?
  - delete account tokens should allow refresh tokens.
