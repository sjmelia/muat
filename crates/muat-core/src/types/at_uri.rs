//! AT URI type.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

use super::{Did, Nsid, Rkey};
use crate::error::{Error, InvalidInputError};

/// A validated AT Protocol URI.
///
/// AT URIs identify specific records in the AT Protocol network.
/// Format: `at://<repo>/<collection>/<rkey>`
///
/// # Example
///
/// ```
/// use muat_core::AtUri;
///
/// let uri = AtUri::new("at://did:plc:z72i7hdynmk6r22z27h6tvur/app.bsky.feed.post/3jui7kd54zh2y").unwrap();
/// assert_eq!(uri.collection().as_str(), "app.bsky.feed.post");
/// assert_eq!(uri.rkey().as_str(), "3jui7kd54zh2y");
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct AtUri {
    repo: Did,
    collection: Nsid,
    rkey: Rkey,
}

impl AtUri {
    /// Create a new AT URI from a string, validating the format.
    ///
    /// # Errors
    ///
    /// Returns an error if the string is not a valid AT URI format.
    pub fn new(s: impl AsRef<str>) -> Result<Self, Error> {
        let s = s.as_ref();
        Self::parse(s)
    }

    /// Create an AT URI from its components.
    pub fn from_parts(repo: Did, collection: Nsid, rkey: Rkey) -> Self {
        Self {
            repo,
            collection,
            rkey,
        }
    }

    /// Returns the repository (DID).
    pub fn repo(&self) -> &Did {
        &self.repo
    }

    /// Returns the collection (NSID).
    pub fn collection(&self) -> &Nsid {
        &self.collection
    }

    /// Returns the record key.
    pub fn rkey(&self) -> &Rkey {
        &self.rkey
    }

    fn parse(s: &str) -> Result<Self, Error> {
        // Format: at://<repo>/<collection>/<rkey>
        let rest = s
            .strip_prefix("at://")
            .ok_or_else(|| InvalidInputError::AtUri {
                value: s.to_string(),
                reason: "must start with 'at://'".to_string(),
            })?;

        // Split into parts
        let parts: Vec<&str> = rest.splitn(3, '/').collect();

        if parts.len() != 3 {
            return Err(InvalidInputError::AtUri {
                value: s.to_string(),
                reason: "must have format 'at://<repo>/<collection>/<rkey>'".to_string(),
            }
            .into());
        }

        let repo = Did::new(parts[0]).map_err(|_| InvalidInputError::AtUri {
            value: s.to_string(),
            reason: format!("invalid DID: {}", parts[0]),
        })?;

        let collection = Nsid::new(parts[1]).map_err(|_| InvalidInputError::AtUri {
            value: s.to_string(),
            reason: format!("invalid NSID: {}", parts[1]),
        })?;

        let rkey = Rkey::new(parts[2]).map_err(|_| InvalidInputError::AtUri {
            value: s.to_string(),
            reason: format!("invalid rkey: {}", parts[2]),
        })?;

        Ok(Self {
            repo,
            collection,
            rkey,
        })
    }
}

impl fmt::Display for AtUri {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "at://{}/{}/{}", self.repo, self.collection, self.rkey)
    }
}

impl FromStr for AtUri {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s)
    }
}

impl Serialize for AtUri {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for AtUri {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        AtUri::new(&s).map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_at_uri() {
        let uri =
            AtUri::new("at://did:plc:z72i7hdynmk6r22z27h6tvur/app.bsky.feed.post/3jui7kd54zh2y")
                .unwrap();

        assert_eq!(uri.repo().as_str(), "did:plc:z72i7hdynmk6r22z27h6tvur");
        assert_eq!(uri.collection().as_str(), "app.bsky.feed.post");
        assert_eq!(uri.rkey().as_str(), "3jui7kd54zh2y");
    }

    #[test]
    fn roundtrip() {
        let original = "at://did:plc:z72i7hdynmk6r22z27h6tvur/app.bsky.feed.post/3jui7kd54zh2y";
        let uri = AtUri::new(original).unwrap();
        assert_eq!(uri.to_string(), original);
    }

    #[test]
    fn from_parts() {
        let repo = Did::new("did:plc:z72i7hdynmk6r22z27h6tvur").unwrap();
        let collection = Nsid::new("app.bsky.feed.post").unwrap();
        let rkey = Rkey::new("3jui7kd54zh2y").unwrap();

        let uri = AtUri::from_parts(repo, collection, rkey);
        assert_eq!(
            uri.to_string(),
            "at://did:plc:z72i7hdynmk6r22z27h6tvur/app.bsky.feed.post/3jui7kd54zh2y"
        );
    }

    #[test]
    fn invalid_missing_prefix() {
        assert!(AtUri::new("did:plc:z72i7hdynmk6r22z27h6tvur/app.bsky.feed.post/rkey").is_err());
    }

    #[test]
    fn invalid_missing_rkey() {
        assert!(AtUri::new("at://did:plc:z72i7hdynmk6r22z27h6tvur/app.bsky.feed.post").is_err());
    }
}
