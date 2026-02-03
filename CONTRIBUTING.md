# Contributing

Thanks for contributing to the muat workspace. This guide describes the shared rules, workflows, and technical constraints for all contributors.

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

## Critical Invariants

The canonical invariants live in `crates/muat-core/docs/Invariants.md`. Please read that document before making changes that affect authentication, tokens, record values, or protocol behavior.

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
| CLI integration tests | `crates/atproto-cli/tests/file_pds.rs`, `crates/atproto-cli/tests/bluesky_pds.rs` |
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

# Run unit tests (no integration tests)
cargo test --workspace --lib --bins

# Check specific crate
cargo check -p muat-core
cargo check -p muat-xrpc
cargo check -p muat-file
cargo check -p atproto-cli

# Run mock PDS tests (no external dependencies)
cargo test -p muat-xrpc --test mock_pds

# Run file-backed CLI integration tests
cargo test -p atproto-cli --test file_pds

# Run Bluesky PDS integration tests (requires credentials)
export ATPROTO_TEST_IDENTIFIER="your.handle"
export ATPROTO_TEST_PASSWORD="your-app-password"
cargo test -p atproto-cli --test bluesky_pds
```

### Test Organization

| Test Type       | Location                                  | Dependencies         |
| --------------- | ----------------------------------------- | -------------------- |
| Unit tests      | Inline in source files                    | None                 |
| Mock PDS tests  | `crates/muat-xrpc/tests/mock_pds.rs`      | `wiremock`           |
| CLI integration (file) | `crates/atproto-cli/tests/file_pds.rs` | None                 |
| CLI integration (Bluesky) | `crates/atproto-cli/tests/bluesky_pds.rs` | Real PDS credentials |

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

## CI and Branch Protection

CI workflows run on pull requests and pushes. GitHub branch protection rules must be configured in repository settings to require the CI workflow checks for merges into `main`. This cannot be enforced by workflow code alone.

If you add crates.io publishing to CI, store a `CARGO_REGISTRY_TOKEN` (or `CRATES_IO_TOKEN`) secret in GitHub Settings and reference it in the workflow environment.

Bluesky PDS e2e tests are intentionally optional and only run when secrets are present and the workflow is not a pull request from a fork.

## Release Process

1. Bump versions in the relevant `Cargo.toml` files.
2. Commit and merge to `main`.
3. Tag the `main` commit with `vX.Y.Z` and push the tag.
4. The `release.yml` workflow builds release binaries and creates a GitHub Release.

Version output is embedded at build time. `atproto --version` prints `{semver}+{short_sha}` using `MUAT_BUILD_VERSION` when set by CI, otherwise it falls back to `CARGO_PKG_VERSION+<git short sha>`.
