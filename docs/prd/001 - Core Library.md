# PRD-001: muat — Core AT Protocol Library

> Note: This document reflects an earlier design. The current implementation uses `Pds`/`Session`/`Firehose` with firehose on `Pds`. See `crates/muat-core/README.md` and `crates/muat-core/docs/Invariants.md`  for current rules. File paths in this document refer to the pre-split `crates/muat` layout.


Status: done

## Purpose

`muat` is a foundational Rust library implementing **AT Protocol fundamentals** directly, without dependence on Bluesky-specific application semantics or third-party client abstractions.

This library exists to provide a **stable, strongly-typed protocol core** that higher-level systems (CLI, TUI, GUI, agents, app-views) can build upon.

A primary requirement is that authentication produces a **session object** which becomes the sole capability for interacting with a PDS.

---

## Motivation

We cannot rely exclusively on existing AT / Bluesky client libraries because:

* We will later need to **experiment with account creation, authentication, and session semantics**
* We want **explicit control over protocol boundaries and invariants**
* We need a protocol layer suitable for **non-social, non-feed-based applications**
* We want to treat AT as a *general data substrate*, not a microblog API

---

## Goals

### G1 — First‑class AT protocol implementation

Implement AT protocol primitives directly over XRPC:

* Authentication & session management
* Repository inspection
* Repository mutation (API-level only)
* Repo sync & subscription primitives

The library must not assume any Bluesky-specific schemas or app behaviour.

---

### G2 — Session‑centric API

All authenticated protocol actions must be accessed via a **`Session` value**.

* Logging in returns a `Session`
* The `Session` encapsulates all authentication state
* All repo operations are methods on `Session`

This pattern is mandatory.

Illustrative example:

```rust
let session = Session::login(credentials).await?;

session.list_records(collection).await?;
session.get_record(uri).await?;
session.subscribe_repos(handler).await?;
```

---

### G3 — Schema‑agnostic core

This iteration of `muat`:

* Does not define lexicon bindings
* Does not interpret record contents
* Treats record values as opaque JSON

Typed lexicons will be layered later.

---

## Non‑Goals

* App View implementation
* Feed or social graph semantics
* Record rendering or indexing
* Local PDS implementation
* Multi-session orchestration

---

## Crate Layout

```
crates/
  muat/          # core protocol library
```

---

## Core Responsibilities

The `muat` crate is responsible for:

* XRPC transport
* Authentication flows
* Session lifecycle
* Repo-level read/write operations
* Streaming subscriptions

It is explicitly **not** responsible for:

* Persistence
* Caching
* Schema validation beyond protocol constraints
* Application-specific behaviour

---

## Public API Overview

### Authentication

* `Session::login(credentials) -> Session`
* `Session::refresh() -> Session`

---

### Repository Inspection

* `session.list_records(collection, cursor)`
* `session.get_record(uri)`

---

### Repository Mutation (Raw)

* `session.create_record_raw(collection, value)`
* `session.delete_record(uri)`

Raw JSON (`serde_json::Value`) is used intentionally.

---

### Repo Sync / Streaming

* `session.subscribe_repos(handler)`

Provides access to repo commit events.

---

## Error Model

* Single error enum for the crate
* Explicit variants for:

  * transport errors
  * authentication errors
  * protocol errors
  * invalid inputs

No silent retries.

---

## Logging & Tracing

* Uses `tracing` for structured logs
* No global subscriber configuration
* Consumers (CLI, apps) configure logging

---

## Definition of Done

* Successful login to a Bluesky-hosted PDS
* Ability to list and fetch records
* Ability to subscribe to repo events
* No dependency on Bluesky app schemas
* All authenticated actions flow through `Session`

---

## Out of Scope

* Typed lexicon bindings
* Alternative auth mechanisms
* Account creation flows
* Offline repo tooling
* App View support
