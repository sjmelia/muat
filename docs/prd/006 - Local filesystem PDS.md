# PRD-006: Local Filesystem PDS backend for `muat` via `file://`

> Note: This document reflects an earlier design. The current implementation uses `Pds`/`Session`/`RepoEventStream` with firehose on `Pds`. See `crates/muat/README.md` and `crates/muat/docs/Invariants.md` for current rules.


## Status

Done

## Motivation

We want to run the `atproto` CLI against a “fake” local PDS that stores records on the filesystem. This enables local-only development, testing, and verification of client behaviour without running a network PDS.

This PRD adds a `file://` PDS URL mode to `muat` and introduces `atproto pds …` commands to manage local accounts.

---

## Goals

1. Allow `muat` to accept a PDS URL of the form `file:///…`.
2. Implement a local filesystem-backed PDS store with:

   * record storage by collection + DID + rkey
   * an append-only firehose log (`firehose.jsonl`) updated on each record write
3. Add `atproto pds …` commands to manage accounts in the local store (create/remove/etc.).
4. Support multiple processes sharing the same data directory.
5. Ensure the firehose appends are cross-platform safe and minimally locked.
6. Add integration tests demonstrating correctness under concurrency.

---

## Non-Goals

* Full federation, repo MST, or CAR file formats
* Network transport emulation
* Cryptographic signing/verification beyond what is required for `muat`’s current client behaviour
* Lexicon schema validation beyond the invariants already mandated elsewhere (e.g. `RecordValue`)

---

## High-Level Design

### `muat` backend abstraction (Normative)

`muat` MUST introduce an explicit PDS backend abstraction so that the choice between network and local filesystem modes is a runtime selection behind a stable interface.

* There MUST be a single backend trait (e.g. `PdsBackend`) that models the minimum set of operations required by the `atproto` CLI.
* There MUST be at least two implementations:

  * `HttpPdsBackend` (existing behaviour)
  * `FilePdsBackend` (new local filesystem behaviour)
* The selection of backend implementation MUST be based solely on the configured PDS URL scheme:

  * `http://` / `https://` → `HttpPdsBackend`
  * `file://` → `FilePdsBackend`

The backend interface SHOULD be minimal and reflect required CLI functionality only (account management, record create/read/list, and any operations needed to support them).

---

## High-Level Design

### URL Selection

`muat` MUST interpret the PDS URL scheme as follows:

* `http://` / `https://`: existing network behaviour (unchanged)
* `file://`: local filesystem-backed PDS backend

The `file://` URL MUST resolve to a base directory (the “PDS root”).

---

## Filesystem Layout (Normative)

> **Note**: PRD-008 supersedes this section. The directory layout is now **repo-centric**.

Given a PDS root directory `$ROOT`, the local store MUST use the following structure:

* `$ROOT/pds/repos/<did>/collections/<collection>/<rkey>.json`
* `$ROOT/pds/firehose.jsonl`

Where:

* `<did>` is the DID string (repository owner)
* `<collection>` is the collection NSID string (as used in record URIs)
* `<rkey>` is the record key (file name without extension)

Records MUST be stored as UTF-8 JSON files.

### Record File Contents

The record file MUST contain the record value JSON (the “value” object), not an envelope.

Example file contents:

```
{
  "$type": "app.bsky.feed.post",
  "text": "hello"
}
```

(Envelope data such as uri/cid may be derived by higher layers if needed.)

---

## Firehose (Append-Only)

### File Location

The firehose MUST be located at:

* `$ROOT/pds/firehose.jsonl`

### Append Event Shape

Each time a record is added (created or updated in whatever sense `muat` supports), the backend MUST append exactly one newline-delimited JSON object to `firehose.jsonl`.

The event MUST include enough information for a consumer to:

* identify the record (at least `uri`)
* read the record value (either inline `value` or a pointer)

The exact schema is implementation-defined, but MUST be stable and documented in-code.

---

## Cross-Process Concurrency Requirements

Multiple processes MUST be able to:

* write records concurrently (different files)
* read all records concurrently
* append to the single firehose concurrently without corruption

### Firehose Locking (Normative)

Because firehose is a single shared append-only file, writers MUST:

