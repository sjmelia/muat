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

use std::process::{Command, Output};

/// Test collection namespace - non-Bluesky to avoid pollution
const TEST_COLLECTION: &str = "org.muat.test.record";

/// Get test credentials from environment.
/// Returns None if not set, causing tests to be skipped.
fn get_test_credentials() -> Option<(String, String)> {
    let identifier = std::env::var("ATPROTO_TEST_IDENTIFIER").ok()?;
    let password = std::env::var("ATPROTO_TEST_PASSWORD").ok()?;
    Some((identifier, password))
}

/// Run the CLI binary with arguments.
fn run_cli(args: &[&str]) -> Output {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_atproto"));
    cmd.args(args);
    cmd.output().expect("Failed to execute CLI")
}

/// Run the CLI and expect success.
fn run_cli_success(args: &[&str]) -> String {
    let output = run_cli(args);
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        panic!("CLI command failed: {:?}\nstderr: {}", args, stderr);
    }
    String::from_utf8_lossy(&output.stdout).to_string()
}

/// Run the CLI and expect failure.
#[allow(dead_code)]
fn run_cli_failure(args: &[&str]) -> String {
    let output = run_cli(args);
    if output.status.success() {
        panic!("CLI command should have failed: {:?}", args);
    }
    String::from_utf8_lossy(&output.stderr).to_string()
}

/// Delete all test records (cleanup helper).
fn cleanup_test_records() {
    // List and delete any existing test records
    let output = run_cli(&["pds", "list-records", TEST_COLLECTION]);
    if !output.status.success() {
        return; // No session or collection doesn't exist
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout.lines() {
        if let Ok(record) = serde_json::from_str::<serde_json::Value>(line) {
            if let Some(uri) = record["uri"].as_str() {
                let _ = run_cli(&["pds", "delete-record", uri]);
            }
        }
    }
}

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

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Logged in successfully") || stderr.contains("✓"));
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

    // Note: Creating records requires a valid lexicon schema, which org.muat.test.record
    // doesn't have. In a real scenario, we'd need to use a real lexicon or set up
    // a test PDS that allows arbitrary records.
    //
    // For now, we just verify that list-records works.

    // List records again
    let stdout = run_cli_success(&["pds", "list-records", TEST_COLLECTION]);
    let final_count = stdout.lines().filter(|l| !l.is_empty()).count();

    // Should be same count (no records created)
    assert_eq!(initial_count, final_count);
}

#[test]
fn test_list_records_positional() {
    let Some((identifier, password)) = get_test_credentials() else {
        eprintln!("Skipping test_list_records_positional: credentials not set");
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

    // Test positional collection argument (the main point of G3)
    let output = run_cli(&["pds", "list-records", "app.bsky.feed.post"]);
    assert!(
        output.status.success(),
        "Positional list-records failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn test_no_session_error() {
    // Clear any existing session by using a temp home
    let temp_dir = tempfile::tempdir().unwrap();
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_atproto"));
    cmd.args(["pds", "whoami"]);
    cmd.env("HOME", temp_dir.path());
    cmd.env("XDG_DATA_HOME", temp_dir.path().join("data"));

    let output = cmd.output().expect("Failed to execute CLI");
    assert!(!output.status.success());

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("No active session") || stderr.contains("login"),
        "Expected 'no session' error, got: {}",
        stderr
    );
}
