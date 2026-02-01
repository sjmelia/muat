//! Repository streaming (subscription) support.
//!
//! This module provides streaming access to repository events via the AT Protocol
//! firehose. It supports both network PDS (via WebSocket) and file-based PDS
//! (via file watching) through a unified Stream interface.
//!
//! # Example
//!
//! ```no_run
//! use muat::{Session, Credentials, PdsUrl};
//! use muat::repo::RepoEvent;
//! use futures_util::StreamExt;
//!
//! # async fn example() -> Result<(), muat::Error> {
//! # let pds = PdsUrl::new("https://bsky.social")?;
//! # let session = Session::login(&pds, Credentials::new("x", "y")).await?;
//! let mut stream = session.subscribe_repos()?;
//!
//! while let Some(result) = stream.next().await {
//!     match result {
//!         Ok(RepoEvent::Commit(commit)) => {
//!             println!("Commit from {}: {} ops", commit.repo, commit.ops.len());
//!         }
//!         Ok(_) => {}
//!         Err(e) => eprintln!("Error: {}", e),
//!     }
//! }
//! # Ok(())
//! # }
//! ```

use std::fs::File;
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::path::PathBuf;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;

use futures_util::{Stream, StreamExt};
use notify::{RecursiveMode, Watcher};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tracing::{debug, error, info, trace, warn};

use crate::auth::Session;
use crate::backend::{BackendKind, FirehoseEvent, FirehoseOp};
use crate::error::{Error, InvalidInputError, TransportError};
use crate::types::PdsUrl;

/// A repository event from the subscription stream.
#[derive(Debug, Clone)]
pub enum RepoEvent {
    /// A commit event containing repository changes.
    Commit(CommitEvent),

    /// An identity update event.
    Identity(IdentityEvent),

    /// A handle update event.
    Handle(HandleEvent),

    /// The stream info event (sent at connection start).
    Info(InfoEvent),

    /// An unknown event type.
    Unknown { kind: String },
}

/// A commit event from the repository.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitEvent {
    /// The repository DID.
    pub repo: String,

    /// The commit revision.
    pub rev: String,

    /// Sequence number.
    pub seq: i64,

    /// Timestamp of the commit.
    pub time: String,

    /// Operations in this commit.
    #[serde(default)]
    pub ops: Vec<CommitOperation>,
}

/// An operation within a commit.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitOperation {
    /// The path (collection/rkey).
    pub path: String,

    /// The operation action ("create", "update", or "delete").
    pub action: String,

    /// The CID of the record (for creates/updates).
    pub cid: Option<String>,
}

/// An identity update event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentityEvent {
    /// The DID.
    pub did: String,

    /// Sequence number.
    pub seq: i64,

    /// Timestamp.
    pub time: String,
}

/// A handle update event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandleEvent {
    /// The DID.
    pub did: String,

    /// The new handle.
    pub handle: String,

    /// Sequence number.
    pub seq: i64,

    /// Timestamp.
    pub time: String,
}

/// Stream info event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InfoEvent {
    /// The name of the stream.
    pub name: String,

    /// Optional message.
    pub message: Option<String>,
}

/// A stream of repository events.
///
/// This is an async stream that yields `RepoEvent` items. It works uniformly
/// for both network PDS (WebSocket) and file-based PDS (file watching).
///
/// The stream should be polled continuously to receive events. Use with
/// `futures_util::StreamExt` for convenient methods like `next()`.
pub struct RepoEventStream {
    inner: Pin<Box<dyn Stream<Item = Result<RepoEvent, Error>> + Send>>,
}

impl RepoEventStream {
    /// Create a new stream from an async stream.
    fn new<S>(stream: S) -> Self
    where
        S: Stream<Item = Result<RepoEvent, Error>> + Send + 'static,
    {
        Self {
            inner: Box::pin(stream),
        }
    }

    /// Create a stream for a WebSocket-based PDS.
    pub(crate) async fn from_websocket(pds: &PdsUrl, cursor: Option<i64>) -> Result<Self, Error> {
        let ws_url = build_ws_url(pds, cursor);
        info!(url = %ws_url, "Connecting to repo subscription");

        let (ws_stream, _) =
            connect_async(&ws_url)
                .await
                .map_err(|e| TransportError::Connection {
                    message: e.to_string(),
                })?;

        debug!("WebSocket connected, listening for events");

        let stream = async_stream::stream! {
            let (mut write, mut read) = ws_stream.split();

            while let Some(msg) = read.next().await {
                match msg {
                    Ok(Message::Binary(data)) => {
                        yield parse_ws_event(&data);
                    }
                    Ok(Message::Ping(data)) => {
                        trace!("Received ping");
                        if let Err(e) = futures_util::SinkExt::send(&mut write, Message::Pong(data)).await {
                            warn!(error = %e, "Failed to send pong");
                        }
                    }
                    Ok(Message::Close(frame)) => {
                        info!(?frame, "WebSocket closed by server");
                        break;
                    }
                    Ok(Message::Text(text)) => {
                        trace!(text = %text, "Received text message");
                    }
                    Ok(Message::Pong(_)) => {
                        trace!("Received pong");
                    }
                    Ok(Message::Frame(_)) => {
                        // Raw frame, ignore
                    }
                    Err(e) => {
                        error!(error = %e, "WebSocket error");
                        yield Err(TransportError::Connection {
                            message: e.to_string(),
                        }.into());
                        break;
                    }
                }
            }
        };

        Ok(Self::new(stream))
    }

