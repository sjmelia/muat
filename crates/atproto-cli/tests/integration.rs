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
use tempfile::TempDir;

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
        if let Ok(record) = serde_json::from_str::<serde_json::Value>(line)
            && let Some(uri) = record["uri"].as_str()
        {
            let _ = run_cli(&["pds", "delete-record", uri]);
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

// ============================================================================
// File-based PDS tests (no external credentials required)
// ============================================================================

/// Run the CLI with a custom HOME directory for isolated session storage.
fn run_cli_with_env(args: &[&str], home: &std::path::Path, pds_url: &str) -> Output {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_atproto"));
    cmd.args(args);
    cmd.env("HOME", home);
    cmd.env("XDG_DATA_HOME", home.join("data"));
    // Set PDS URL via environment if needed for commands that default to bsky.social
    if !args.contains(&"--pds") {
        cmd.env("ATPROTO_PDS", pds_url);
    }
    cmd.output().expect("Failed to execute CLI")
}

/// Run the CLI with a custom HOME and expect success.
fn run_cli_with_env_success(args: &[&str], home: &std::path::Path, pds_url: &str) -> String {
    let output = run_cli_with_env(args, home, pds_url);
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        panic!("CLI command failed: {:?}\nstderr: {}", args, stderr);
    }
    String::from_utf8_lossy(&output.stdout).to_string()
}

#[test]
fn test_file_pds_create_account() {
    let temp_dir = TempDir::new().unwrap();
    let pds_path = temp_dir.path().join("pds");
    let pds_url = format!("file://{}", pds_path.display());
    let home = temp_dir.path().join("home");
    std::fs::create_dir_all(&home).unwrap();
    let password = "test-password";

    // Create an account
    let output = run_cli_with_env(
        &[
            "pds",
            "create-account",
            "--pds",
            &pds_url,
            "--password",
            password,
            "alice.local",
        ],
        &home,
        &pds_url,
    );

    assert!(
        output.status.success(),
        "Create account failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("did:plc:"));
    assert!(stdout.contains("alice.local"));
}

#[test]
fn test_file_pds_login() {
    let temp_dir = TempDir::new().unwrap();
    let pds_path = temp_dir.path().join("pds");
    let pds_url = format!("file://{}", pds_path.display());
    let home = temp_dir.path().join("home");
    std::fs::create_dir_all(&home).unwrap();
    let password = "test-password";

    // Create an account first
    run_cli_with_env_success(
        &[
            "pds",
            "create-account",
            "--pds",
            &pds_url,
            "--password",
            password,
            "bob.local",
        ],
        &home,
        &pds_url,
    );

    // Now login
    let output = run_cli_with_env(
        &[
            "pds",
            "login",
            "--pds",
            &pds_url,
            "--identifier",
            "bob.local",
            "--password",
            password,
        ],
        &home,
        &pds_url,
    );

    assert!(
        output.status.success(),
        "Login failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Logged in successfully") || stdout.contains("✓"));
}

#[test]
fn test_file_pds_login_nonexistent_account() {
    let temp_dir = TempDir::new().unwrap();
    let pds_path = temp_dir.path().join("pds");
    let pds_url = format!("file://{}", pds_path.display());
    let home = temp_dir.path().join("home");
    std::fs::create_dir_all(&home).unwrap();
    std::fs::create_dir_all(&pds_path).unwrap();

    // Try to login without creating an account first
    let output = run_cli_with_env(
        &[
            "pds",
            "login",
            "--pds",
            &pds_url,
            "--identifier",
            "nonexistent.local",
            "--password",
            "ignored",
        ],
        &home,
        &pds_url,
    );

    assert!(
        !output.status.success(),
        "Login should have failed for nonexistent account"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("not found") || stderr.contains("invalid credentials"),
        "Expected 'not found' or 'invalid credentials' error, got: {}",
        stderr
    );
}

