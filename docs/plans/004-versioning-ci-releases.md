# Implementation Plan: PRD-004 Versioning, CI Builds, and Releases

## Overview

This plan details the implementation of PRD-004, which covers version reporting in the `atproto` binary, CI builds from `main`, and tagged releases for Linux and Windows platforms.

## Goals Summary

| Goal | Description |
|------|-------------|
| G1 | `atproto --version` with semantic version + commit SHA |
| G2 | CI builds from `main` (Linux + Windows, opt-in toggle) |
| G3 | Tagged releases with binaries and checksums |
| G4 | README documentation for CI/release process |

---

## G1: Version Reporting

### Current State

The CLI already has `#[command(version)]` in clap which uses the Cargo.toml version. We need to enhance this to include git commit information.

### Implementation

1. **Add build script** to capture git information at compile time

**File:** `crates/atproto-cli/build.rs`

```rust
use std::process::Command;

fn main() {
    // Get git describe output (tag + commits + sha)
    let version = Command::new("git")
        .args(["describe", "--tags", "--always", "--dirty"])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| env!("CARGO_PKG_VERSION").to_string());

    println!("cargo:rustc-env=ATPROTO_VERSION={}", version);
    println!("cargo:rerun-if-changed=.git/HEAD");
    println!("cargo:rerun-if-changed=.git/refs/");
}
```

2. **Update CLI version string**

**File:** `crates/atproto-cli/src/cli.rs`

```rust
#[command(version = env!("ATPROTO_VERSION"))]
pub struct Cli { ... }
```

### Version Output Format

- Tagged release: `atproto 0.2.0`
- Main build: `atproto 0.2.0+abc1234` or `atproto v0.2.0-5-gabc1234`
- No tags: `atproto 0.0.0+abc1234`

---

## G2: Builds from `main`

### New Workflow

**File:** `.github/workflows/build.yml`

This workflow:
- Triggers on pushes to `main` (when enabled)
- Builds release binaries for Linux x86_64 and Windows x86_64
- Uploads as CI artifacts
- Disabled by default via workflow input toggle

### Implementation

```yaml
name: Build

on:
  push:
    branches: [main]
  workflow_dispatch:
    inputs:
      build_main:
        description: 'Build artifacts from main'
        required: false
        default: 'false'
        type: boolean

jobs:
  build:
    if: github.event_name == 'workflow_dispatch' && inputs.build_main == 'true'
    strategy:
      matrix:
        include:
          - os: ubuntu-latest
            target: x86_64-unknown-linux-gnu
            artifact: atproto-linux-x86_64
          - os: windows-latest
            target: x86_64-pc-windows-msvc
            artifact: atproto-windows-x86_64.exe
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v4
        with:
          fetch-depth: 0  # For git describe
      - uses: dtolnay/rust-toolchain@stable
        with:
          targets: ${{ matrix.target }}
      - name: Build release binary
        run: cargo build --release --package atproto-cli
      - name: Upload artifact
        uses: actions/upload-artifact@v4
        with:
          name: ${{ matrix.artifact }}
          path: target/release/atproto${{ matrix.os == 'windows-latest' && '.exe' || '' }}
```

---

## G3: Tagged Releases

### New Workflow

**File:** `.github/workflows/release.yml`

This workflow:
- Triggers on tags matching `v*.*.*`
- Builds release binaries for Linux and Windows
- Creates GitHub Release
- Attaches binaries and checksums
- Generates release notes automatically

### Implementation

```yaml
name: Release

on:
  push:
    tags:
      - 'v*.*.*'

permissions:
  contents: write

jobs:
  build:
    strategy:
      matrix:
        include:
          - os: ubuntu-latest
            target: x86_64-unknown-linux-gnu
            artifact: atproto-linux-x86_64
          - os: windows-latest
            target: x86_64-pc-windows-msvc
            artifact: atproto-windows-x86_64.exe
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v4
        with:
          fetch-depth: 0
      - uses: dtolnay/rust-toolchain@stable
        with:
          targets: ${{ matrix.target }}
      - name: Build release binary
        run: cargo build --release --package atproto-cli
      - name: Prepare artifact (Linux)
        if: matrix.os == 'ubuntu-latest'
        run: |
          cp target/release/atproto ${{ matrix.artifact }}
          sha256sum ${{ matrix.artifact }} > ${{ matrix.artifact }}.sha256
      - name: Prepare artifact (Windows)
        if: matrix.os == 'windows-latest'
        run: |
          copy target\release\atproto.exe ${{ matrix.artifact }}
          certutil -hashfile ${{ matrix.artifact }} SHA256 > ${{ matrix.artifact }}.sha256
      - name: Upload artifact
        uses: actions/upload-artifact@v4
        with:
          name: ${{ matrix.artifact }}
          path: |
            ${{ matrix.artifact }}
            ${{ matrix.artifact }}.sha256

  release:
    needs: build
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Download all artifacts
        uses: actions/download-artifact@v4
        with:
          path: artifacts
      - name: Create Release
        uses: softprops/action-gh-release@v2
        with:
          generate_release_notes: true
          files: |
            artifacts/**/*
```

---

## G4: README Documentation

### Updates to README.md

Add a new section documenting:

1. **CI Pipeline** - fmt, clippy, unit tests, integration tests
2. **Builds from `main`** - how to enable, artifact locations
3. **Creating Releases** - tag format, automated process
4. **Supported Platforms** - Linux x86_64, Windows x86_64

---

## Files to Create/Modify

| File | Action | Description |
|------|--------|-------------|
| `crates/atproto-cli/build.rs` | Create | Build script for version info |
| `crates/atproto-cli/Cargo.toml` | Modify | Add build script reference |
| `crates/atproto-cli/src/cli.rs` | Modify | Use custom version string |
| `.github/workflows/build.yml` | Create | Main branch builds |
| `.github/workflows/release.yml` | Create | Tagged release workflow |
| `README.md` | Modify | Add CI/release documentation |

---

## Implementation Order

1. **G1** - Version reporting (enables proper versioning in G2/G3)
2. **G3** - Tagged releases (most important for production use)
3. **G2** - Main builds (optional, disabled by default)
4. **G4** - README documentation

---

## Success Criteria

- [ ] `atproto --version` shows version with commit SHA for non-tagged builds
- [ ] Tagged releases show clean semantic version
- [ ] `vX.Y.Z` tags trigger release workflow
- [ ] Release artifacts include Linux and Windows binaries
- [ ] Release artifacts include SHA256 checksums
- [ ] Main builds work when manually enabled
- [ ] README documents CI structure and release process
