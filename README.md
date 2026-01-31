# Orbit

A Rust toolkit for working with the AT Protocol (Bluesky's decentralized social network protocol).

## Overview

Orbit provides foundational tools for interacting with AT Protocol Personal Data Servers (PDS). It's designed as a modular, layered architecture where protocol primitives are separate from application-specific behavior.

### Crates

| Crate | Description |
|-------|-------------|
| `muat` | Core AT Protocol library - authentication, session management, repo operations |
| `atproto-cli` | CLI tool for PDS exploration and debugging |

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
# Login to a PDS
atproto pds login --identifier alice.bsky.social --password your-app-password

# Check your session
atproto pds whoami

# List records in a collection
atproto pds list-records --collection app.bsky.feed.post

# Get a specific record
atproto pds get-record --uri at://did:plc:xxx/app.bsky.feed.post/yyy

# Subscribe to the firehose
atproto pds subscribe
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
2. **Strong typing** - Protocol types (`Did`, `Nsid`, `AtUri`) are validated at construction
3. **Schema-agnostic** - Record values are `serde_json::Value`, not typed lexicons
4. **Explicit over magic** - No hidden retries, no global state, no implicit defaults

## Core Types

| Type | Description |
|------|-------------|
| `Did` | Decentralized Identifier (`did:plc:...`, `did:web:...`) |
| `Nsid` | Namespaced Identifier (`app.bsky.feed.post`) |
| `AtUri` | AT Protocol URI (`at://did/collection/rkey`) |
| `PdsUrl` | PDS base URL (HTTPS required, HTTP for localhost only) |
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

### Project Structure

```
orbit/
├── Cargo.toml              # Workspace manifest
├── crates/
│   ├── muat/               # Core protocol library
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── types/      # Did, Nsid, AtUri, PdsUrl
│   │   │   ├── auth/       # Credentials, Session, Tokens
│   │   │   ├── xrpc/       # HTTP client, endpoints
│   │   │   ├── repo/       # Repository operations
│   │   │   └── error.rs
│   │   └── docs/
│   │       ├── prd/
│   │       └── Invariants.md
│   └── atproto-cli/        # CLI tool
│       ├── src/
│       │   ├── main.rs
│       │   ├── cli.rs
│       │   ├── commands/
│       │   ├── session/
│       │   └── output.rs
│       └── docs/
│           └── prd/
└── docs/
    └── plans/              # Implementation plans
```

## License

MIT OR Apache-2.0

## Links

- [AT Protocol Specification](https://atproto.com/specs/atp)
- [Bluesky](https://bsky.app)
