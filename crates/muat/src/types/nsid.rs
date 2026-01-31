//! Namespaced Identifier (NSID) type.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

use crate::error::{Error, InvalidInputError};

/// A validated AT Protocol Namespaced Identifier (NSID).
///
/// NSIDs use reverse-DNS notation to identify lexicon types and collections.
///
/// # Example
///
/// ```
/// use muat::Nsid;
///
/// let nsid = Nsid::new("app.bsky.feed.post").unwrap();
/// assert_eq!(nsid.authority(), "app.bsky");
/// assert_eq!(nsid.name(), "feed.post");
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Nsid(String);

impl Nsid {
    /// Create a new NSID from a string, validating the format.
    ///
    /// # Errors
    ///
    /// Returns an error if the string is not a valid NSID format.
    pub fn new(s: impl Into<String>) -> Result<Self, Error> {
        let s = s.into();
        Self::validate(&s)?;
        Ok(Self(s))
    }

    /// Returns the authority portion (first two segments).
    ///
    /// For "app.bsky.feed.post", returns "app.bsky".
    pub fn authority(&self) -> &str {
        let parts: Vec<&str> = self.0.splitn(3, '.').collect();
        if parts.len() >= 2 {
            let end = parts[0].len() + 1 + parts[1].len();
            &self.0[..end]
        } else {
            &self.0
        }
    }

    /// Returns the name portion (segments after authority).
    ///
    /// For "app.bsky.feed.post", returns "feed.post".
    pub fn name(&self) -> &str {
        let parts: Vec<&str> = self.0.splitn(3, '.').collect();
        if parts.len() >= 3 {
            parts[2]
        } else {
            ""
        }
    }

    /// Returns the full NSID string.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Returns the segments of the NSID.
    pub fn segments(&self) -> impl Iterator<Item = &str> {
        self.0.split('.')
    }

    fn validate(s: &str) -> Result<(), Error> {
        // NSID format: <authority>.<name>
        // Authority: reverse-DNS (at least 2 segments)
        // Name: at least 1 segment
        // Total: at least 3 segments

        if s.is_empty() {
            return Err(InvalidInputError::Nsid {
                value: s.to_string(),
                reason: "cannot be empty".to_string(),
            }
            .into());
        }

        let segments: Vec<&str> = s.split('.').collect();

        if segments.len() < 3 {
            return Err(InvalidInputError::Nsid {
                value: s.to_string(),
                reason: "must have at least 3 segments (e.g., 'app.bsky.feed')".to_string(),
            }
            .into());
        }

        // Validate each segment
        for (i, segment) in segments.iter().enumerate() {
            if segment.is_empty() {
                return Err(InvalidInputError::Nsid {
                    value: s.to_string(),
                    reason: format!("segment {} is empty", i + 1),
                }
                .into());
            }

            // Segments must start with a letter
            let first_char = segment.chars().next().unwrap();
            if !first_char.is_ascii_alphabetic() {
                return Err(InvalidInputError::Nsid {
                    value: s.to_string(),
                    reason: format!("segment '{}' must start with a letter", segment),
                }
                .into());
            }

            // Segments can only contain letters, numbers, and hyphens
            for c in segment.chars() {
                if !c.is_ascii_alphanumeric() && c != '-' {
                    return Err(InvalidInputError::Nsid {
                        value: s.to_string(),
                        reason: format!(
                            "segment '{}' contains invalid character '{}'",
                            segment, c
                        ),
                    }
                    .into());
                }
            }
        }

        // Total length check (max 317 per spec)
        if s.len() > 317 {
            return Err(InvalidInputError::Nsid {
                value: s.to_string(),
                reason: "exceeds maximum length of 317 characters".to_string(),
            }
            .into());
        }

        Ok(())
    }
}

impl fmt::Display for Nsid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for Nsid {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s)
    }
}

impl TryFrom<String> for Nsid {
    type Error = Error;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        Self::new(s)
    }
}

impl From<Nsid> for String {
    fn from(nsid: Nsid) -> Self {
        nsid.0
    }
}

impl AsRef<str> for Nsid {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_nsid() {
        let nsid = Nsid::new("app.bsky.feed.post").unwrap();
        assert_eq!(nsid.authority(), "app.bsky");
        assert_eq!(nsid.name(), "feed.post");
    }

    #[test]
    fn valid_three_segment_nsid() {
        let nsid = Nsid::new("com.example.record").unwrap();
        assert_eq!(nsid.authority(), "com.example");
        assert_eq!(nsid.name(), "record");
    }

    #[test]
    fn invalid_too_few_segments() {
        assert!(Nsid::new("app.bsky").is_err());
    }

    #[test]
    fn invalid_empty_segment() {
        assert!(Nsid::new("app..feed.post").is_err());
    }

    #[test]
    fn invalid_starts_with_number() {
        assert!(Nsid::new("1app.bsky.feed").is_err());
    }
}
