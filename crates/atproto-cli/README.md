# atproto-cli

CLI tool for AT Protocol PDS exploration and debugging.

## Overview

`atproto-cli` is a thin CLI wrapper over the `muat` library, providing command-line access to AT Protocol Personal Data Servers. It's designed for exploration, debugging, and scripting workflows.

## Installation

```bash
# Build from source
cargo build --release -p atproto-cli

# Or install directly
cargo install --path crates/atproto-cli
```

The binary will be available as `atproto`.

## Quick Start

```bash
# Login to a PDS (use an app password, not your main password)
atproto pds login --identifier your.handle.bsky.social --password your-app-password

# Check your session
atproto pds whoami

# List your posts
atproto pds list-records app.bsky.feed.post

# Subscribe to the firehose
atproto pds subscribe
```

## Commands

### Session Management

#### `pds login`

Create a new session (login to a PDS).

```bash
atproto pds login --identifier <HANDLE_OR_DID> --password <APP_PASSWORD> [--pds <URL>]
```

| Flag | Description | Default |
|------|-------------|---------|
| `--identifier`, `-i` | Handle or DID | Required |
| `--password`, `-p` | App password | Required |
| `--pds` | PDS URL | `https://bsky.social` |

#### `pds whoami`

Display the active session.

```bash
atproto pds whoami
```

#### `pds refresh-token`

Refresh the session tokens.

```bash
atproto pds refresh-token
```

### Record Operations

#### `pds list-records`

List records in a collection.

```bash
atproto pds list-records <COLLECTION> [OPTIONS]
```

| Argument/Flag | Description | Default |
|---------------|-------------|---------|
| `<COLLECTION>` | Collection NSID (e.g., `app.bsky.feed.post`) | Required |
| `--repo` | Repository DID | Session DID |
| `--limit` | Maximum number of records | None |
| `--cursor` | Pagination cursor | None |
| `--pretty` | Pretty-print JSON output | false |

Examples:
```bash
# List your posts
atproto pds list-records app.bsky.feed.post

# List another user's likes
atproto pds list-records app.bsky.feed.like --repo did:plc:xxx

# Paginate through results
atproto pds list-records app.bsky.feed.post --limit 10 --cursor "..."
```

#### `pds get-record`

Fetch a single record.

```bash
atproto pds get-record [URI] [OPTIONS]
```

| Argument/Flag | Description |
|---------------|-------------|
| `[URI]` | AT URI of the record |
| `--repo` | Repository DID (alternative to URI) |
| `--collection` | Collection NSID (alternative to URI) |
| `--rkey` | Record key (alternative to URI) |

Examples:
```bash
# Using AT URI
atproto pds get-record at://did:plc:xxx/app.bsky.feed.post/yyy

# Using components
atproto pds get-record --collection app.bsky.feed.post --rkey 3jui7kd54zh2y
```

#### `pds delete-record`

Delete a record.

```bash
atproto pds delete-record [URI] [OPTIONS]
```

| Argument/Flag | Description |
|---------------|-------------|
| `[URI]` | AT URI of the record to delete |
| `--repo` | Repository DID (alternative to URI) |
| `--collection` | Collection NSID (alternative to URI) |
| `--rkey` | Record key (alternative to URI) |

### Streaming

#### `pds subscribe`

Subscribe to repository events (firehose).

```bash
atproto pds subscribe [OPTIONS]
```

| Flag | Description | Default |
|------|-------------|---------|
| `--pds` | PDS URL to subscribe to | Session PDS |
| `--cursor` | Sequence number to start from | Latest |

The command outputs JSON events for commits, identity changes, handle updates, account status, and tombstones.

## Global Options

| Flag | Description |
|------|-------------|
| `-v`, `--verbose` | Increase verbosity (-v, -vv, -vvv) |
| `--json-logs` | Output logs as JSON |

## Session Storage

Sessions are persisted in the XDG data directory:

| Platform | Path |
|----------|------|
| Linux | `~/.local/share/atproto/session.json` |
| macOS | `~/Library/Application Support/atproto/session.json` |
| Windows | `{FOLDERID_RoamingAppData}/atproto/session.json` |

## Testing

### Integration Tests

Integration tests require real PDS credentials:

```bash
export ATPROTO_TEST_IDENTIFIER="your.handle"
export ATPROTO_TEST_PASSWORD="your-app-password"
cargo test -p atproto-cli --test integration
```

Tests are skipped automatically if credentials are not set.

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | Error (see stderr for details) |

## License

MIT OR Apache-2.0

## See Also

- [Orbit workspace README](../../README.md)
- [muat library](../muat/README.md)
- [AT Protocol Specification](https://atproto.com/specs/atp)
