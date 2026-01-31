# PRD-0002: atproto CLI & muat Hardening

## Status

Done

## Scope Note

This PRD spans two crates:

* `muat` (protocol library)
* `atproto` (CLI)

The scope is intentionally limited to **hardening existing behaviour** discovered during early end-to-end testing, rather than introducing new protocol features.

---

## Motivation

Initial validation against a real Bluesky-hosted PDS has succeeded, but surfaced several issues that should be addressed before further feature work:

* a bug in the session refresh path
* missing explicit control over token refresh
* CLI ergonomics issues (overuse of flags for primary parameters)
* lack of automated CLI tests to prevent regressions

This PRD addresses these issues together as a single hardening step.

---

## Goals

### G1 — Fix the session refresh bug (`muat`)

* Identify and fix the current refresh bug
* Ensure refresh failures do not prevent normal operation
* Improve error handling only insofar as required to make refresh reliable

No attempt is made in this PRD to redesign refresh strategy or lifecycle management.

---

### G2 — Explicit token refresh command (`atproto`)

Add a CLI command:

```
atproto pds refresh-token
```

This command:

* explicitly invokes session refresh
* reports success or failure clearly
* does not perform any other side effects

---

### G3 — Positional primary parameters (`atproto`)

Improve CLI ergonomics by making the **primary noun** of a command positional rather than a flag.

This applies to, at minimum:

```
atproto pds list-records <collection>
atproto pds get-record <collection> <rkey>
atproto pds get-record <at-uri>
atproto pds delete-record <at-uri>
```

Flags such as `--collection` may be removed or retained only as deprecated aliases.

---

### G4 — CLI integration tests against a real PDS

Add automated CLI tests that:

* authenticate using environment-provided credentials
* operate against a real Bluesky-hosted PDS
* create, list, and delete **non-Bluesky** test records

These tests validate:

* session handling
* CLI argument parsing
* end-to-end protocol correctness

---

## Non-Goals

* Redesigning session lifecycle management
* Adding new protocol endpoints
* Defining lexicon bindings
* Supporting multiple concurrent sessions

---

## Detailed Requirements

## G1 — Session Refresh Bug Fix (`muat`)

* Fix the observed refresh failure
* Ensure refresh errors are surfaced meaningfully
* Ensure secrets are never logged

Definition of done:

* `Session::refresh()` succeeds or fails deterministically
* No spurious JSON parse errors are emitted

---

## G2 — `refresh-token` CLI Command (`atproto`)

### Command Behaviour

```
atproto pds refresh-token
```

* Requires an existing persisted session
* Calls `Session::refresh()`
* On success:

  * updates persisted session state
  * prints a short confirmation to stderr
* On failure:

  * prints a clear error message
  * exits non-zero

---

## G3 — Positional Parameters (`atproto`)

* The collection for `list-records` MUST be positional
* `get-record` and `delete-record` MUST prefer `at://` URIs where possible
* Help text and examples must reflect positional usage

---

## G4 — Tests

Testing is split deliberately by layer to balance realism, speed, and determinism.

### Test Strategy Overview (Normative)

* **`muat` library tests** use a **mock PDS**
* **`atproto` CLI tests** use a **real PDS** (env-gated)
* No direct `muat` tests against a real PDS are required at this stage

This provides fast, deterministic coverage of protocol logic while preserving end-to-end validation of real-world behaviour via the CLI.

---

### G4.1 — `muat` Tests (Mock PDS)

`muat` tests must:

* run fully in-process
* use a mock or local HTTP server
* not require network access or real credentials

These tests focus on:

* request construction (paths, headers, bodies)
* response parsing
* error handling branches (including refresh failures)
* token refresh behaviour in isolation

Mock responses should include:

* successful JSON responses
* error envelopes
* non-JSON error bodies (plain text, empty body)

---

### G4.2 — `atproto` CLI Tests (Real PDS)

CLI integration tests:

* are opt-in
* are gated by environment variables
* run the compiled `atproto` binary
* exercise real end-to-end behaviour against a Bluesky-hosted PDS

#### Required Environment Variables

* `ATPROTO_TEST_IDENTIFIER`
* `ATPROTO_TEST_PASSWORD`

Tests must be skipped if these are not set.

---

#### Test Record Type

All CLI tests must use a **non-Bluesky namespace**, e.g.:

```
org.muat.test.record
```

Rationale:

* avoids polluting `app.bsky.*` collections
* avoids coupling to Bluesky app semantics
* clearly marks records as disposable test artefacts

---

#### Required CLI Test Cases

1. Login using test credentials
2. Run `atproto pds refresh-token`
3. Create a test record in `org.muat.test.record`
4. List records and verify the test record appears
5. Delete the test record
6. Verify the record no longer appears in `list-records`

All tests must attempt cleanup on failure.

---

## Output Contract (Reaffirmed)

* `tracing` → logs
* `stdout` → command results
* `stderr` → commentary

CLI tests must assert against **stdout only** unless explicitly testing commentary.

---

## Definition of Done (Overall)

* Refresh bug fixed
* `atproto pds refresh-token` implemented
* Positional parameters implemented
* CLI integration tests passing against a real PDS
* No regressions to existing behaviour
