# AGENTS.md - Context Bootstrap for AI Agents

This document provides essential context for AI coding agents working on the muat codebase.

## Project Summary

**muat** is a Rust toolkit for the AT Protocol (Bluesky's decentralized social network protocol). It provides:

1. **muat-core** - Core protocol types, errors, repo operations, tokens, and traits
2. **muat-xrpc** - XRPC-backed implementation (real PDS over HTTPS)
3. **muat-file** - Local filesystem PDS backend for offline development/testing
4. **atproto-cli** - CLI tool for manual PDS exploration

Key capabilities:

- Network PDS support (HTTPS)
- Local filesystem PDS support (`file://` URLs) for offline development
- Version reporting with git commit SHA

## Quick Context

```text
Language: Rust (2024 edition)
Build: Cargo workspace
Protocol: AT Protocol over XRPC (HTTPS)
Key pattern: Trait-based PDS/Session API with first-class tokens
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
- Tokens are opaque, first-class values (do not parse them)

### Schema Agnosticism

- Record values use `RecordValue` (guarantees `$type` field, wraps `serde_json::Value`)
- No lexicon-specific types in `muat-core`
- Protocol layer does not interpret record contents beyond `$type` presence

### RecordValue Invariants

- `RecordValue` MUST be a JSON object
- `RecordValue` MUST contain a `$type` field
- `$type` MUST be a string
- These are enforced at deserialization time

## File Locations

| What                  | Where                                                            |
| --------------------- | ---------------------------------------------------------------- |
| Core types            | `crates/muat-core/src/types/`                                    |
| Core traits           | `crates/muat-core/src/traits/`                                   |
| Repo operations       | `crates/muat-core/src/repo/`                                     |
| RecordValue           | `crates/muat-core/src/repo/record_value.rs`                      |
| Error types           | `crates/muat-core/src/error.rs`                                  |
| XRPC client           | `crates/muat-xrpc/src/xrpc/`                                     |
| XRPC PDS/session      | `crates/muat-xrpc/src/pds.rs`, `crates/muat-xrpc/src/session.rs` |
| File backend          | `crates/muat-file/src/`                                          |
| CLI commands          | `crates/atproto-cli/src/commands/pds/`                           |
| Session storage       | `crates/atproto-cli/src/session/`                                |
| CLI build script      | `crates/atproto-cli/build.rs`                                    |
| PRDs                  | `docs/prd/`                                                      |
| Implementation plans  | `docs/plans/`                                                    |
| Invariants doc        | `crates/muat-core/docs/Invariants.md`                            |
| Mock PDS tests        | `crates/muat-xrpc/tests/mock_pds.rs`                             |
| CLI integration tests | `crates/atproto-cli/tests/integration.rs`                        |
| CI workflows          | `.github/workflows/`                                             |

## Common Tasks

### Adding a new XRPC endpoint

1. Add endpoint constant to `crates/muat-xrpc/src/xrpc/endpoints.rs`
2. Define request/response types in the same file
3. Implement the method on `XrpcSession` in `crates/muat-xrpc/src/session.rs`
4. Ensure the method uses `query_authed` or `procedure_authed`

### Adding a new CLI command

1. Create new file in `crates/atproto-cli/src/commands/pds/`
2. Define `Args` struct with clap derives
3. Implement `run(args)` async function
4. Add to `PdsSubcommand` enum in `mod.rs`
5. Add match arm to `handle()` function

### Adding a new core type

1. Create file in `crates/muat-core/src/types/`
2. Implement: `new()`, `FromStr`, `Display`, `Serialize`, `Deserialize`
3. Add validation in `new()` that returns `Error::InvalidInput`
4. Export from `crates/muat-core/src/types/mod.rs`
5. Re-export from `crates/muat-core/src/lib.rs`

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
cargo check -p muat-core
cargo check -p muat-xrpc
cargo check -p muat-file
cargo check -p atproto-cli

# Run mock PDS tests (no external dependencies)
cargo test -p muat-xrpc --test mock_pds

# Run CLI integration tests (requires credentials)
export ATPROTO_TEST_IDENTIFIER="your.handle"
export ATPROTO_TEST_PASSWORD="your-app-password"
cargo test -p atproto-cli --test integration
```

### Test Organization

| Test Type       | Location                                  | Dependencies         |
| --------------- | ----------------------------------------- | -------------------- |
| Unit tests      | Inline in source files                    | None                 |
| Mock PDS tests  | `crates/muat-xrpc/tests/mock_pds.rs`      | `wiremock`           |
| CLI integration | `crates/atproto-cli/tests/integration.rs` | Real PDS credentials |

Integration tests are skipped automatically if `ATPROTO_TEST_IDENTIFIER` is not set.

## Dependencies

Key dependencies and their purposes:

| Dependency             | Purpose                                  |
| ---------------------- | ---------------------------------------- |
| `reqwest`              | HTTP client for XRPC                     |
| `tokio-tungstenite`    | WebSocket for subscriptions              |
| `serde` / `serde_json` | Serialization                            |
| `clap`                 | CLI argument parsing                     |
| `tracing`              | Structured logging                       |
| `thiserror`            | Error type derivation                    |
| `async-trait`          | Async traits for PDS/session             |
| `fs2`                  | Cross-platform file locking for firehose |
| `uuid`                 | DID generation for local accounts        |
| `bcrypt`               | Password hashing for file backend        |

## Code Style

- No emojis in code or comments
- Explicit error handling (no `.unwrap()` in library code)
- Use `tracing` macros, not `println!` for diagnostics
- Prefer composition over inheritance
- Keep functions focused and small

## What NOT to Do

- Don't add Bluesky-specific types to `muat-core`
- Don't implement retry logic in `muat-xrpc` (caller's responsibility)
- Don't log tokens or passwords (even at trace level)
- Don't use global state
- Don't add lexicon bindings to core library
- Don't duplicate protocol logic in CLI

## Architecture Decisions

1. **Why no typed lexicons?** - This iteration focuses on protocol primitives. Typed lexicons will be layered on top later.

2. **Why trait-based PDS/Session?** - Keeps auth behavior explicit and supports multiple backends without enums.

3. **Why schema-agnostic records?** - Allows `muat-core` to work with any AT Protocol application, not just Bluesky.

4. **Why thin CLI?** - CLI is for debugging/exploration. Complex UIs should build on `muat-*` crates directly.

## Getting Help

- PRDs: `docs/prd/`
- Invariants: `crates/muat-core/docs/Invariants.md`
- Implementation plans: `docs/plans/`
- muat-core README: `crates/muat-core/README.md`
- muat-xrpc README: `crates/muat-xrpc/README.md`
- muat-file README: `crates/muat-file/README.md`
- CLI README: `crates/atproto-cli/README.md`
