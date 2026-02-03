//! Session storage for persisting login state.

use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};

use muat_core::types::{Did, PdsUrl};
use muat_core::{AccessToken, RefreshToken};
use muat_file::{FilePds, FileSession};
use muat_xrpc::XrpcSession;

use super::CliSession;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

/// Stored session data.
#[derive(Debug, Serialize, Deserialize)]
struct StoredSession {
    did: String,
    pds: String,
    access_token: String,
    refresh_token: Option<String>,
}

/// Get the session file path.
fn session_path() -> Result<PathBuf> {
    let dirs =
        ProjectDirs::from("", "", "atproto").context("Could not determine config directory")?;

    let data_dir = dirs.data_dir();
    fs::create_dir_all(data_dir).context("Failed to create data directory")?;

    Ok(data_dir.join("session.json"))
}

/// Save a session to disk.
pub async fn save_session(session: &CliSession) -> Result<()> {
    let access_token = session.access_token();

    let stored = StoredSession {
        did: session.did().to_string(),
        pds: session.pds().to_string(),
        access_token: access_token.as_str().to_string(),
        refresh_token: session.refresh_token().map(|t| t.as_str().to_string()),
    };

    let path = session_path()?;
    let json = serde_json::to_string_pretty(&stored)?;

    fs::write(&path, &json).context("Failed to write session file")?;

    // Set restrictive permissions (Unix only)
    #[cfg(unix)]
    {
        let mut perms = fs::metadata(&path)?.permissions();
        perms.set_mode(0o600);
        fs::set_permissions(&path, perms)?;
    }

    Ok(())
}

/// Load a session from disk.
pub async fn load_session() -> Result<Option<CliSession>> {
    let path = session_path()?;

    if !path.exists() {
        return Ok(None);
    }

    let json = fs::read_to_string(&path).context("Failed to read session file")?;
    let stored: StoredSession = serde_json::from_str(&json).context("Invalid session file")?;

    let pds = PdsUrl::new(&stored.pds).context("Invalid PDS URL in session")?;
    let did = Did::new(&stored.did).context("Invalid DID in session")?;

    let access_token = AccessToken::new(stored.access_token);
    let refresh_token = stored.refresh_token.map(RefreshToken::new);

    if pds.is_local() {
        let path = pds
            .to_file_path()
            .context("Failed to convert file:// URL to path")?;
        let file_pds = FilePds::new(&path, pds);
        let session = FileSession::from_persisted(file_pds, access_token)?;
        Ok(Some(CliSession::File(session)))
    } else {
        let session = XrpcSession::from_persisted(pds.clone(), did, access_token, refresh_token);
        if let Err(e) = session.refresh().await {
            tracing::warn!(error = %e, "Failed to refresh session, using existing tokens");
        }
        Ok(Some(CliSession::Xrpc(session)))
    }
}

/// Clear the stored session.
#[allow(dead_code)]
pub async fn clear_session() -> Result<()> {
    let path = session_path()?;

    if path.exists() {
        fs::remove_file(&path).context("Failed to remove session file")?;
    }

    Ok(())
}
