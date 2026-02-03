//! Token types for AT Protocol authentication.

use std::fmt;

/// An access token for authenticated XRPC requests.
///
/// Access tokens are short-lived JWTs used to authenticate requests to the PDS.
///
/// # Security
///
/// - Never logged or displayed in Debug output
/// - Treat as opaque; do not parse or inspect
#[derive(Clone)]
pub struct AccessToken(pub(crate) String);

impl AccessToken {
    /// Create a new access token.
    pub(crate) fn new(token: impl Into<String>) -> Self {
        Self(token.into())
    }

    /// Returns the token value for use in authorization headers.
    ///
    /// # Security
    ///
    /// Use only when constructing HTTP authorization headers.
    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

// Hide token value in Debug output
impl fmt::Debug for AccessToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("AccessToken").field(&"[REDACTED]").finish()
    }
}

/// A refresh token for obtaining new access tokens.
///
/// Refresh tokens are longer-lived and used to obtain new access tokens
/// without requiring re-authentication.
///
/// # Security
///
/// - Never logged or displayed in Debug output
/// - Treat as opaque; do not parse or inspect
#[derive(Clone)]
pub struct RefreshToken(pub(crate) String);

impl RefreshToken {
    /// Create a new refresh token.
    pub(crate) fn new(token: impl Into<String>) -> Self {
        Self(token.into())
    }

    /// Returns the token value for use in refresh requests.
    ///
    /// # Security
    ///
    /// Use only when constructing token refresh requests.
    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

// Hide token value in Debug output
impl fmt::Debug for RefreshToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("RefreshToken").field(&"[REDACTED]").finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn access_token_hides_value_in_debug() {
        let token = AccessToken::new("eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...");
        let debug = format!("{:?}", token);
        assert!(!debug.contains("eyJ"));
        assert!(debug.contains("[REDACTED]"));
    }

    #[test]
    fn refresh_token_hides_value_in_debug() {
        let token = RefreshToken::new("refresh_token_value_here");
        let debug = format!("{:?}", token);
        assert!(!debug.contains("refresh_token"));
        assert!(debug.contains("[REDACTED]"));
    }
}
