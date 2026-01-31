//! Login credentials type.

use std::fmt;

/// Login credentials for AT Protocol authentication.
///
/// This type holds the identifier (handle or DID) and secret (password or app password)
/// required to authenticate with a PDS.
///
/// # Security
///
/// The secret is never exposed in Debug output to prevent accidental logging.
///
/// # Example
///
/// ```
/// use muat::Credentials;
///
/// let creds = Credentials::new("alice.bsky.social", "app-password-here");
/// assert_eq!(creds.identifier(), "alice.bsky.social");
/// ```
pub struct Credentials {
    identifier: String,
    password: String,
}

impl Credentials {
    /// Create new credentials.
    ///
    /// # Arguments
    ///
    /// * `identifier` - A handle (e.g., "alice.bsky.social") or DID
    /// * `password` - The account password or an app password
    pub fn new(identifier: impl Into<String>, password: impl Into<String>) -> Self {
        Self {
            identifier: identifier.into(),
            password: password.into(),
        }
    }

    /// Returns the identifier (handle or DID).
    pub fn identifier(&self) -> &str {
        &self.identifier
    }

    /// Returns the password.
    ///
    /// # Security
    ///
    /// Use this only when constructing authentication requests.
    /// Never log or display this value.
    pub(crate) fn password(&self) -> &str {
        &self.password
    }
}

// Intentionally hide password in Debug output
impl fmt::Debug for Credentials {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Credentials")
            .field("identifier", &self.identifier)
            .field("password", &"[REDACTED]")
            .finish()
    }
}

// Clone is intentionally derived to allow credentials to be reused,
// but the type is not Copy to make credential passing explicit.
impl Clone for Credentials {
    fn clone(&self) -> Self {
        Self {
            identifier: self.identifier.clone(),
            password: self.password.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn credentials_hides_password_in_debug() {
        let creds = Credentials::new("alice.bsky.social", "secret123");
        let debug = format!("{:?}", creds);
        assert!(debug.contains("alice.bsky.social"));
        assert!(!debug.contains("secret123"));
        assert!(debug.contains("[REDACTED]"));
    }
}
