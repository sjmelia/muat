//! Mock PDS tests for the muat library.
//!
//! These tests use wiremock to simulate a PDS server and test the library's
//! behavior without requiring network access or real credentials.

use muat::{Credentials, Nsid, PdsUrl, Session};
use serde_json::json;
use wiremock::matchers::{body_json, header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Helper to create a PDS URL from a mock server.
fn mock_pds_url(server: &MockServer) -> PdsUrl {
    // For tests, we need to allow HTTP localhost
    PdsUrl::new(&format!("http://127.0.0.1:{}", server.address().port())).unwrap()
}

// ============================================================================
// Authentication Tests
// ============================================================================

#[tokio::test]
async fn test_login_success() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/xrpc/com.atproto.server.createSession"))
        .and(body_json(json!({
            "identifier": "alice.test",
            "password": "secret123"
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "did": "did:plc:test123",
            "handle": "alice.test",
            "accessJwt": "test-access-token",
            "refreshJwt": "test-refresh-token"
        })))
        .mount(&server)
        .await;

    let pds = mock_pds_url(&server);
    let credentials = Credentials::new("alice.test", "secret123");
    let session = Session::login(&pds, credentials).await.unwrap();

    assert_eq!(session.did().as_str(), "did:plc:test123");
}

#[tokio::test]
async fn test_login_invalid_credentials() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/xrpc/com.atproto.server.createSession"))
        .respond_with(ResponseTemplate::new(401).set_body_json(json!({
            "error": "AuthenticationRequired",
            "message": "Invalid identifier or password"
        })))
        .mount(&server)
        .await;

    let pds = mock_pds_url(&server);
    let credentials = Credentials::new("bad@user", "wrongpass");
    let result = Session::login(&pds, credentials).await;

    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("401"));
}

#[tokio::test]
async fn test_session_refresh_success() {
    let server = MockServer::start().await;

    // First, login
    Mock::given(method("POST"))
        .and(path("/xrpc/com.atproto.server.createSession"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "did": "did:plc:test123",
            "handle": "alice.test",
            "accessJwt": "old-access-token",
            "refreshJwt": "old-refresh-token"
        })))
        .mount(&server)
        .await;

    // Then, refresh
    Mock::given(method("POST"))
        .and(path("/xrpc/com.atproto.server.refreshSession"))
        .and(header("authorization", "Bearer old-refresh-token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "did": "did:plc:test123",
            "handle": "alice.test",
            "accessJwt": "new-access-token",
            "refreshJwt": "new-refresh-token"
        })))
        .mount(&server)
        .await;

    let pds = mock_pds_url(&server);
    let credentials = Credentials::new("alice.test", "secret");
    let session = Session::login(&pds, credentials).await.unwrap();

    // Refresh should succeed
    session.refresh().await.unwrap();

    // Verify the new token is used (by checking export)
    let new_token = session.export_access_token().await;
    assert_eq!(new_token, "new-access-token");
}

#[tokio::test]
async fn test_session_refresh_expired_token() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/xrpc/com.atproto.server.createSession"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "did": "did:plc:test123",
            "handle": "alice.test",
            "accessJwt": "access-token",
            "refreshJwt": "expired-refresh-token"
        })))
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/xrpc/com.atproto.server.refreshSession"))
        .respond_with(ResponseTemplate::new(400).set_body_json(json!({
            "error": "ExpiredToken",
            "message": "Token has expired"
        })))
        .mount(&server)
        .await;

    let pds = mock_pds_url(&server);
    let credentials = Credentials::new("alice.test", "secret");
    let session = Session::login(&pds, credentials).await.unwrap();

    let result = session.refresh().await;
    assert!(result.is_err());
}

// ============================================================================
// Repository Operation Tests
// ============================================================================

