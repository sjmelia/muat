//! Record Key (rkey) type.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

use crate::error::{Error, InvalidInputError};

/// A validated AT Protocol Record Key (rkey).
///
/// Record keys identify individual records within a collection.
/// They can be TIDs (timestamp identifiers) or other valid key formats.
///
/// # Example
///
/// ```
/// use muat::Rkey;
///
/// let rkey = Rkey::new("3jui7kd54zh2y").unwrap();
/// assert_eq!(rkey.as_str(), "3jui7kd54zh2y");
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Rkey(String);

impl Rkey {
    /// Create a new rkey from a string, validating the format.
    ///
    /// # Errors
    ///
    /// Returns an error if the string is not a valid rkey format.
    pub fn new(s: impl Into<String>) -> Result<Self, Error> {
        let s = s.into();
        Self::validate(&s)?;
        Ok(Self(s))
    }

    /// Returns the rkey string.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    fn validate(s: &str) -> Result<(), Error> {
        // rkey validation per AT Protocol spec
        // - 1-512 characters
        // - Can contain: a-z, A-Z, 0-9, ., -, _, ~
        // - Cannot be "." or ".."

        if s.is_empty() {
            return Err(InvalidInputError::Rkey {
                value: s.to_string(),
                reason: "cannot be empty".to_string(),
            }
            .into());
        }

        if s.len() > 512 {
            return Err(InvalidInputError::Rkey {
                value: s.to_string(),
                reason: "exceeds maximum length of 512 characters".to_string(),
            }
            .into());
        }

        if s == "." || s == ".." {
            return Err(InvalidInputError::Rkey {
                value: s.to_string(),
                reason: "cannot be '.' or '..'".to_string(),
            }
            .into());
        }

        for c in s.chars() {
            if !c.is_ascii_alphanumeric() && c != '.' && c != '-' && c != '_' && c != '~' {
                return Err(InvalidInputError::Rkey {
                    value: s.to_string(),
                    reason: format!("contains invalid character '{}'", c),
                }
                .into());
            }
        }

        Ok(())
    }
}

impl fmt::Display for Rkey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for Rkey {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s)
    }
}

impl TryFrom<String> for Rkey {
    type Error = Error;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        Self::new(s)
    }
}

impl From<Rkey> for String {
    fn from(rkey: Rkey) -> Self {
        rkey.0
    }
}

impl AsRef<str> for Rkey {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_tid_rkey() {
        let rkey = Rkey::new("3jui7kd54zh2y").unwrap();
        assert_eq!(rkey.as_str(), "3jui7kd54zh2y");
    }

    #[test]
    fn valid_self_rkey() {
        let rkey = Rkey::new("self").unwrap();
        assert_eq!(rkey.as_str(), "self");
    }

    #[test]
    fn invalid_empty() {
        assert!(Rkey::new("").is_err());
    }

    #[test]
    fn invalid_dot() {
        assert!(Rkey::new(".").is_err());
    }

    #[test]
    fn invalid_double_dot() {
        assert!(Rkey::new("..").is_err());
    }

    #[test]
    fn invalid_character() {
        assert!(Rkey::new("test/key").is_err());
    }
}
