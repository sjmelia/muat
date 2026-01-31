# AGENTS.md - Context Bootstrap for AI Agents

This document provides essential context for AI coding agents working on the Orbit codebase.

## Project Summary

**Orbit** is a Rust toolkit for the AT Protocol (Bluesky's decentralized social network protocol). It provides:

1. **muat** - Core protocol library implementing XRPC, authentication, repository operations, and local filesystem PDS backend
2. **atproto-cli** - CLI tool for manual PDS exploration

Key capabilities:
- Network PDS support (HTTPS)
- Local filesystem PDS support (`file://` URLs) for offline development
- Version reporting with git commit SHA

## Quick Context

```
Language: Rust (2024 edition)
Build: Cargo workspace
Protocol: AT Protocol over XRPC (HTTPS)
Key pattern: Session-centric API
```

## Critical Invariants

Before modifying code, understand these non-negotiable invariants:

### Session-First Capability
- ALL authenticated operations MUST go through `Session`
- No free functions for authenticated endpoints
- `Session` is the only way to make authenticated requests

### Strong Typing at Boundaries
- Use `Did`, `Nsid`, `AtUri`, `PdsUrl` - NOT `String`
- Validation happens at construction, not at call sites
- Parse once, use everywhere

### Security Requirements
- Tokens and passwords MUST NEVER appear in logs
- `Debug` implementations MUST NOT expose secrets
- Session files MUST have restricted permissions (0600)

### Schema Agnosticism
- Record values use `RecordValue` (guarantees `$type` field, wraps `serde_json::Value`)
- No lexicon-specific types in `muat`
- Protocol layer does not interpret record contents beyond `$type` presence

### RecordValue Invariants
- `RecordValue` MUST be a JSON object
- `RecordValue` MUST contain a `$type` field
- `$type` MUST be a string
- These are enforced at deserialization time

## File Locations

| What | Where |
|------|-------|
| Core types | `crates/muat/src/types/` |
| Session/Auth | `crates/muat/src/auth/` |
| XRPC client | `crates/muat/src/xrpc/` |
| Repo operations | `crates/muat/src/repo/` |
| RecordValue | `crates/muat/src/repo/record_value.rs` |
| PDS backends | `crates/muat/src/backend/` |
| File backend | `crates/muat/src/backend/file.rs` |
| Error types | `crates/muat/src/error.rs` |
| CLI commands | `crates/atproto-cli/src/commands/pds/` |
| Session storage | `crates/atproto-cli/src/session/` |
| CLI build script | `crates/atproto-cli/build.rs` |
| PRDs | `docs/prd/` |
| Implementation plans | `docs/plans/` |
| Invariants doc | `crates/muat/docs/Invariants.md` |
| Mock PDS tests | `crates/muat/tests/mock_pds.rs` |
| CLI integration tests | `crates/atproto-cli/tests/integration.rs` |
| CI workflows | `.github/workflows/` |

## Common Tasks

### Adding a new XRPC endpoint to muat

1. Add endpoint constant to `crates/muat/src/xrpc/endpoints.rs`
2. Define request/response types in the same file
3. Implement the method on `Session` in `crates/muat/src/auth/session.rs`
4. Ensure the method uses `query_authed` or `procedure_authed`

### Adding a new CLI command

1. Create new file in `crates/atproto-cli/src/commands/pds/`
2. Define `Args` struct with clap derives
3. Implement `run(args)` async function
4. Add to `PdsSubcommand` enum in `mod.rs`
5. Add match arm to `handle()` function

### Adding a new core type

1. Create file in `crates/muat/src/types/`
2. Implement: `new()`, `FromStr`, `Display`, `Serialize`, `Deserialize`
3. Add validation in `new()` that returns `Error::InvalidInput`
4. Export from `crates/muat/src/types/mod.rs`
5. Re-export from `crates/muat/src/lib.rs`

## Error Handling

```rust
// Good - specific error variant
Err(InvalidInputError::Did { value, reason }.into())

// Bad - generic error
Err(Error::Other("bad DID"))
```

## Testing

```bash
# Check compilation
cargo check --workspace

# Run all tests
cargo test --workspace

# Check specific crate
cargo check -p muat
cargo check -p atproto-cli

# Run mock PDS tests (no external dependencies)
cargo test -p muat --test mock_pds

# Run CLI integration tests (requires credentials)
export ATPROTO_TEST_IDENTIFIER="your.handle"
export ATPROTO_TEST_PASSWORD="your-app-password"
cargo test -p atproto-cli --test integration
```

### Test Organization

| Test Type | Location | Dependencies |
|-----------|----------|--------------|
| Unit tests | Inline in source files | None |
| Mock PDS tests | `crates/muat/tests/mock_pds.rs` | `wiremock` |
| CLI integration | `crates/atproto-cli/tests/integration.rs` | Real PDS credentials |

Integration tests are skipped automatically if `ATPROTO_TEST_IDENTIFIER` is not set.

## Dependencies

Key dependencies and their purposes:

| Dependency | Purpose |
|------------|---------|
| `reqwest` | HTTP client for XRPC |
| `tokio-tungstenite` | WebSocket for subscriptions |
| `serde` / `serde_json` | Serialization |
| `clap` | CLI argument parsing |
| `tracing` | Structured logging |
| `thiserror` | Error type derivation |
| `async-trait` | Async traits for PDS backend |
| `fs2` | Cross-platform file locking for firehose |
| `uuid` | DID generation for local accounts |

## Code Style

- No emojis in code or comments
- Explicit error handling (no `.unwrap()` in library code)
- Use `tracing` macros, not `println!` for diagnostics
- Prefer composition over inheritance
- Keep functions focused and small

## What NOT to Do

- Don't add Bluesky-specific types to `muat`
- Don't implement retry logic in `muat` (caller's responsibility)
- Don't log tokens or passwords (even at trace level)
- Don't use global state
- Don't add lexicon bindings to core library
- Don't duplicate protocol logic in CLI

## Architecture Decisions

1. **Why no typed lexicons?** - This iteration focuses on protocol primitives. Typed lexicons will be layered on top later.

2. **Why session-centric?** - Makes authentication state explicit and prevents token leakage through function parameters.

3. **Why schema-agnostic records?** - Allows `muat` to work with any AT Protocol application, not just Bluesky.

4. **Why thin CLI?** - CLI is for debugging/exploration. Complex UIs should build on `muat` directly.

## Getting Help

- PRDs: `docs/prd/`
- Invariants: `crates/muat/docs/Invariants.md`
- Implementation plans: `docs/plans/`
- muat README: `crates/muat/README.md`
- CLI README: `crates/atproto-cli/README.md`
