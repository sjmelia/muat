use std::path::Path;
use std::process::{Command, Output};

/// Test collection namespace - non-Bluesky to avoid pollution
pub const TEST_COLLECTION: &str = "org.muat.test.record";

/// Get test credentials from environment.
/// Returns None if not set, causing tests to be skipped.
pub fn get_test_credentials() -> Option<(String, String)> {
    let identifier = std::env::var("ATPROTO_TEST_IDENTIFIER").ok()?;
    let password = std::env::var("ATPROTO_TEST_PASSWORD").ok()?;
    Some((identifier, password))
}

/// Run the CLI binary with arguments.
pub fn run_cli(args: &[&str]) -> Output {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_atproto"));
    cmd.args(args);
    cmd.output().expect("Failed to execute CLI")
}

/// Run the CLI and expect success.
pub fn run_cli_success(args: &[&str]) -> String {
    let output = run_cli(args);
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        panic!("CLI command failed: {:?}\nstderr: {}", args, stderr);
    }
    String::from_utf8_lossy(&output.stdout).to_string()
}

/// Run the CLI with a custom HOME directory for isolated session storage.
pub fn run_cli_with_env(args: &[&str], home: &Path, pds_url: &str) -> Output {
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
pub fn run_cli_with_env_success(args: &[&str], home: &Path, pds_url: &str) -> String {
    let output = run_cli_with_env(args, home, pds_url);
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        panic!("CLI command failed: {:?}\nstderr: {}", args, stderr);
    }
    String::from_utf8_lossy(&output.stdout).to_string()
}

/// Delete all test records (cleanup helper).
pub fn cleanup_test_records() {
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