#[tokio::test]
async fn test_list_records_success() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/xrpc/com.atproto.server.createSession"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "did": "did:plc:test123",
            "handle": "alice.test",
            "accessJwt": "access-token",
            "refreshJwt": "refresh-token"
        })))
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path("/xrpc/com.atproto.repo.listRecords"))
        .and(header("authorization", "Bearer access-token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "records": [
                {
                    "uri": "at://did:plc:test123/org.test.record/abc123",
                    "cid": "bafytest1",
                    "value": {"text": "Hello, world!"}
                },
                {
                    "uri": "at://did:plc:test123/org.test.record/def456",
                    "cid": "bafytest2",
                    "value": {"text": "Another record"}
                }
            ],
            "cursor": "next-page-cursor"
        })))
        .mount(&server)
        .await;

    let pds = mock_pds_url(&server);
    let session = Session::login(&pds, Credentials::new("alice.test", "secret"))
        .await
        .unwrap();

    let collection = Nsid::new("org.test.record").unwrap();
    let result = session
        .list_records(session.did(), &collection, None, None)
        .await
        .unwrap();

    assert_eq!(result.records.len(), 2);
    assert_eq!(result.cursor, Some("next-page-cursor".to_string()));
    assert_eq!(
        result.records[0].value["text"].as_str().unwrap(),
        "Hello, world!"
    );
}

#[tokio::test]
async fn test_list_records_empty() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/xrpc/com.atproto.server.createSession"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "did": "did:plc:test123",
            "handle": "alice.test",
            "accessJwt": "access-token",
            "refreshJwt": "refresh-token"
        })))
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path("/xrpc/com.atproto.repo.listRecords"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "records": []
        })))
        .mount(&server)
        .await;

    let pds = mock_pds_url(&server);
    let session = Session::login(&pds, Credentials::new("alice.test", "secret"))
        .await
        .unwrap();

    let collection = Nsid::new("org.empty.collection").unwrap();
    let result = session
        .list_records(session.did(), &collection, None, None)
        .await
        .unwrap();

    assert!(result.records.is_empty());
    assert!(result.cursor.is_none());
}

#[tokio::test]
async fn test_create_record_success() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/xrpc/com.atproto.server.createSession"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "did": "did:plc:test123",
            "handle": "alice.test",
            "accessJwt": "access-token",
            "refreshJwt": "refresh-token"
        })))
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/xrpc/com.atproto.repo.createRecord"))
        .and(header("authorization", "Bearer access-token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "uri": "at://did:plc:test123/org.test.record/newrecord123",
            "cid": "bafynewrecord"
        })))
        .mount(&server)
        .await;

    let pds = mock_pds_url(&server);
    let session = Session::login(&pds, Credentials::new("alice.test", "secret"))
        .await
        .unwrap();

    let collection = Nsid::new("org.test.record").unwrap();
    let value = json!({"text": "New test record"});
    let uri = session.create_record_raw(&collection, &value).await.unwrap();

    assert_eq!(uri.rkey().as_str(), "newrecord123");
}

#[tokio::test]
async fn test_delete_record_success() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/xrpc/com.atproto.server.createSession"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "did": "did:plc:test123",
            "handle": "alice.test",
            "accessJwt": "access-token",
            "refreshJwt": "refresh-token"
        })))
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/xrpc/com.atproto.repo.deleteRecord"))
        .and(header("authorization", "Bearer access-token"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;

    let pds = mock_pds_url(&server);
    let session = Session::login(&pds, Credentials::new("alice.test", "secret"))
        .await
        .unwrap();

    let uri = muat::AtUri::new("at://did:plc:test123/org.test.record/todelete").unwrap();
    let result = session.delete_record(&uri).await;

    assert!(result.is_ok());
}

// ============================================================================
// Error Handling Tests
// ============================================================================

#[tokio::test]
async fn test_non_json_error_response() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/xrpc/com.atproto.server.createSession"))
        .respond_with(
            ResponseTemplate::new(500)
                .set_body_string("Internal Server Error")
                .insert_header("content-type", "text/plain"),
        )
        .mount(&server)
        .await;

    let pds = mock_pds_url(&server);
    let credentials = Credentials::new("alice.test", "secret");
    let result = Session::login(&pds, credentials).await;

    assert!(result.is_err());
    // Should handle non-JSON error gracefully
    let err = result.unwrap_err().to_string();
    assert!(err.contains("500"));
}

#[tokio::test]
async fn test_empty_error_response() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/xrpc/com.atproto.server.createSession"))
        .respond_with(ResponseTemplate::new(503))
        .mount(&server)
        .await;

    let pds = mock_pds_url(&server);
    let credentials = Credentials::new("alice.test", "secret");
    let result = Session::login(&pds, credentials).await;

    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("503"));
}
