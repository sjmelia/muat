//! PDS URL type.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::PathBuf;
use std::str::FromStr;
use url::Url;

use crate::error::{Error, InvalidInputError};

/// A validated PDS (Personal Data Server) URL.
///
/// This type supports both network PDS URLs (HTTPS/HTTP) and local filesystem
/// PDS URLs (`file://`).
///
/// # Network URLs
///
/// Network URLs must use HTTPS (or HTTP for localhost) and are used to
/// connect to remote PDS instances.
///
/// # File URLs
///
/// File URLs (`file:///path/to/pds`) enable local-only development and testing
/// without running a network PDS. Records are stored on the filesystem.
///
/// # Example
///
/// ```
/// use muat_core::PdsUrl;
///
/// // Network PDS
/// let pds = PdsUrl::new("https://bsky.social").unwrap();
/// assert_eq!(pds.xrpc_url("com.atproto.server.createSession"),
///            "https://bsky.social/xrpc/com.atproto.server.createSession");
///
/// // Local filesystem PDS
/// let local = PdsUrl::new("file:///tmp/test-pds").unwrap();
/// assert!(local.is_local());
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct PdsUrl(Url);

impl PdsUrl {
    /// Create a new PDS URL from a string, validating the format.
    ///
    /// # Errors
    ///
    /// Returns an error if the URL is not valid or doesn't meet requirements.
    pub fn new(s: impl AsRef<str>) -> Result<Self, Error> {
        let s = s.as_ref();
        let url = Url::parse(s).map_err(|e| InvalidInputError::PdsUrl {
            value: s.to_string(),
            reason: e.to_string(),
        })?;

        Self::validate(&url, s)?;

        // Normalize: remove trailing slash
        let normalized = if url.path() == "/" {
            let mut u = url.clone();
            u.set_path("");
            u
        } else {
            url
        };

        Ok(Self(normalized))
    }

    /// Returns the XRPC endpoint URL for a given method.
    pub fn xrpc_url(&self, method: &str) -> String {
        // The URL crate always adds a trailing slash to root paths,
        // so we need to handle that when constructing the XRPC URL
        let base = self.0.as_str().trim_end_matches('/');
        format!("{}/xrpc/{}", base, method)
    }

    /// Returns the base URL as a string.
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    /// Returns the inner URL.
    pub fn as_url(&self) -> &Url {
        &self.0
    }

    /// Returns the host string.
    pub fn host(&self) -> Option<&str> {
        self.0.host_str()
    }

    /// Returns the URL scheme (e.g., "https", "http", "file").
    pub fn scheme(&self) -> &str {
        self.0.scheme()
    }

    /// Returns true if this is a local filesystem PDS (file:// URL).
    pub fn is_local(&self) -> bool {
        self.0.scheme() == "file"
    }

    /// Returns true if this is a network PDS (http:// or https:// URL).
    pub fn is_network(&self) -> bool {
        let scheme = self.0.scheme();
        scheme == "http" || scheme == "https"
    }

    /// Returns the filesystem path for file:// URLs.
    ///
    /// Returns `None` for non-file URLs.
    pub fn to_file_path(&self) -> Option<PathBuf> {
        if self.is_local() {
            self.0.to_file_path().ok()
        } else {
            None
        }
    }

    fn validate(url: &Url, original: &str) -> Result<(), Error> {
        // Must be absolute
        if url.cannot_be_a_base() {
            return Err(InvalidInputError::PdsUrl {
                value: original.to_string(),
                reason: "must be an absolute URL".to_string(),
            }
            .into());
        }

        let scheme = url.scheme();

        // Handle file:// URLs
        if scheme == "file" {
            // file:// URLs don't need a host, just a path
            if url.path().is_empty() {
                return Err(InvalidInputError::PdsUrl {
                    value: original.to_string(),
                    reason: "file:// URL must have a path".to_string(),
                }
                .into());
            }
            return Ok(());
        }

        // Must be HTTPS (or HTTP for localhost)
        let is_localhost = url
            .host_str()
            .is_some_and(|h| h == "localhost" || h == "127.0.0.1" || h == "::1");

        if scheme != "https" && !(scheme == "http" && is_localhost) {
            return Err(InvalidInputError::PdsUrl {
                value: original.to_string(),
                reason: "must use HTTPS (HTTP allowed only for localhost)".to_string(),
            }
            .into());
        }

        // Must have a host for network URLs
        if url.host_str().is_none() {
            return Err(InvalidInputError::PdsUrl {
                value: original.to_string(),
                reason: "must have a host".to_string(),
            }
            .into());
        }

        Ok(())
    }
}

impl fmt::Display for PdsUrl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for PdsUrl {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s)
    }
}

impl Serialize for PdsUrl {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.0.as_str())
    }
}

impl<'de> Deserialize<'de> for PdsUrl {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        PdsUrl::new(&s).map_err(serde::de::Error::custom)
    }
}

impl AsRef<str> for PdsUrl {
    fn as_ref(&self) -> &str {
        self.0.as_str()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_https_url() {
        let pds = PdsUrl::new("https://bsky.social").unwrap();
        assert_eq!(pds.host(), Some("bsky.social"));
    }

    #[test]
    fn valid_localhost_http() {
        let pds = PdsUrl::new("http://localhost:2583").unwrap();
        assert_eq!(pds.host(), Some("localhost"));
    }

    #[test]
    fn xrpc_url_construction() {
        let pds = PdsUrl::new("https://bsky.social").unwrap();
        assert_eq!(
            pds.xrpc_url("com.atproto.server.createSession"),
            "https://bsky.social/xrpc/com.atproto.server.createSession"
        );
    }

    #[test]
    fn normalizes_trailing_slash_in_xrpc_url() {
        let pds = PdsUrl::new("https://bsky.social/").unwrap();
        // The important thing is that xrpc_url works correctly
        assert_eq!(
            pds.xrpc_url("com.atproto.server.createSession"),
            "https://bsky.social/xrpc/com.atproto.server.createSession"
        );
    }

    #[test]
    fn invalid_http_non_localhost() {
        assert!(PdsUrl::new("http://bsky.social").is_err());
    }

    #[test]
    fn invalid_relative_url() {
        assert!(PdsUrl::new("/xrpc/method").is_err());
    }

    #[test]
    fn valid_file_url() {
        let pds = PdsUrl::new("file:///tmp/test-pds").unwrap();
        assert!(pds.is_local());
        assert!(!pds.is_network());
        assert_eq!(pds.scheme(), "file");
    }

    #[test]
    fn file_url_to_path() {
        #[cfg(unix)]
        {
            let pds = PdsUrl::new("file:///tmp/test-pds").unwrap();
            let path = pds.to_file_path().unwrap();
            assert_eq!(path, std::path::PathBuf::from("/tmp/test-pds"));
        }

        #[cfg(windows)]
        {
            let pds = PdsUrl::new("file:///C:/tmp/test-pds").unwrap();
            let path = pds.to_file_path().unwrap();
            assert_eq!(path, std::path::PathBuf::from(r"C:\tmp\test-pds"));
        }
    }

    #[test]
    fn network_url_not_local() {
        let pds = PdsUrl::new("https://bsky.social").unwrap();
        assert!(!pds.is_local());
        assert!(pds.is_network());
        assert!(pds.to_file_path().is_none());
    }
}
