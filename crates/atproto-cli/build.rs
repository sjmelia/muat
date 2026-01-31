//! Build script to capture git version information at compile time.

use std::process::Command;

fn main() {
    // Tell Cargo to rerun this if git state changes
    println!("cargo:rerun-if-changed=.git/HEAD");
    println!("cargo:rerun-if-changed=.git/refs/");

    // Try to get version from git describe
    let version = get_git_version().unwrap_or_else(|| env!("CARGO_PKG_VERSION").to_string());

    println!("cargo:rustc-env=ATPROTO_VERSION={}", version);
}

fn get_git_version() -> Option<String> {
    // First try git describe with tags
    let output = Command::new("git")
        .args(["describe", "--tags", "--always"])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let version = String::from_utf8(output.stdout).ok()?;
    let version = version.trim();

    if version.is_empty() {
        return None;
    }

    // If it starts with 'v', strip it for cleaner output
    let version = version.strip_prefix('v').unwrap_or(version);

    Some(version.to_string())
}
