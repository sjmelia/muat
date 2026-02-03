//! Firehose stream for XRPC-backed PDS.

use std::pin::Pin;
use std::task::{Context, Poll};

use futures_util::{Stream, StreamExt};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tracing::{debug, error, info, trace, warn};

use muat_core::Result;
use muat_core::error::{Error, TransportError};
use muat_core::repo::RepoEvent;
use muat_core::types::PdsUrl;

/// Firehose stream for XRPC-backed PDS.
pub struct XrpcFirehose {
    inner: Pin<Box<dyn Stream<Item = Result<RepoEvent>> + Send>>,
}

impl XrpcFirehose {
    pub(crate) fn new<S>(stream: S) -> Self
    where
        S: Stream<Item = Result<RepoEvent>> + Send + 'static,
    {
        Self {
            inner: Box::pin(stream),
        }
    }

    pub async fn from_websocket(pds: &PdsUrl, cursor: Option<i64>) -> Result<Self> {
        let ws_url = build_ws_url(pds, cursor);
        info!(url = %ws_url, "Connecting to firehose");

        let (ws_stream, _) = connect_async(&ws_url).await.map_err(|e| {
            Error::Transport(TransportError::Connection {
                message: e.to_string(),
            })
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
                        yield Err(Error::Transport(TransportError::Connection {
                            message: e.to_string(),
                        }));
                        break;
                    }
                }
            }
        };

        Ok(Self::new(stream))
    }
}

impl Stream for XrpcFirehose {
    type Item = Result<RepoEvent>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.inner.as_mut().poll_next(cx)
    }
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

fn parse_ws_event(data: &[u8]) -> Result<RepoEvent> {
    let preview = data
        .iter()
        .take(32)
        .map(|b| format!("{:02x}", b))
        .collect::<String>();

    Ok(RepoEvent::Unknown {
        kind: format!("binary:{}", preview),
    })
}
