//! Firehose streaming support.

use std::fs::File;
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::path::PathBuf;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;

use futures_util::{Stream, StreamExt};
use notify::{RecursiveMode, Watcher};
use tokio::sync::mpsc;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tracing::{debug, error, info, trace, warn};

use crate::error::{Error, InvalidInputError, TransportError};
use crate::repo::{CommitEvent, CommitOperation, RepoEvent};
use crate::types::PdsUrl;

/// An event stored in the file-backed firehose log.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(crate) struct FirehoseLogEvent {
    /// The AT URI of the affected record.
    pub uri: String,
    /// ISO 8601 timestamp.
    pub time: String,
    /// The operation type.
    pub op: FirehoseLogOp,
}

/// The type of firehose operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum FirehoseLogOp {
    /// A record was created.
    Create,
    /// A record was deleted.
    Delete,
}

/// A stream of repository events.
pub struct RepoEventStream {
    inner: Pin<Box<dyn Stream<Item = Result<RepoEvent, Error>> + Send>>,
}

impl RepoEventStream {
    pub(crate) fn new<S>(stream: S) -> Self
    where
        S: Stream<Item = Result<RepoEvent, Error>> + Send + 'static,
    {
        Self {
            inner: Box::pin(stream),
        }
    }

    pub(crate) async fn from_websocket(pds: &PdsUrl, cursor: Option<i64>) -> Result<Self, Error> {
        let ws_url = build_ws_url(pds, cursor);
        info!(url = %ws_url, "Connecting to firehose");

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

    pub(crate) fn from_file(root: PathBuf) -> Result<Self, Error> {
        let pds_dir = root.join("pds");
        let firehose_path = pds_dir.join("firehose.jsonl");

        std::fs::create_dir_all(&pds_dir).map_err(|e| {
            Error::InvalidInput(InvalidInputError::Other {
                message: format!("Failed to create PDS directory: {}", e),
            })
        })?;

        let (tx, mut rx) = mpsc::channel::<Result<RepoEvent, Error>>(100);

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

        let mut watcher = notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
            if let Ok(event) = res {
                if !matches!(
                    event.kind,
                    notify::EventKind::Modify(_) | notify::EventKind::Create(_)
                ) {
                    return;
                }

                let is_firehose = event
                    .paths
                    .iter()
                    .any(|p| p.file_name().is_some_and(|n| n == "firehose.jsonl"));

                if !is_firehose {
                    return;
                }

                read_new_firehose_events(&firehose_path_clone, &position_clone, &tx_clone);
            }
        })
        .map_err(|e| {
            Error::InvalidInput(InvalidInputError::Other {
                message: format!("Failed to create file watcher: {}", e),
            })
        })?;

        watcher
            .watch(&pds_dir, RecursiveMode::NonRecursive)
            .map_err(|e| {
                Error::InvalidInput(InvalidInputError::Other {
                    message: format!("Failed to watch directory: {}", e),
                })
            })?;

        let firehose_path_poll = firehose_path.clone();
        tokio::spawn(async move {
            let _watcher = watcher;
            let mut interval = tokio::time::interval(Duration::from_millis(500));

            loop {
                interval.tick().await;
                read_new_firehose_events(&firehose_path_poll, &position, &tx);
            }
        });

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

fn read_new_firehose_events(
    firehose_path: &PathBuf,
    position: &std::sync::Arc<std::sync::Mutex<u64>>,
    tx: &mpsc::Sender<Result<RepoEvent, Error>>,
) {
    if let Ok(mut file) = File::open(firehose_path) {
        let mut pos = position.lock().unwrap();
        if file.seek(SeekFrom::Start(*pos)).is_ok() {
            let reader = BufReader::new(&file);
            for line in reader.lines().map_while(Result::ok) {
                if line.trim().is_empty() {
                    continue;
                }
                if let Ok(event) = serde_json::from_str::<FirehoseLogEvent>(&line) {
                    let repo_event = firehose_to_repo_event(&event);
                    let _ = tx.blocking_send(Ok(repo_event));
                }
            }
            if let Ok(new_pos) = file.stream_position() {
                *pos = new_pos;
            }
        }
    }
}

fn firehose_to_repo_event(event: &FirehoseLogEvent) -> RepoEvent {
    let uri = &event.uri;

    let (repo, path) = if let Some(rest) = uri.strip_prefix("at://") {
        if let Some(slash_pos) = rest.find('/') {
            let repo = rest[..slash_pos].to_string();
            let path = rest[slash_pos + 1..].to_string();
            (repo, path)
        } else {
            ("unknown".to_string(), "unknown".to_string())
        }
    } else {
        ("unknown".to_string(), "unknown".to_string())
    };

    let action = match event.op {
        FirehoseLogOp::Create => "create",
        FirehoseLogOp::Delete => "delete",
    };

    let seq = chrono::DateTime::parse_from_rfc3339(&event.time)
        .map(|dt| dt.timestamp_micros())
        .unwrap_or(0);

    RepoEvent::Commit(CommitEvent {
        repo,
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

fn build_ws_url(pds: &PdsUrl, cursor: Option<i64>) -> String {
    let base = pds.as_str();
    let ws_base = base
        .replace("https://", "wss://")
        .replace("http://", "ws://");

    let mut url = format!("{}/xrpc/com.atproto.sync.subscribeRepos", ws_base);

    if let Some(cursor) = cursor {
        url.push_str(&format!("?cursor={}", cursor));
    }

    url
}

fn parse_ws_event(data: &[u8]) -> Result<RepoEvent, Error> {
    let preview = data
        .iter()
        .take(32)
        .map(|b| format!("{:02x}", b))
        .collect::<String>();

    Ok(RepoEvent::Unknown {
        kind: format!("binary:{}", preview),
    })
}
