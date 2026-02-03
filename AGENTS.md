# AGENTS.md - Context Bootstrap

This file is a minimal project facts sheet for agents. For full contribution rules, see `CONTRIBUTING.md`.

## Project Facts

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

## Core Paths

| What                | Where                                 |
| ------------------- | ------------------------------------- |
| Core types/traits   | `crates/muat-core/src/`               |
| XRPC implementation | `crates/muat-xrpc/src/`               |
| File backend        | `crates/muat-file/src/`               |
| CLI                 | `crates/atproto-cli/src/`             |
| PRDs                | `docs/prd/`                           |
| Plans               | `docs/plans/`                         |
| Invariants          | `crates/muat-core/docs/Invariants.md` |
| CI workflows        | `.github/workflows/`                  |
