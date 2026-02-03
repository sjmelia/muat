# PRD-009: Backend Split, Tokens, Coverage

> Status: Done

## Summary

This PRD captures the completed refactor that split the original `muat` crate into three crates, introduced trait-based backends with first-class tokens, aligned the file backend with the XRPC API surface, and added coverage reporting to CI with a baseline threshold.

## Goals Delivered

1. Split the original library into `muat-core`, `muat-xrpc`, and `muat-file`.
2. Define trait-based interfaces for `Pds`, `Session`, and `Firehose` in `muat-core`.
3. Make tokens first-class, opaque values and expose them to consumers for session persistence.
4. Align file backend behavior with XRPC by enforcing auth checks and returning tokens.
5. Update CLI to select backend by URL scheme and persist tokens.
6. Update docs to reflect the new crate layout and invariants.
7. Add coverage reporting in CI with a threshold based on current coverage plus headroom.

## Key Decisions

* `muat-core` holds all shared types, errors, traits, repo structures, and token/credential types.
* `muat-xrpc` implements `Pds` and `Session` over XRPC and exposes an explicit `refresh()` method.
* `muat-file` implements `Pds` and `Session` over the local filesystem with bcrypt-hashed passwords.
* `Firehose` is a `Stream<Item = Result<RepoEvent, Error>>` and is exposed from `Pds`.
* Token refresh is explicit for XRPC sessions; no auto-refresh in the trait.
* Coverage is reported via `cargo llvm-cov` and enforced in CI with a conservative floor.

## Implementation Summary

* New crates: `crates/muat-core`, `crates/muat-xrpc`, `crates/muat-file`.
* Trait APIs defined in `crates/muat-core/src/traits/`.
* File backend uses bcrypt hashing and JSON tokens containing DID + password hash.
* CLI now uses a `CliSession` wrapper to handle backend-specific sessions and persists tokens.
* Documentation updated across READMEs and invariants to reflect the new architecture.
* CI coverage job added to `default.yml` with a baseline threshold.

## Validation

* `cargo test --workspace` passes locally.
* `cargo llvm-cov --workspace --all-targets --all-features` is used for coverage.

