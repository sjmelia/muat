//! Build script to capture git version information at compile time.

use std::process::Command;

fn main() {
    // Tell Cargo to rerun this if git state changes
    println!("cargo:rerun-if-changed=.git/HEAD");
    println!("cargo:rerun-if-changed=.git/refs/");

    let version = std::env::var("MUAT_BUILD_VERSION")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .or_else(get_version_from_git)
        .unwrap_or_else(|| env!("CARGO_PKG_VERSION").to_string());

    println!("cargo:rustc-env=ATPROTO_VERSION={}", version);
}

fn get_version_from_git() -> Option<String> {
    let pkg_version = env!("CARGO_PKG_VERSION");

    let short_sha = Command::new("git")
        .args(["rev-parse", "--short=7", "HEAD"])
        .output()
        .ok()
        .and_then(|output| {
            if output.status.success() {
                String::from_utf8(output.stdout).ok()
            } else {
                None
            }
        })
        .map(|value| value.trim().to_string())?;

    Some(format!("{pkg_version}+{short_sha}"))
}