    /// Create a stream for a file-based PDS.
    pub(crate) fn from_file(root: PathBuf, did: String) -> Result<Self, Error> {
        let pds_dir = root.join("pds");
        let firehose_path = pds_dir.join("firehose.jsonl");

        // Ensure the pds directory exists
        std::fs::create_dir_all(&pds_dir).map_err(|e| {
            Error::InvalidInput(InvalidInputError::Other {
                message: format!("Failed to create PDS directory: {}", e),
            })
        })?;

        // Create tokio channel for events
        let (tx, mut rx) = mpsc::channel::<Result<RepoEvent, Error>>(100);

        // Track file position for tailing
        let initial_pos = if firehose_path.exists() {
            std::fs::metadata(&firehose_path)
                .map(|m| m.len())
                .unwrap_or(0)
        } else {
            0
        };

        let position = std::sync::Arc::new(std::sync::Mutex::new(initial_pos));
        let position_clone = position.clone();
        let firehose_path_clone = firehose_path.clone();
        let tx_clone = tx.clone();
        let did_clone = did.clone();

        // Create file watcher
        let mut watcher = notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
            if let Ok(event) = res {
                // Only process modify/create events
                if !matches!(
                    event.kind,
                    notify::EventKind::Modify(_) | notify::EventKind::Create(_)
                ) {
                    return;
                }

                // Check if the event is for our firehose file
                let is_firehose = event
                    .paths
                    .iter()
                    .any(|p| p.file_name().is_some_and(|n| n == "firehose.jsonl"));

                if !is_firehose {
                    return;
                }

                // Read new lines from the firehose
                read_new_firehose_events(
                    &firehose_path_clone,
                    &position_clone,
                    &tx_clone,
                    &did_clone,
                );
            }
        })
        .map_err(|e| {
            Error::InvalidInput(InvalidInputError::Other {
                message: format!("Failed to create file watcher: {}", e),
            })
        })?;

        // Start watching the pds directory
        watcher
            .watch(&pds_dir, RecursiveMode::NonRecursive)
            .map_err(|e| {
                Error::InvalidInput(InvalidInputError::Other {
                    message: format!("Failed to watch directory: {}", e),
                })
            })?;

        // Spawn a background task to keep the watcher alive and do periodic polling
        let firehose_path_poll = firehose_path.clone();
        let did_poll = did.clone();
        tokio::spawn(async move {
            let _watcher = watcher; // Keep watcher alive
            let mut interval = tokio::time::interval(Duration::from_millis(500));

            loop {
                interval.tick().await;
                // Periodic poll as fallback for platforms where notify doesn't work well
                read_new_firehose_events(&firehose_path_poll, &position, &tx, &did_poll);
            }
        });

        // Create the stream from the receiver
        let stream = async_stream::stream! {
            while let Some(event) = rx.recv().await {
                yield event;
            }
        };

        Ok(Self::new(stream))
    }
}

impl Stream for RepoEventStream {
    type Item = Result<RepoEvent, Error>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.inner.as_mut().poll_next(cx)
    }
}

/// Read new firehose events from the file and send them to the channel.
fn read_new_firehose_events(
    firehose_path: &PathBuf,
    position: &std::sync::Arc<std::sync::Mutex<u64>>,
    tx: &mpsc::Sender<Result<RepoEvent, Error>>,
    did: &str,
) {
    if let Ok(mut file) = File::open(firehose_path) {
        let mut pos = position.lock().unwrap();
        if file.seek(SeekFrom::Start(*pos)).is_ok() {
            let reader = BufReader::new(&file);
            for line in reader.lines() {
                if let Ok(line) = line {
                    if line.trim().is_empty() {
                        continue;
                    }
                    if let Ok(event) = serde_json::from_str::<FirehoseEvent>(&line) {
                        let repo_event = firehose_to_repo_event(&event, did);
                        let _ = tx.blocking_send(Ok(repo_event));
                    }
                }
            }
            if let Ok(new_pos) = file.stream_position() {
                *pos = new_pos;
            }
        }
    }
}

