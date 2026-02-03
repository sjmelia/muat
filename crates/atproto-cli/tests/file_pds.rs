//! CLI integration tests against the file-backed PDS.

mod common;

use std::path::Path;
use std::process::Command;

use tempfile::TempDir;
use url::Url;

use common::{TEST_COLLECTION, run_cli_with_env, run_cli_with_env_success};

fn file_pds_url(path: &Path) -> String {
    Url::from_directory_path(path)
        .expect("Failed to convert path to file URL")
        .to_string()
}

#[test]
fn test_file_pds_create_account() {
    let temp_dir = TempDir::new().unwrap();
    let pds_path = temp_dir.path().join("pds");
    let pds_url = file_pds_url(&pds_path);
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
    let pds_url = file_pds_url(&pds_path);
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
    let pds_url = file_pds_url(&pds_path);
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
    let pds_url = file_pds_url(&pds_path);
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
    let pds_url = file_pds_url(&pds_path);
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
    let temp_dir = TempDir::new().unwrap();
    let pds_path = temp_dir.path().join("pds");
    let pds_url = file_pds_url(&pds_path);
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
            "erin.local",
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
            "erin.local",
            "--password",
            password,
        ],
        &home,
        &pds_url,
    );

    // Test positional collection argument
    let output = run_cli_with_env(&["pds", "list-records", TEST_COLLECTION], &home, &pds_url);
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
