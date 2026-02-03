# Implementation Plan: atproto-cli

> Note: This document reflects an earlier design. The current implementation uses `Pds`/`Session`/`RepoEventStream` with firehose on `Pds`. See `crates/muat/README.md` and `crates/muat/docs/Invariants.md` for current rules.


## Overview

This plan details the implementation of `atproto`, the CLI tool for PDS exploration. It is a thin wrapper over `muat` with no protocol logic of its own.

## Architecture

```
crates/atproto-cli/
├── Cargo.toml
├── src/
│   ├── main.rs             # Entry point, CLI setup
│   ├── cli.rs              # Clap argument definitions
│   ├── commands/
│   │   ├── mod.rs
│   │   └── pds/
│   │       ├── mod.rs
│   │       ├── login.rs
│   │       ├── whoami.rs
│   │       ├── list_records.rs
│   │       ├── get_record.rs
│   │       ├── delete_record.rs
│   │       └── subscribe.rs
│   ├── session/
│   │   ├── mod.rs
│   │   └── storage.rs      # Session persistence
│   └── output.rs           # Output formatting helpers
└── docs/
    └── prd/
        └── 001 - Atproto.md
```

## Implementation Phases

### Phase 1: CLI Structure

**Files:** `src/main.rs`, `src/cli.rs`

1. **Argument Parser** (using `clap`)
   ```rust
   #[derive(Parser)]
   struct Cli {
       #[command(subcommand)]
       command: Commands,

       /// Increase verbosity (-v, -vv, -vvv)
       #[arg(short, long, action = ArgAction::Count)]
       verbose: u8,

       /// Output logs as JSON
       #[arg(long)]
       json_logs: bool,
   }

   #[derive(Subcommand)]
   enum Commands {
       Pds(PdsCommands),
   }
   ```

2. **Logging Setup**
   - Initialize `tracing-subscriber` based on verbosity
   - Support JSON log format via `--json-logs`
   - Configure before any muat calls

### Phase 2: Session Storage

**Files:** `src/session/*.rs`

1. **Storage Location**
   - XDG base directory: `$XDG_DATA_HOME/atproto/` or `~/.local/share/atproto/`
   - Session file: `session.json`
   - Permissions: `0600` (user read/write only)

2. **Session File Format**
   ```json
   {
     "did": "did:plc:...",
     "pds": "https://bsky.social",
     "access_token": "...",
     "refresh_token": "...",
     "expires_at": "2024-01-01T00:00:00Z"
   }
   ```

3. **Storage Operations**
   - `load_session() -> Option<Session>` - Load from disk, refresh if expired
   - `save_session(&Session) -> Result<()>` - Persist to disk
   - `clear_session() -> Result<()>` - Delete session file

### Phase 3: PDS Commands

**Files:** `src/commands/pds/*.rs`

#### 3.1 Login (`login.rs`)

```rust
#[derive(Args)]
struct LoginArgs {
    #[arg(long)]
    identifier: String,

    #[arg(long)]
    password: String,

    #[arg(long, default_value = "https://bsky.social")]
    pds: String,
}
```

Implementation:
1. Parse PDS URL
2. Create credentials
3. Call `Session::login()`
4. Save session to storage
5. Print DID and PDS URL

#### 3.2 Whoami (`whoami.rs`)

```rust
#[derive(Args)]
struct WhoamiArgs {}
```

Implementation:
1. Load session from storage (error if none)
2. Print DID, PDS URL, token expiry

#### 3.3 List Records (`list_records.rs`)

```rust
#[derive(Args)]
struct ListRecordsArgs {
    #[arg(long)]
    repo: Option<String>,

    #[arg(long)]
    collection: String,

    #[arg(long)]
    limit: Option<u32>,

    #[arg(long)]
    cursor: Option<String>,

    #[arg(long)]
    pretty: bool,
}
```

Implementation:
1. Load session
2. Build `muat::ListRecordsArgs`
3. Call `session.list_records()`
4. Print records as JSON (pretty or compact)
5. Print cursor if present

#### 3.4 Get Record (`get_record.rs`)

```rust
#[derive(Args)]
struct GetRecordArgs {
    #[arg(long)]
    uri: Option<String>,

    #[arg(long)]
    repo: Option<String>,

    #[arg(long)]
    collection: Option<String>,

    #[arg(long)]
    rkey: Option<String>,
}
```

Implementation:
1. Load session
2. Parse URI or construct from repo/collection/rkey
3. Call `session.get_record()`
4. Print record JSON

#### 3.5 Delete Record (`delete_record.rs`)

```rust
#[derive(Args)]
struct DeleteRecordArgs {
    #[arg(long)]
    uri: Option<String>,

    #[arg(long)]
    repo: Option<String>,

    #[arg(long)]
    collection: Option<String>,

    #[arg(long)]
    rkey: Option<String>,
}
```

Implementation:
1. Load session
2. Parse URI or construct from components
3. Call `session.delete_record()`
4. Print deleted URI

#### 3.6 Subscribe (`subscribe.rs`)

```rust
#[derive(Args)]
struct SubscribeArgs {
    #[arg(long)]
    cursor: Option<String>,

    #[arg(long)]
    json: bool,

    #[arg(long)]
    filter: Option<String>,
}
```

Implementation:
1. Load session
2. Create event handler (JSON or human-readable)
3. Call `session.subscribe_repos(handler)`
4. Print events as they arrive
5. Handle Ctrl+C gracefully

### Phase 4: Output Formatting

**File:** `src/output.rs`

1. **JSON Output**
   - Pretty-print with `serde_json::to_string_pretty`
   - Compact with `serde_json::to_string`

2. **Human-Readable Output**
   - Colorized output using `colored` or `owo-colors`
   - Structured event summaries for subscribe

## Dependencies

```toml
[dependencies]
muat = { path = "../muat" }
clap = { version = "4", features = ["derive"] }
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["json", "env-filter"] }
directories = "5"
anyhow = "1"
colored = "2"
```

## Testing Strategy

1. **Unit tests** for session storage
2. **Integration tests** against muat (mocked)
3. **Manual testing** against Bluesky PDS

## Success Criteria

- [ ] `atproto pds login` authenticates and persists session
- [ ] `atproto pds whoami` displays session info
- [ ] `atproto pds list-records` lists collection records
- [ ] `atproto pds get-record` fetches single record
- [ ] `atproto pds delete-record` deletes record
- [ ] `atproto pds subscribe` streams repo events
- [ ] All protocol logic delegated to `muat`
- [ ] Session stored securely in XDG directory
- [ ] Verbosity flags control log output
