//! Decentralized Identifier (DID) type.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

use crate::error::{Error, InvalidInputError};

/// A validated Decentralized Identifier (DID).
///
/// DIDs in the AT Protocol typically use the `did:plc:` or `did:web:` methods.
///
/// # Example
///
/// ```
/// use muat::Did;
///
/// let did = Did::new("did:plc:z72i7hdynmk6r22z27h6tvur").unwrap();
/// assert_eq!(did.method(), "plc");
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Did(String);

impl Did {
    /// Create a new DID from a string, validating the format.
    ///
    /// # Errors
    ///
    /// Returns an error if the string is not a valid DID format.
    pub fn new(s: impl Into<String>) -> Result<Self, Error> {
        let s = s.into();
        Self::validate(&s)?;
        Ok(Self(s))
    }

    /// Returns the DID method (e.g., "plc" for "did:plc:...").
    pub fn method(&self) -> &str {
        // Safe because we validated at construction
        self.0
            .strip_prefix("did:")
            .and_then(|s| s.split(':').next())
            .unwrap_or("")
    }

    /// Returns the method-specific identifier.
    pub fn identifier(&self) -> &str {
        // Safe because we validated at construction
        self.0
            .strip_prefix("did:")
            .and_then(|s| s.split_once(':'))
            .map(|(_, id)| id)
            .unwrap_or("")
    }

    /// Returns the full DID string.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    fn validate(s: &str) -> Result<(), Error> {
        // Basic DID validation per AT Protocol spec
        // Format: did:<method>:<method-specific-id>
        if !s.starts_with("did:") {
            return Err(InvalidInputError::Did {
                value: s.to_string(),
                reason: "must start with 'did:'".to_string(),
            }
            .into());
        }

        let rest = &s[4..];
        let parts: Vec<&str> = rest.splitn(2, ':').collect();

        if parts.len() < 2 {
            return Err(InvalidInputError::Did {
                value: s.to_string(),
                reason: "must have format 'did:<method>:<identifier>'".to_string(),
            }
            .into());
        }

        let method = parts[0];
        let identifier = parts[1];

        // Method must be non-empty lowercase alphanumeric
        if method.is_empty() || !method.chars().all(|c| c.is_ascii_lowercase()) {
            return Err(InvalidInputError::Did {
                value: s.to_string(),
                reason: "method must be non-empty lowercase letters".to_string(),
            }
            .into());
        }

        // Identifier must be non-empty
        if identifier.is_empty() {
            return Err(InvalidInputError::Did {
                value: s.to_string(),
                reason: "identifier must be non-empty".to_string(),
            }
            .into());
        }

        Ok(())
    }
}

impl fmt::Display for Did {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for Did {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s)
    }
}

impl TryFrom<String> for Did {
    type Error = Error;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        Self::new(s)
    }
}

impl From<Did> for String {
    fn from(did: Did) -> Self {
        did.0
    }
}

impl AsRef<str> for Did {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_plc_did() {
        let did = Did::new("did:plc:z72i7hdynmk6r22z27h6tvur").unwrap();
        assert_eq!(did.method(), "plc");
        assert_eq!(did.identifier(), "z72i7hdynmk6r22z27h6tvur");
    }

    #[test]
    fn valid_web_did() {
        let did = Did::new("did:web:example.com").unwrap();
        assert_eq!(did.method(), "web");
        assert_eq!(did.identifier(), "example.com");
    }

    #[test]
    fn invalid_missing_prefix() {
        assert!(Did::new("plc:z72i7hdynmk6r22z27h6tvur").is_err());
    }

    #[test]
    fn invalid_missing_identifier() {
        assert!(Did::new("did:plc:").is_err());
    }

    #[test]
    fn invalid_missing_method() {
        assert!(Did::new("did::identifier").is_err());
    }
}
