# Orbit

A Rust toolkit for working with the AT Protocol (Bluesky's decentralized social network protocol).

## Overview

Orbit provides foundational tools for interacting with AT Protocol Personal Data Servers (PDS). It's designed as a modular, layered architecture where protocol primitives are separate from application-specific behavior.

Orbit supports both **network PDS instances** (via HTTPS) and **local filesystem-backed PDS** (via `file://` URLs) for development and testing.

### Crates

| Crate | Description | Docs |
|-------|-------------|------|
| `muat` | Core AT Protocol library - authentication, session management, repo operations, local PDS backend | [README](crates/muat/README.md) |
| `atproto-cli` | CLI tool for PDS exploration and debugging | [README](crates/atproto-cli/README.md) |

## Quick Start

### Installation

```bash
# Build from source
cargo build --release

# Install the CLI
cargo install --path crates/atproto-cli
```

### CLI Usage

```bash
# Check version
atproto --version

# Login to a PDS
atproto pds login --identifier alice.bsky.social --password your-app-password

# Check your session
atproto pds whoami

# Create a record
atproto pds create-record org.example.record --type org.example.record

# List records in a collection
atproto pds list-records app.bsky.feed.post

# Get a specific record
atproto pds get-record at://did:plc:xxx/app.bsky.feed.post/yyy

# Delete a record
atproto pds delete-record at://did:plc:xxx/org.example.record/yyy

# Subscribe to the firehose
atproto pds subscribe
```

### Local Development with file:// PDS

```bash
# Create a local account
atproto pds create-account alice.local --pds file://./pds

# Remove a local account
atproto pds remove-account did:plc:xxx --pds file://./pds
```

### Library Usage

```rust
use muat::{Session, Credentials, PdsUrl, Nsid};

#[tokio::main]
async fn main() -> Result<(), muat::Error> {
    // Connect to a PDS
    let pds = PdsUrl::new("https://bsky.social")?;
    let credentials = Credentials::new("alice.bsky.social", "app-password");

    // Create a session
    let session = Session::login(&pds, credentials).await?;
    println!("Logged in as: {}", session.did());

    // List records
    let collection = Nsid::new("app.bsky.feed.post")?;
    let records = session.list_records(session.did(), &collection, Some(10), None).await?;

    for record in records.records {
        println!("{}: {:?}", record.uri, record.value);
    }

    Ok(())
}
```

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│                    Applications                          │
│              (TUI, GUI, Agents, App-Views)              │
├─────────────────────────────────────────────────────────┤
│                    atproto-cli                           │
│              (Thin CLI wrapper over muat)               │
├─────────────────────────────────────────────────────────┤
│                        muat                              │
│    (Core protocol: XRPC, Auth, Session, Repo ops)       │
├─────────────────────────────────────────────────────────┤
│                    AT Protocol                           │
│                  (XRPC over HTTPS)                      │
└─────────────────────────────────────────────────────────┘
```

### Key Design Principles

1. **Session-centric API** - All authenticated operations flow through a `Session` object
2. **Strong typing** - Protocol types (`Did`, `Nsid`, `AtUri`, `RecordValue`) are validated at construction
3. **Schema-agnostic** - Record values use `RecordValue` (guarantees `$type` field), not typed lexicons
4. **Explicit over magic** - No hidden retries, no global state, no implicit defaults
5. **Local-first development** - `file://` URLs enable offline development without a network PDS

## Core Types

| Type | Description |
|------|-------------|
| `Did` | Decentralized Identifier (`did:plc:...`, `did:web:...`) |
| `Nsid` | Namespaced Identifier (`app.bsky.feed.post`) |
| `AtUri` | AT Protocol URI (`at://did/collection/rkey`) |
| `PdsUrl` | PDS URL (HTTPS for network, HTTP for localhost, `file://` for local) |
| `RecordValue` | Validated record payload (JSON object with `$type` field) |
| `Session` | Authenticated session with a PDS |
| `Credentials` | Login identifier + password |

## Configuration

### CLI Session Storage

Sessions are persisted in the XDG data directory:
- Linux: `~/.local/share/atproto/session.json`
- macOS: `~/Library/Application Support/atproto/session.json`

### Logging

The CLI supports verbosity levels:
- Default: warnings only
- `-v`: info level
- `-vv`: debug level
- `-vvv`: trace level
- `--json-logs`: structured JSON output

## Development

### Prerequisites

- Rust 2024 edition
- Cargo

### Building

```bash
# Check all crates
cargo check --workspace

# Run tests
cargo test --workspace

# Build release
cargo build --release --workspace
```

## CI/CD & Releases

### CI Pipeline

The CI pipeline runs on every push and pull request:

1. **fmt** - Code formatting check
2. **clippy** - Lints with warnings as errors
3. **unit_tests** - Unit tests with mock PDS
4. **integration_tests** - Real PDS tests (requires secrets)

### Builds from Main

CI can produce release-style binaries from `main` for manual testing:
- Triggered via workflow dispatch with `build_main: true`
- Builds for Linux x86_64 and Windows x86_64
- Artifacts available for 14 days

### Creating Releases

Releases are created by pushing an annotated tag:

```bash
git tag -a v0.2.0 -m "v0.2.0"
git push origin v0.2.0
```

This triggers the release workflow which:
- Builds binaries for Linux and Windows
- Generates SHA256 checksums
- Creates a GitHub Release with auto-generated notes
- Attaches all artifacts

### Supported Platforms

| Platform | Build | Tests |
|----------|-------|-------|
| Linux x86_64 | Yes | Unit + Integration |
| Windows x86_64 | Yes | Unit only |

### Project Structure

```
orbit/
├── Cargo.toml              # Workspace manifest
├── crates/
│   ├── muat/               # Core protocol library
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── types/      # Did, Nsid, AtUri, PdsUrl, Rkey
│   │   │   ├── auth/       # Credentials, Session, Tokens
│   │   │   ├── xrpc/       # HTTP client, endpoints
│   │   │   ├── repo/       # Repository operations, RecordValue
│   │   │   ├── backend/    # PDS backends (file://)
│   │   │   └── error.rs
│   │   └── docs/
│   │       └── Invariants.md
│   └── atproto-cli/        # CLI tool
│       ├── src/
│       │   ├── main.rs
│       │   ├── cli.rs
│       │   ├── commands/   # pds subcommands
│       │   ├── session/    # Session storage
│       │   └── output.rs
│       └── tests/
│           └── integration.rs
├── docs/
│   ├── prd/                # Product requirements
│   └── plans/              # Implementation plans
└── .github/
    └── workflows/          # CI/CD workflows
```

## License

MIT OR Apache-2.0

## Links

- [AT Protocol Specification](https://atproto.com/specs/atp)
- [Bluesky](https://bsky.app)
