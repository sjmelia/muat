//! CLI integration tests against a real PDS.
//!
//! These tests are opt-in and require environment variables to be set:
//! - ATPROTO_TEST_IDENTIFIER: Test account handle or DID
//! - ATPROTO_TEST_PASSWORD: Test account app password
//!
//! Tests are skipped if these variables are not set.
//!
//! All test records use the `org.muat.test.record` namespace to avoid
//! polluting real collections.

mod common;

use common::{
    TEST_COLLECTION, cleanup_test_records, get_test_credentials, run_cli, run_cli_success,
};

#[test]
fn test_login() {
    let Some((identifier, password)) = get_test_credentials() else {
        eprintln!("Skipping test_login: ATPROTO_TEST_IDENTIFIER/PASSWORD not set");
        return;
    };

    let output = run_cli(&[
        "pds",
        "login",
        "--identifier",
        &identifier,
        "--password",
        &password,
    ]);

    assert!(
        output.status.success(),
        "Login failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Logged in successfully") || stdout.contains("✓"));
}

#[test]
fn test_whoami() {
    let Some((identifier, password)) = get_test_credentials() else {
        eprintln!("Skipping test_whoami: credentials not set");
        return;
    };

    // Ensure logged in
    run_cli(&[
        "pds",
        "login",
        "--identifier",
        &identifier,
        "--password",
        &password,
    ]);

    let stdout = run_cli_success(&["pds", "whoami"]);
    assert!(stdout.contains("DID:") || stdout.contains("did:"));
}

#[test]
fn test_refresh_token() {
    let Some((identifier, password)) = get_test_credentials() else {
        eprintln!("Skipping test_refresh_token: credentials not set");
        return;
    };

    // Ensure logged in
    run_cli(&[
        "pds",
        "login",
        "--identifier",
        &identifier,
        "--password",
        &password,
    ]);

    let output = run_cli(&["pds", "refresh-token"]);
    assert!(
        output.status.success(),
        "Refresh failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn test_record_lifecycle() {
    let Some((identifier, password)) = get_test_credentials() else {
        eprintln!("Skipping test_record_lifecycle: credentials not set");
        return;
    };

    // Ensure logged in
    run_cli(&[
        "pds",
        "login",
        "--identifier",
        &identifier,
        "--password",
        &password,
    ]);

    // Cleanup any existing test records
    cleanup_test_records();

    // List records (should be empty or have no test records)
    let stdout = run_cli_success(&["pds", "list-records", TEST_COLLECTION]);
    let initial_count = stdout.lines().filter(|l| !l.is_empty()).count();

    // Note: Creating records on a real Bluesky PDS requires a valid lexicon schema,
    // which org.muat.test.record doesn't have. For full record lifecycle testing,
    // see test_file_pds_record_lifecycle which uses a local file-based PDS.

    // List records again
    let stdout = run_cli_success(&["pds", "list-records", TEST_COLLECTION]);
    let final_count = stdout.lines().filter(|l| !l.is_empty()).count();

    // Should be same count (no records created)
    assert_eq!(initial_count, final_count);
}
