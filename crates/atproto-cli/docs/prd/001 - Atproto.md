# PRD-001: atproto — PDS Exploration CLI

## Purpose

`atproto` is a thin command-line wrapper over the `muat` library, intended for **manual protocol exploration** and **debugging** against a PDS.

This CLI exists to:
- verify assumptions about PDS behaviour,
- exercise authentication/session flows,
- inspect repo contents,
- observe repo commits via subscription APIs.

The CLI must remain deliberately small, stable, and boring.

---

## Goals

### G1 — Thin wrapper over `muat`
- The CLI must not re-implement protocol logic.
- All network/protocol actions must be delegated to `muat`.

### G2 — Single top-level surface: `pds`
The binary may have multiple subcommands, but currently **only under**:

```
atproto pds ...
```

No other top-level domains (eg `relay`, `appview`, `feed`) in this iteration.

### G3 — Session-based operation
- Commands requiring authentication must operate through a `muat::Session`.
- The CLI may persist session material for convenience (see Session Storage).

### G4 — Schema-agnostic
This iteration does **not** require typed lexicons.
- Listing and fetching records is supported.
- Record values are displayed as JSON.
- Creating records is optional and may be omitted until lexicons exist.

---

## Non-Goals

- No record schema/lexicon generation
- No record rendering beyond JSON
- No indexing / app-view behaviour
- No “chat app” semantics
- No multi-profile account management UI (beyond choosing a stored session)

---

## User Stories

1. **As a developer**, I can log in and see which DID/PDS I am using.
2. **As a developer**, I can list records in a collection and page through them.
3. **As a developer**, I can fetch a specific record to inspect its raw JSON.
4. **As a developer**, I can subscribe to repo events and see my writes appear.
5. **As a developer**, I can delete a record given its URI (cleanup of test data).

---

## Command Surface (V1)

### `atproto pds login`
Creates a new session.

**Inputs**
- `--identifier <handle-or-did>`
- `--password <app-password-or-password>`
- `--pds <base-url>` (optional; default: derived by identity resolution if possible)

**Outputs**
- Prints DID
- Prints PDS base URL
- Stores session (if enabled)

---

### `atproto pds whoami`
Displays the active session:
- DID
- PDS base URL
- token expiry (if known)

---

### `atproto pds list-records`
Lists records in a collection.

**Inputs**
- `--repo <did>` (optional; default: session DID)
- `--collection <nsid>` (required)
- `--limit <n>` (optional)
- `--cursor <cursor>` (optional)

**Outputs**
- Records printed as JSON lines or pretty JSON (flag controlled)
- Shows next cursor if present

---

### `atproto pds get-record`
Fetches a single record.

**Inputs**
- `--repo <did>` (optional; default: session DID)
- `--collection <nsid>` (required)
- `--rkey <rkey>` (required)

**Alternative form**
- `--uri <at-uri>` (optional; if provided, other inputs ignored)

**Outputs**
- Raw record JSON

---

### `atproto pds delete-record`
Deletes a record.

**Inputs**
- `--uri <at-uri>` (preferred)
- or: `--collection <nsid> --rkey <rkey> [--repo <did>]`

**Outputs**
- Prints deleted URI

---

### `atproto pds subscribe`
Subscribes to repo events (firehose-style).

**Inputs**
- `--cursor <cursor>` (optional)
- `--json` (optional; default: human-ish summaries)
- `--filter <prefix>` (optional; eg `dev.orbit.`)

**Outputs**
- Prints commit events
- Should surface `did`, `time`, and `ops` paths (eg `collection/rkey`)

---

## Session Storage

The CLI may persist session material for convenience.

### Requirements
- Store in XDG base dir (or platform equivalent)
- Permissions: user-readable only
- Multiple profiles supported by name (optional)

### Non-Requirements (for now)
- Encrypted secret storage integration
- Keychain / credential helpers

---

## Logging & Tracing

- `muat` emits `tracing` events; `atproto` configures the subscriber.
- CLI flags:
  - `-v/--verbose` increases log verbosity
  - `--json-logs` emits structured logs

---

## Definition of Done

- `atproto pds login` works against a Bluesky-hosted PDS using an app password
- `whoami`, `list-records`, `get-record`, `delete-record` work through `muat::Session`
- `subscribe` shows repo commit events in near real time
- No protocol logic duplicated outside `muat`
