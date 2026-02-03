# Implementation Plan: PRD-003 Hardening & Tests

## Overview

This plan details the implementation of PRD-003, which covers hardening the `muat` library and `atproto` CLI based on early end-to-end testing, plus adding comprehensive tests.

## Goals Summary

| Goal | Crate | Description |
|------|-------|-------------|
| G1 | muat | Fix session refresh bug |
| G2 | atproto-cli | Add `refresh-token` command |
| G3 | atproto-cli | Positional parameters for primary nouns |
| G4.1 | muat | Mock PDS unit tests |
| G4.2 | atproto-cli | Real PDS integration tests |

---

## G1: Fix Session Refresh Bug (muat)

### Current Issue

The `Session::refresh()` method calls `procedure_authed` with an empty unit `&()` as the body, which serializes to JSON `null`. The AT Protocol `refreshSession` endpoint expects either:
- An empty request body, or
- An empty JSON object `{}`

### Solution

1. Create an empty struct for the refresh request body
2. Ensure it serializes to `{}`
3. Improve error handling for refresh failures

### Files to Modify

- `crates/muat/src/xrpc/endpoints.rs` - Add `RefreshSessionRequest` type
- `crates/muat/src/auth/session.rs` - Update `refresh()` implementation

### Implementation

```rust
// In endpoints.rs
#[derive(Debug, Serialize)]
pub struct RefreshSessionRequest {}

// In session.rs - update refresh() to use the new type
let response: RefreshSessionResponse = self
    .inner
    .client
    .procedure_authed(REFRESH_SESSION, &RefreshSessionRequest {}, &refresh_token)
    .await?;
```

---

## G2: Add `refresh-token` Command (atproto-cli)

### New Command

```
atproto pds refresh-token
```

### Behavior

1. Load existing session from storage
2. Call `session.refresh()`
3. On success: save updated session, print confirmation to stderr
4. On failure: print error, exit non-zero

### Files to Create/Modify

- Create: `crates/atproto-cli/src/commands/pds/refresh_token.rs`
- Modify: `crates/atproto-cli/src/commands/pds/mod.rs` - Add command variant

---

## G3: Positional Parameters (atproto-cli)

### Changes Required

| Command | Current | New |
|---------|---------|-----|
| `list-records` | `--collection <nsid>` | `<collection>` (positional) |
| `get-record` | `--uri` or `--collection/--rkey` | `<uri>` (positional) |
| `delete-record` | `--uri` or `--collection/--rkey` | `<uri>` (positional) |

### Implementation Approach

Use clap's positional arguments with optional fallback to flags for backwards compatibility during transition.

### Files to Modify

- `crates/atproto-cli/src/commands/pds/list_records.rs`
- `crates/atproto-cli/src/commands/pds/get_record.rs`
- `crates/atproto-cli/src/commands/pds/delete_record.rs`

---

## G4.1: muat Mock PDS Tests

### Test Structure

```
crates/muat/
├── src/
│   └── ...
└── tests/
    ├── mock_pds.rs      # Shared mock server setup
    ├── auth_tests.rs    # Login and refresh tests
    └── repo_tests.rs    # Repository operation tests
```

### Test Cases

1. **Authentication**
   - Successful login
   - Invalid credentials
   - Session refresh (success)
   - Session refresh (expired token)
   - Session refresh (invalid token)

2. **Repository Operations**
   - List records (success, empty, pagination)
   - Get record (success, not found)
   - Create record (success, validation error)
   - Delete record (success, not found)

3. **Error Handling**
   - Non-JSON error responses
   - Empty response bodies
   - Network errors (timeout, connection refused)

### Dependencies

Add to `crates/muat/Cargo.toml`:
```toml
[dev-dependencies]
wiremock = "0.6"
tokio-test = "0.4"
```

---

## G4.2: CLI Integration Tests (atproto-cli)

### Test Structure

```
crates/atproto-cli/
├── src/
│   └── ...
└── tests/
    └── integration/
        ├── mod.rs
        └── pds_tests.rs
```

### Environment Variables

- `ATPROTO_TEST_IDENTIFIER` - Test account handle/DID
- `ATPROTO_TEST_PASSWORD` - Test account app password

### Test Collection Namespace

All tests use: `org.muat.test.record`

### Test Cases

1. Login with test credentials
2. Run `refresh-token` command
3. Create test record in `org.muat.test.record`
4. List records, verify test record appears
5. Get the specific record
6. Delete the test record
7. Verify record no longer appears

### Test Utilities

- `run_cli()` - Execute CLI binary and capture output
- `cleanup_test_records()` - Delete any leftover test records
- Skip logic when env vars not set

---

## Implementation Order

1. **G1** - Fix refresh bug (enables everything else)
2. **G2** - Add refresh-token command (simple, needed for G4.2)
3. **G3** - Positional parameters (improves G4.2 test ergonomics)
4. **G4.1** - Mock tests (can run without credentials)
5. **G4.2** - Integration tests (requires test account)

---

## Success Criteria

- [ ] `Session::refresh()` succeeds deterministically
- [ ] `atproto pds refresh-token` works correctly
- [ ] Positional parameters work for list-records, get-record, delete-record
- [ ] Mock PDS tests pass (`cargo test -p muat`)
- [ ] CLI integration tests pass when env vars set
- [ ] No regressions to existing behavior