/// Convert a file-based FirehoseEvent to a RepoEvent.
fn firehose_to_repo_event(event: &FirehoseEvent, did: &str) -> RepoEvent {
    // Parse the AT URI to extract collection and rkey
    // URI format: at://did:plc:xxx/collection.name/rkey
    let uri = &event.uri;
    let path = if let Some(rest) = uri.strip_prefix("at://") {
        // Skip the DID part
        if let Some(slash_pos) = rest.find('/') {
            rest[slash_pos + 1..].to_string()
        } else {
            "unknown".to_string()
        }
    } else {
        "unknown".to_string()
    };

    let action = match event.op {
        FirehoseOp::Create => "create",
        FirehoseOp::Delete => "delete",
    };

    // Generate a simple sequence number from the timestamp
    let seq = chrono::DateTime::parse_from_rfc3339(&event.time)
        .map(|dt| dt.timestamp_micros())
        .unwrap_or(0);

    RepoEvent::Commit(CommitEvent {
        repo: did.to_string(),
        rev: format!("rev-{}", seq),
        seq,
        time: event.time.clone(),
        ops: vec![CommitOperation {
            path,
            action: action.to_string(),
            cid: None,
        }],
    })
}

/// Build WebSocket URL for subscription.
fn build_ws_url(pds: &PdsUrl, cursor: Option<i64>) -> String {
    let base = pds.as_str();
    // Convert https:// to wss://
    let ws_base = base
        .replace("https://", "wss://")
        .replace("http://", "ws://");

    let mut url = format!("{}/xrpc/com.atproto.sync.subscribeRepos", ws_base);

    if let Some(cursor) = cursor {
        url.push_str(&format!("?cursor={}", cursor));
    }

    url
}

/// Parse a WebSocket event.
fn parse_ws_event(data: &[u8]) -> Result<RepoEvent, Error> {
    // The AT Protocol uses CBOR-encoded CAR files for the firehose
    // For simplicity, we'll parse what we can from the header
    // A full implementation would use a CBOR parser

    // Try to extract the event type from the data
    // This is a simplified implementation - a production version
    // would properly parse the CBOR/CAR format

    // For now, return Unknown with a hex preview
    let preview = data
        .iter()
        .take(32)
        .map(|b| format!("{:02x}", b))
        .collect::<String>();

    Ok(RepoEvent::Unknown {
        kind: format!("binary:{}", preview),
    })
}

// ============================================================================
// Session integration
// ============================================================================

impl Session {
    /// Subscribe to repository events.
    ///
    /// Returns a stream of repository events. This works uniformly for both
    /// network PDS (via WebSocket) and file-based PDS (via file watching).
    ///
    /// # Example
    ///
    /// ```no_run
    /// use muat::{Session, Credentials, PdsUrl};
    /// use muat::repo::RepoEvent;
    /// use futures_util::StreamExt;
    ///
    /// # async fn example() -> Result<(), muat::Error> {
    /// # let pds = PdsUrl::new("https://bsky.social")?;
    /// # let session = Session::login(&pds, Credentials::new("x", "y")).await?;
    /// let mut stream = session.subscribe_repos()?;
    ///
    /// while let Some(result) = stream.next().await {
    ///     match result {
    ///         Ok(RepoEvent::Commit(commit)) => {
    ///             println!("Commit from {}: {} ops", commit.repo, commit.ops.len());
    ///         }
    ///         Ok(_) => {}
    ///         Err(e) => eprintln!("Error: {}", e),
    ///     }
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn subscribe_repos(&self) -> Result<RepoEventStream, Error> {
        self.subscribe_repos_from(None)
    }

    /// Subscribe to repository events with an optional starting cursor.
    ///
    /// For WebSocket connections, the cursor is the sequence number to resume from.
    /// For file-based PDS, the cursor is currently ignored (always starts from current position).
    pub fn subscribe_repos_from(&self, cursor: Option<i64>) -> Result<RepoEventStream, Error> {
        match self.backend() {
            BackendKind::File(file_backend) => {
                let root = file_backend.root().to_path_buf();
                let did = self.did().to_string();
                RepoEventStream::from_file(root, did)
            }
            BackendKind::Xrpc(_) => {
                // For XRPC, we need to create the stream asynchronously
                // Use a channel to bridge sync -> async
                let pds = self.pds().clone();
                let (tx, mut rx) = mpsc::channel::<Result<RepoEvent, Error>>(100);

                tokio::spawn(async move {
                    match RepoEventStream::from_websocket(&pds, cursor).await {
                        Ok(mut stream) => {
                            while let Some(event) = stream.next().await {
                                if tx.send(event).await.is_err() {
                                    break;
                                }
                            }
                        }
                        Err(e) => {
                            let _ = tx.send(Err(e)).await;
                        }
                    }
                });

                let stream = async_stream::stream! {
                    while let Some(event) = rx.recv().await {
                        yield event;
                    }
                };

                Ok(RepoEventStream::new(stream))
            }
        }
    }
}
