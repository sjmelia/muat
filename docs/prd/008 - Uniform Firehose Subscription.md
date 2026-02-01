# PRD-008: Uniform Firehose Subscription API

## Status

Done

## Motivation

The orbit chat application (built on muat) needed event-driven updates when messages arrive. Initially, the file-based PDS had a separate `watch_firehose()` API on `FilePdsBackend`, while the network PDS used WebSocket subscriptions via a callback pattern. This created two problems:

1. **Different APIs for the same concept**: Consumers had to write separate code paths for file:// vs https:// PDS URLs
2. **Callback-based API**: The callback pattern didn't compose well with async/await and `tokio::select!`

We wanted a **uniform interface** where `session.subscribe_repos()` works identically for both file-based and network-based PDS, returning an async `Stream` that integrates naturally with tokio's async ecosystem.

Additionally, the file-based PDS directory layout was collection-centric (`/collections/<collection>/<did>/<rkey>.json`) which doesn't match the AT Protocol's repo-centric data model where repositories belong to users.

---

## Goals

1. Provide a single `session.subscribe_repos()` method that works uniformly for both file:// and https:// PDS URLs
2. Return an async `Stream` rather than using callbacks, enabling natural use with `tokio::select!`
3. Convert file-based firehose events to the same `RepoEvent` type used by network subscriptions
4. Fix the file-based PDS directory layout to be repo-centric: `/repos/<did>/collections/<collection>/<rkey>.json`
5. Maintain backwards compatibility for existing network PDS usage

---

## Non-Goals

* Changing the firehose file format (still `firehose.jsonl`)
* Adding new event types beyond what already exists
* Implementing cursor/replay functionality for file-based subscriptions
* Full MST/CAR support for file-based repos

---

## Design

### Uniform Stream API

`Session` exposes a single method:

```rust
impl Session {
    /// Subscribe to repository events.
    /// Works uniformly for both file:// and https:// PDS URLs.
    pub fn subscribe_repos(&self) -> Result<RepoEventStream, Error>;
}
```

`RepoEventStream` wraps a `Pin<Box<dyn Stream<Item = Result<RepoEvent, Error>> + Send>>` and provides:

* `next(&mut self) -> impl Future<Output = Option<Result<RepoEvent, Error>>>`
* Natural integration with `tokio::select!` and `StreamExt`

### Backend-Specific Implementation

**File-based PDS (`file://`)**:
* Uses `notify` crate to watch `firehose.jsonl` for changes
* Converts `FirehoseEvent` (create/delete with URI) to `RepoEvent::Commit`
* Tails the file from current position, emitting new events as they're appended
* Runs in a background task communicating via `mpsc` channel

**Network PDS (`https://`)**:
* Connects via WebSocket to `/xrpc/com.atproto.sync.subscribeRepos`
* Deserializes CBOR frames into `RepoEvent` variants
* Wraps the WebSocket stream as an async Stream

### RepoEvent Types

```rust
pub enum RepoEvent {
    Commit(CommitEvent),
    Identity(IdentityEvent),
    Handle(HandleEvent),
    Info(InfoEvent),
}

pub struct CommitEvent {
    pub seq: i64,
    pub repo: String,
    pub time: String,
    pub ops: Vec<CommitOperation>,
}

pub struct CommitOperation {
    pub path: String,    // e.g., "chat.orbit.message/abc123"
    pub action: String,  // "create" or "delete"
    pub cid: Option<String>,
}
```

### Repo-Centric Directory Layout

The file-based PDS now uses a repo-centric layout that mirrors AT Protocol's data model:

**Old (collection-centric)**:
```
$ROOT/pds/
├── accounts/<did>/account.json
├── collections/<collection>/<did>/<rkey>.json
└── firehose.jsonl
```

**New (repo-centric)**:
```
$ROOT/pds/
├── accounts/<did>/account.json
├── repos/<did>/collections/<collection>/<rkey>.json
└── firehose.jsonl
```

This change:
* Better reflects that repositories belong to users (DIDs)
* Simplifies operations like "delete all data for a user"
* Aligns with how the protocol conceptually organizes data

---

## Usage Example

```rust
use muat::Session;
use futures_util::StreamExt;

async fn watch_events(session: Session) -> Result<(), Error> {
    let mut stream = session.subscribe_repos()?;

    while let Some(result) = stream.next().await {
        match result {
            Ok(RepoEvent::Commit(commit)) => {
                for op in commit.ops {
                    println!("{}:{} {}", commit.repo, op.path, op.action);
                }
            }
            Ok(_) => {} // Handle other event types
            Err(e) => eprintln!("Stream error: {}", e),
        }
    }

    Ok(())
}
```

### With tokio::select!

```rust
tokio::select! {
    event = stream.next() => {
        if let Some(Ok(repo_event)) = event {
            handle_event(repo_event).await;
        }
    }
    _ = tokio::time::sleep(Duration::from_millis(50)) => {
        // Handle other work
    }
}
```

---

## Dependencies Added

* `async-stream = "0.3"` - For creating async streams from generators
* `tokio-stream = "0.1"` - For stream utilities
* `futures-util = "0.3"` - For `StreamExt` trait

---

## Error Handling

* File watcher errors are mapped to `Error::InvalidInput`
* Network errors continue to map to `Error::Transport`
* Stream termination (file deleted, WebSocket closed) returns `None`

---

## Testing

1. **Unit tests**: File backend streaming tests verify event conversion
2. **Integration tests**: Existing tests continue to pass with new layout
3. **CLI tests**: `atproto pds subscribe` command updated to use new stream API

---

## Migration Notes

* The directory layout change is **breaking** for existing file-based PDS data
* Existing `$ROOT/pds/collections/...` directories must be migrated to `$ROOT/pds/repos/...`
* The firehose file format is unchanged
* Network PDS users are unaffected

---

## Success Criteria

This PRD is complete when:

* `session.subscribe_repos()` returns a uniform `RepoEventStream` for both backends
* File-based events are converted to the same `RepoEvent` type as network events
* The stream API works naturally with `tokio::select!`
* The directory layout is repo-centric
* All existing tests pass
