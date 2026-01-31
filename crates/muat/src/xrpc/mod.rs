//! XRPC client implementation.
//!
//! This module provides the HTTP client for AT Protocol XRPC communication.

mod client;
mod endpoints;

pub(crate) use client::XrpcClient;
pub(crate) use endpoints::*;
