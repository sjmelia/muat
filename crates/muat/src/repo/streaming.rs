//! Repository streaming (subscription) support.

use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::future::Future;
use std::pin::Pin;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tracing::{debug, error, info, trace, warn};

use crate::auth::Session;
use crate::error::{Error, TransportError};
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

    /// The operation action.
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

/// Trait for handling repository events.
///
/// Implement this trait to process events from [`Session::subscribe_repos`].
pub trait RepoEventHandler: Send {
    /// Handle a repository event.
    ///
    /// Return `Ok(true)` to continue receiving events, `Ok(false)` to stop.
    fn handle(&mut self, event: RepoEvent) -> Pin<Box<dyn Future<Output = Result<bool, Error>> + Send + '_>>;
}

/// A function-based event handler.
impl<F> RepoEventHandler for F
where
    F: FnMut(RepoEvent) -> bool + Send,
{
    fn handle(&mut self, event: RepoEvent) -> Pin<Box<dyn Future<Output = Result<bool, Error>> + Send + '_>> {
        let continue_listening = (self)(event);
        Box::pin(async move { Ok(continue_listening) })
    }
}

/// An active subscription to repository events.
pub struct RepoSubscription {
    pds: PdsUrl,
    cursor: Option<i64>,
}

impl RepoSubscription {
    /// Create a new subscription configuration.
    pub fn new(pds: &PdsUrl) -> Self {
        Self {
            pds: pds.clone(),
            cursor: None,
        }
    }

    /// Set the starting cursor position.
    pub fn with_cursor(mut self, cursor: i64) -> Self {
        self.cursor = Some(cursor);
        self
    }

    /// Start the subscription and process events with the given handler.
    pub async fn run<H: RepoEventHandler>(&self, mut handler: H) -> Result<(), Error> {
        let ws_url = self.build_ws_url();
        info!(url = %ws_url, "Connecting to repo subscription");

        let (ws_stream, _) = connect_async(&ws_url)
            .await
            .map_err(|e| TransportError::Connection {
                message: e.to_string(),
            })?;

        let (mut write, mut read) = ws_stream.split();

        debug!("WebSocket connected, listening for events");

        while let Some(msg) = read.next().await {
            match msg {
                Ok(Message::Binary(data)) => {
                    match self.parse_event(&data) {
                        Ok(event) => {
                            trace!(?event, "Received event");
                            let should_continue = handler.handle(event).await?;
                            if !should_continue {
                                info!("Handler requested stop");
                                break;
                            }
                        }
                        Err(e) => {
                            warn!(error = %e, "Failed to parse event");
                        }
                    }
                }
                Ok(Message::Ping(data)) => {
                    trace!("Received ping");
                    if let Err(e) = write.send(Message::Pong(data)).await {
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
                    return Err(TransportError::Connection {
                        message: e.to_string(),
                    }
                    .into());
                }
            }
        }

        Ok(())
    }

    fn build_ws_url(&self) -> String {
        let base = self.pds.as_str();
        // Convert https:// to wss://
        let ws_base = base
            .replace("https://", "wss://")
            .replace("http://", "ws://");

        let mut url = format!("{}/xrpc/com.atproto.sync.subscribeRepos", ws_base);

        if let Some(cursor) = self.cursor {
            url.push_str(&format!("?cursor={}", cursor));
        }

        url
    }

    fn parse_event(&self, data: &[u8]) -> Result<RepoEvent, Error> {
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
}

impl Session {
    /// Subscribe to repository events.
    ///
    /// This connects to the firehose and streams repository commit events.
    /// The handler is called for each event; return `false` to stop.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use muat::{Session, Credentials, PdsUrl};
    /// use muat::repo::RepoEvent;
    ///
    /// # async fn example() -> Result<(), muat::Error> {
    /// # let pds = PdsUrl::new("https://bsky.social")?;
    /// # let session = Session::login(&pds, Credentials::new("x", "y")).await?;
    /// session.subscribe_repos(|event| {
    ///     match event {
    ///         RepoEvent::Commit(commit) => {
    ///             println!("Commit from {}: {} ops", commit.repo, commit.ops.len());
    ///         }
    ///         _ => {}
    ///     }
    ///     true // continue listening
    /// }).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn subscribe_repos<H: RepoEventHandler>(&self, handler: H) -> Result<(), Error> {
        let subscription = RepoSubscription::new(self.pds());
        subscription.run(handler).await
    }

    /// Subscribe to repository events with a starting cursor.
    pub async fn subscribe_repos_from<H: RepoEventHandler>(
        &self,
        cursor: i64,
        handler: H,
    ) -> Result<(), Error> {
        let subscription = RepoSubscription::new(self.pds()).with_cursor(cursor);
        subscription.run(handler).await
    }
}