1. Acquire an OS file lock prior to appending.

   * The lock MUST be cross-platform (Linux/macOS/Windows).
   * The lock target MAY be either:

     * `$ROOT/pds/firehose.lock` (preferred), or
     * the `firehose.jsonl` file itself
2. Lock scope MUST be limited to **“append one line”**.
3. While holding the lock, perform a single append operation:

   * Writes MUST be a single unbuffered write of the full line + `\n`.
   * Avoid stdio buffering that might split writes.
4. After writing the line:

   * MUST flush
   * MUST request durability using `fsync`/equivalent (`sync_data` or `sync_all`)
5. Release the lock.

### Implementation Notes (Non-Normative)

* Use `OpenOptions::new().create(true).append(true).open(...)`.
* Use `std::io::Write::write_all` on the raw file handle, not a buffered writer.
* Use `File::sync_data()` (or `sync_all()` if needed).
* Use a cross-platform file-lock crate (implementation choice), with an exclusive lock.

---

## `muat` Behaviour in Local Mode

### Record Writes

When `muat` is configured with a `file://` PDS URL, any operation that would create a record on a network PDS MUST instead:

1. Determine the record's target path:

   * `$ROOT/pds/repos/<did>/collections/<collection>/<rkey>.json`
2. Create directories as needed.
3. Write the record JSON file.

   * Writes SHOULD be atomic where feasible (write temp + rename).
4. Append one event line to the firehose under the firehose lock.

### Record Reads

Operations that fetch records or list records MUST read from the filesystem layout.

---

## New CLI Commands: `atproto pds …`

`atproto` MUST provide a `pds` command group for managing PDS-level resources.

When operating against a `file://` PDS URL, these commands manage the local filesystem-backed PDS.
When operating against a network (`http://` / `https://`) PDS URL, these commands MUST fail with a clear error indicating that the operation is not supported against a remote PDS.

### Required Commands

1. `atproto pds create-account`

   * General account creation command for the configured PDS.
   * When the PDS URL is `file://`, this command MUST create a new local account identity in the filesystem-backed PDS.
   * When the PDS URL is `http://` or `https://`, this command MUST fail with a clear, explicit error indicating that remote PDS account creation is not supported by this CLI.
   * This command is intentionally framed as a general “create account” operation; the local-only limitation is an implementation constraint, not a separate command category.
   * Must write any required account metadata for `muat` to function.

2. `atproto pds remove-account`

   * General account removal / disable command for the configured PDS.
   * When the PDS URL is `file://`, this command MUST remove or disable the local account.
   * When the PDS URL is `http://` or `https://`, this command MUST fail with a clear, explicit error indicating that remote PDS account removal is not supported by this CLI.
   * Behaviour MUST be safe and explicit (no silent deletes).

Additional commands may be added as needed for parity (e.g. `list-accounts`), but the above two are mandatory.

### Relationship to `muat`

* `atproto pds …` commands MUST be implemented in terms of `muat` functionality.
* `muat` MUST expose APIs to support these operations in local mode.

---

## Integration Tests

Integration tests MUST be added to cover the following.

### Local Store Basics

* Creating an account creates required metadata in the local store
* Removing an account removes/disables it as specified
* Creating a record writes the expected record file path
* The record file contains valid JSON and includes `$type`

### Firehose Append Correctness

* Creating a record appends exactly one line to `firehose.jsonl`
* The appended line is valid JSON and ends with `\n`

### Concurrency

* Two or more processes (or test threads using OS-level locks) appending concurrently MUST not corrupt `firehose.jsonl`.
* The firehose must contain whole lines (no interleaving of partial JSON).

Tests MUST assert:

* line count matches expected number of record writes
* every line parses as JSON

---

## Migration / Backwards Compatibility

* Existing behaviour for `http(s)://` PDS URLs MUST remain unchanged.
* `file://` support is additive.

---

## Success Criteria

This PRD is complete when:

* `muat` accepts `file://` PDS URLs and performs record IO against the filesystem layout
* `atproto pds create-account` and `atproto pds remove-account` exist and operate on the local store
* Firehose appends are cross-platform safe under concurrent writers
* Integration tests cover both correctness and concurrency requirements