#[test]
fn test_file_pds_record_lifecycle() {
    let temp_dir = TempDir::new().unwrap();
    let pds_path = temp_dir.path().join("pds");
    let pds_url = format!("file://{}", pds_path.display());
    let home = temp_dir.path().join("home");
    std::fs::create_dir_all(&home).unwrap();
    let password = "test-password";

    // Create an account
    run_cli_with_env_success(
        &[
            "pds",
            "create-account",
            "--pds",
            &pds_url,
            "--password",
            password,
            "charlie.local",
        ],
        &home,
        &pds_url,
    );

    // Login
    run_cli_with_env_success(
        &[
            "pds",
            "login",
            "--pds",
            &pds_url,
            "--identifier",
            "charlie.local",
            "--password",
            password,
        ],
        &home,
        &pds_url,
    );

    // List records (should be empty initially)
    let stdout =
        run_cli_with_env_success(&["pds", "list-records", TEST_COLLECTION], &home, &pds_url);
    // Count only JSON lines (actual records), not messages like "No records found."
    let initial_count = stdout.lines().filter(|l| l.starts_with('{')).count();
    assert_eq!(initial_count, 0, "Expected no records initially");

    // Create a record using stdin
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_atproto"));
    cmd.args([
        "pds",
        "create-record",
        TEST_COLLECTION,
        "--type",
        TEST_COLLECTION,
        "--json",
        "-",
    ]);
    cmd.env("HOME", &home);
    cmd.env("XDG_DATA_HOME", home.join("data"));
    cmd.stdin(std::process::Stdio::piped());
    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::piped());

    let mut child = cmd.spawn().expect("Failed to spawn CLI");
    {
        use std::io::Write;
        let stdin = child.stdin.as_mut().expect("Failed to open stdin");
        stdin
            .write_all(b"{\"text\": \"test message\"}")
            .expect("Failed to write to stdin");
    }
    let output = child.wait_with_output().expect("Failed to wait for CLI");

    assert!(
        output.status.success(),
        "Create record failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("at://"), "Expected AT URI in output");

    // Extract the AT URI from the output
    let uri = stdout
        .lines()
        .find(|line| line.starts_with("at://"))
        .expect("Could not find AT URI in output")
        .trim();

    // List records (should have one record now)
    let stdout =
        run_cli_with_env_success(&["pds", "list-records", TEST_COLLECTION], &home, &pds_url);
    // Count only JSON lines (actual records)
    let count = stdout.lines().filter(|l| l.starts_with('{')).count();
    assert_eq!(count, 1, "Expected 1 record after creation");

    // Get the record
    let stdout = run_cli_with_env_success(&["pds", "get-record", uri], &home, &pds_url);
    assert!(
        stdout.contains("test message"),
        "Record should contain our text"
    );

    // Delete the record
    let output = run_cli_with_env(&["pds", "delete-record", uri], &home, &pds_url);
    assert!(
        output.status.success(),
        "Delete record failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // List records (should be empty again)
    let stdout =
        run_cli_with_env_success(&["pds", "list-records", TEST_COLLECTION], &home, &pds_url);
    // Count only JSON lines (actual records)
    let final_count = stdout.lines().filter(|l| l.starts_with('{')).count();
    assert_eq!(final_count, 0, "Expected no records after deletion");
}

#[test]
fn test_file_pds_whoami() {
    let temp_dir = TempDir::new().unwrap();
    let pds_path = temp_dir.path().join("pds");
    let pds_url = format!("file://{}", pds_path.display());
    let home = temp_dir.path().join("home");
    std::fs::create_dir_all(&home).unwrap();
    let password = "test-password";

    // Create and login
    run_cli_with_env_success(
        &[
            "pds",
            "create-account",
            "--pds",
            &pds_url,
            "--password",
            password,
            "dave.local",
        ],
        &home,
        &pds_url,
    );
    run_cli_with_env_success(
        &[
            "pds",
            "login",
            "--pds",
            &pds_url,
            "--identifier",
            "dave.local",
            "--password",
            password,
        ],
        &home,
        &pds_url,
    );

    // Whoami
    let stdout = run_cli_with_env_success(&["pds", "whoami"], &home, &pds_url);
    assert!(stdout.contains("did:"), "Expected DID in whoami output");
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
