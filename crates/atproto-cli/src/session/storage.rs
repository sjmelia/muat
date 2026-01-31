//! Session storage for persisting login state.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

use anyhow::{Context, Result};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};

use muat::{Did, PdsUrl, Session};

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
pub async fn save_session(session: &Session) -> Result<()> {
    let stored = StoredSession {
        did: session.did().to_string(),
        pds: session.pds().to_string(),
        access_token: session.export_access_token().await,
        refresh_token: session.export_refresh_token().await,
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
pub async fn load_session() -> Result<Option<Session>> {
    let path = session_path()?;

    if !path.exists() {
        return Ok(None);
    }

    let json = fs::read_to_string(&path).context("Failed to read session file")?;
    let stored: StoredSession = serde_json::from_str(&json).context("Invalid session file")?;

    let pds = PdsUrl::new(&stored.pds).context("Invalid PDS URL in session")?;
    let did = Did::new(&stored.did).context("Invalid DID in session")?;

    let session = Session::from_persisted(pds, did, stored.access_token, stored.refresh_token);

    // Try to refresh the session
    if let Err(e) = session.refresh().await {
        tracing::warn!(error = %e, "Failed to refresh session, using existing tokens");
    }

    Ok(Some(session))
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
