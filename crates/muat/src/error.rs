//! Error types for the muat library.
//!
//! This module provides a unified error type with explicit variants for
//! transport, authentication, protocol, and input validation errors.

use std::fmt;
use thiserror::Error;

/// The unified error type for muat operations.
///
/// This error type covers all possible failure modes in the library,
/// with explicit variants to allow callers to handle specific cases.
#[derive(Debug, Error)]
pub enum Error {
    /// Network transport errors (DNS, TLS, connection, timeout).
    #[error("transport error: {0}")]
    Transport(#[from] TransportError),

    /// Authentication errors (invalid credentials, expired session).
    #[error("authentication error: {0}")]
    Auth(#[from] AuthError),

    /// Protocol errors (XRPC errors, unexpected responses).
    #[error("protocol error: {0}")]
    Protocol(#[from] ProtocolError),

    /// Input validation errors (invalid DID, NSID, URI format).
    #[error("invalid input: {0}")]
    InvalidInput(#[from] InvalidInputError),
}

/// Transport-level errors.
#[derive(Debug, Error)]
pub enum TransportError {
    /// Network connection failed.
    #[error("connection failed: {message}")]
    Connection { message: String },

    /// DNS resolution failed.
    #[error("DNS resolution failed: {host}")]
    Dns { host: String },

    /// TLS/SSL error.
    #[error("TLS error: {message}")]
    Tls { message: String },

    /// Request timed out.
    #[error("request timed out after {duration_ms}ms")]
    Timeout { duration_ms: u64 },

    /// Generic HTTP error.
    #[error("HTTP error: {message}")]
    Http { message: String },
}

impl From<reqwest::Error> for TransportError {
    fn from(err: reqwest::Error) -> Self {
        if err.is_timeout() {
            TransportError::Timeout { duration_ms: 0 }
        } else if err.is_connect() {
            TransportError::Connection {
                message: err.to_string(),
            }
        } else {
            TransportError::Http {
                message: err.to_string(),
            }
        }
    }
}

impl From<reqwest::Error> for Error {
    fn from(err: reqwest::Error) -> Self {
        Error::Transport(TransportError::from(err))
    }
}

/// Authentication-related errors.
#[derive(Debug, Error)]
pub enum AuthError {
    /// Invalid credentials provided.
    #[error("invalid credentials")]
    InvalidCredentials,

    /// Session has expired.
    #[error("session expired")]
    SessionExpired,

    /// Refresh token is invalid or expired.
    #[error("refresh token invalid")]
    RefreshTokenInvalid,

    /// Account is suspended or deactivated.
    #[error("account unavailable: {reason}")]
    AccountUnavailable { reason: String },
}

/// Protocol-level errors from XRPC responses.
#[derive(Debug)]
pub struct ProtocolError {
    /// HTTP status code.
    pub status: u16,
    /// XRPC error code (if present).
    pub error: Option<String>,
    /// Error message from the server.
    pub message: Option<String>,
}

impl fmt::Display for ProtocolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "HTTP {}", self.status)?;
        if let Some(ref error) = self.error {
            write!(f, " [{}]", error)?;
        }
        if let Some(ref message) = self.message {
            write!(f, ": {}", message)?;
        }
        Ok(())
    }
}

impl std::error::Error for ProtocolError {}

impl ProtocolError {
    /// Create a new protocol error.
    pub fn new(status: u16, error: Option<String>, message: Option<String>) -> Self {
        Self {
            status,
            error,
            message,
        }
    }

    /// Check if this is an authentication error.
    pub fn is_auth_error(&self) -> bool {
        self.status == 401
            || self.error.as_deref() == Some("AuthenticationRequired")
            || self.error.as_deref() == Some("ExpiredToken")
            || self.error.as_deref() == Some("InvalidToken")
    }
}

/// Input validation errors.
#[derive(Debug, Error)]
pub enum InvalidInputError {
    /// Invalid DID format.
    #[error("invalid DID '{value}': {reason}")]
    Did { value: String, reason: String },

    /// Invalid NSID format.
    #[error("invalid NSID '{value}': {reason}")]
    Nsid { value: String, reason: String },

    /// Invalid AT URI format.
    #[error("invalid AT URI '{value}': {reason}")]
    AtUri { value: String, reason: String },

    /// Invalid PDS URL format.
    #[error("invalid PDS URL '{value}': {reason}")]
    PdsUrl { value: String, reason: String },

    /// Invalid record key format.
    #[error("invalid rkey '{value}': {reason}")]
    Rkey { value: String, reason: String },

    /// Invalid CID format.
    #[error("invalid CID '{value}': {reason}")]
    Cid { value: String, reason: String },

    /// Generic invalid input.
    #[error("invalid input: {message}")]
    Other { message: String },
}
