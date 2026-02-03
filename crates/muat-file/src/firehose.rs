//! Firehose stream for file-backed PDS.

use std::fs::File;
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::path::PathBuf;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;

use futures_util::Stream;
use notify::{RecursiveMode, Watcher};
use tokio::sync::mpsc;

use muat_core::Result;
use muat_core::error::{Error, InvalidInputError};
use muat_core::repo::{CommitEvent, CommitOperation, RepoEvent};

use crate::store::{FileStore, FirehoseLogEvent, FirehoseLogOp};

/// Firehose stream for file-backed PDS.
pub struct FileFirehose {
    inner: Pin<Box<dyn Stream<Item = Result<RepoEvent>> + Send>>,
}

impl FileFirehose {
    pub(crate) fn from_store(store: FileStore) -> Result<Self> {
        let pds_dir = store.root().join("pds");
        let firehose_path = store.firehose_path();

        std::fs::create_dir_all(&pds_dir).map_err(|e| {
            Error::InvalidInput(InvalidInputError::Other {
                message: format!("Failed to create PDS directory: {}", e),
            })
        })?;

        let (tx, mut rx) = mpsc::channel::<Result<RepoEvent>>(100);

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

        Ok(Self {
            inner: Box::pin(stream),
        })
    }
}

impl Stream for FileFirehose {
    type Item = Result<RepoEvent>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.inner.as_mut().poll_next(cx)
    }
}

fn read_new_firehose_events(
    firehose_path: &PathBuf,
    position: &std::sync::Arc<std::sync::Mutex<u64>>,
    tx: &mpsc::Sender<Result<RepoEvent>>,
) {
    if let Ok(mut file) = File::open(firehose_path) {
        let mut pos = position.lock().unwrap();
        if file.seek(SeekFrom::Start(*pos)).is_ok() {
            let reader = BufReader::new(&file);
            for line in reader.lines().map_while(|line| line.ok()) {
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
