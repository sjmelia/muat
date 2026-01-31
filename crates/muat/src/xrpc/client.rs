//! XRPC HTTP client implementation.

use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use serde::{de::DeserializeOwned, Serialize};
use tracing::{debug, instrument, trace};

use crate::error::{Error, ProtocolError};
use crate::types::PdsUrl;

use super::endpoints::XrpcErrorResponse;

/// HTTP client for XRPC requests.
#[derive(Debug, Clone)]
pub struct XrpcClient {
    client: reqwest::Client,
    pds: PdsUrl,
}

impl XrpcClient {
    /// Create a new XRPC client for the given PDS.
    pub fn new(pds: PdsUrl) -> Self {
        let client = reqwest::Client::builder()
            .user_agent(concat!("muat/", env!("CARGO_PKG_VERSION")))
            .build()
            .expect("failed to build HTTP client");

        Self { client, pds }
    }

    /// Returns the PDS URL this client is configured for.
    #[allow(dead_code)]
    pub fn pds(&self) -> &PdsUrl {
        &self.pds
    }

    /// Make an unauthenticated XRPC query (GET request).
    #[allow(dead_code)]
    #[instrument(skip(self), fields(pds = %self.pds))]
    pub async fn query<Q, R>(&self, method: &str, params: &Q) -> Result<R, Error>
    where
        Q: Serialize + std::fmt::Debug,
        R: DeserializeOwned,
    {
        let url = self.pds.xrpc_url(method);
        debug!(method, "XRPC query");
        trace!(?params, "query parameters");

        let response = self.client.get(&url).query(params).send().await?;

        self.handle_response(response).await
    }

    /// Make an authenticated XRPC query (GET request).
    #[instrument(skip(self, token), fields(pds = %self.pds))]
    pub async fn query_authed<Q, R>(
        &self,
        method: &str,
        params: &Q,
        token: &str,
    ) -> Result<R, Error>
    where
        Q: Serialize + std::fmt::Debug,
        R: DeserializeOwned,
    {
        let url = self.pds.xrpc_url(method);
        debug!(method, "XRPC authenticated query");
        trace!(?params, "query parameters");

        let response = self
            .client
            .get(&url)
            .query(params)
            .headers(self.auth_headers(token))
            .send()
            .await?;

        self.handle_response(response).await
    }

    /// Make an unauthenticated XRPC procedure (POST request).
    #[instrument(skip(self), fields(pds = %self.pds))]
    pub async fn procedure<B, R>(&self, method: &str, body: &B) -> Result<R, Error>
    where
        B: Serialize + std::fmt::Debug,
        R: DeserializeOwned,
    {
        let url = self.pds.xrpc_url(method);
        debug!(method, %url, "XRPC procedure");

        let response = self.client.post(&url).json(body).send().await?;

        self.handle_response(response).await
    }

    /// Make an authenticated XRPC procedure (POST request).
    #[instrument(skip(self, token), fields(pds = %self.pds))]
    pub async fn procedure_authed<B, R>(
        &self,
        method: &str,
        body: &B,
        token: &str,
    ) -> Result<R, Error>
    where
        B: Serialize + std::fmt::Debug,
        R: DeserializeOwned,
    {
        let url = self.pds.xrpc_url(method);
        debug!(method, "XRPC authenticated procedure");

        let response = self
            .client
            .post(&url)
            .json(body)
            .headers(self.auth_headers(token))
            .send()
            .await?;

        self.handle_response(response).await
    }

    /// Make an authenticated XRPC procedure that returns no content.
    #[instrument(skip(self, token), fields(pds = %self.pds))]
    pub async fn procedure_authed_no_response<B>(
        &self,
        method: &str,
        body: &B,
        token: &str,
    ) -> Result<(), Error>
    where
        B: Serialize + std::fmt::Debug,
    {
        let url = self.pds.xrpc_url(method);
        debug!(method, "XRPC authenticated procedure (no response)");

        let response = self
            .client
            .post(&url)
            .json(body)
            .headers(self.auth_headers(token))
            .send()
            .await?;

        let status = response.status();
        if status.is_success() {
            Ok(())
        } else {
            let error = self.parse_error_response(response).await;
            Err(Error::Protocol(error))
        }
    }

    /// Make an authenticated XRPC procedure with no request body.
    /// Used for endpoints like refreshSession that don't accept a body.
    #[instrument(skip(self, token), fields(pds = %self.pds))]
    pub async fn procedure_authed_no_body<R>(
        &self,
        method: &str,
        token: &str,
    ) -> Result<R, Error>
    where
        R: DeserializeOwned,
    {
        let url = self.pds.xrpc_url(method);
        debug!(method, "XRPC authenticated procedure (no body)");

        let response = self
            .client
            .post(&url)
            .header(AUTHORIZATION, format!("Bearer {}", token))
            .send()
            .await?;

        self.handle_response(response).await
    }

    /// Create authorization headers for authenticated requests.
    fn auth_headers(&self, token: &str) -> HeaderMap {
        let mut headers = HeaderMap::new();
        let auth_value = format!("Bearer {}", token);
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&auth_value).expect("invalid token characters"),
        );
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        headers
    }

    /// Handle an XRPC response, parsing the body or error.
    async fn handle_response<R: DeserializeOwned>(
        &self,
        response: reqwest::Response,
    ) -> Result<R, Error> {
        let status = response.status();
        trace!(status = %status, "XRPC response");

        if status.is_success() {
            let body = response.json::<R>().await?;
            Ok(body)
        } else {
            let error = self.parse_error_response(response).await;
            Err(Error::Protocol(error))
        }
    }

    /// Parse an XRPC error response.
    async fn parse_error_response(&self, response: reqwest::Response) -> ProtocolError {
        let status = response.status().as_u16();

        // Try to parse as XRPC error format
        match response.json::<XrpcErrorResponse>().await {
            Ok(error_body) => ProtocolError::new(status, error_body.error, error_body.message),
            Err(_) => ProtocolError::new(status, None, None),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn client_creation() {
        let pds = PdsUrl::new("https://bsky.social").unwrap();
        let client = XrpcClient::new(pds.clone());
        assert_eq!(client.pds().as_str(), pds.as_str());
    }
}
