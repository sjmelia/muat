# PRD-0004: Versioning, CI Builds, and Releases

## Status

Done

## Scope

This PRD defines:

* how the `atproto` binary reports its version
* how CI produces build artifacts from `main`
* how tagged releases are produced
* which platforms are supported
* how this process is documented

The scope explicitly covers **CI and release mechanics**, not protocol behaviour.

---

## Motivation

The project has reached a point where built binaries are useful for:

* manual testing
* sharing with others
* validating cross-platform behaviour

Rather than introducing a separate "nightly" concept, this PRD adopts a simpler model:

> **Every push to `main` may produce a build artifact, identified by commit SHA.**

Tagged releases remain the only source of official, versioned releases.

---

## Goals

### G1 — Explicit version reporting

* The `atproto` binary MUST support:

```
atproto --version
```

* Version output MUST clearly distinguish:

  * builds from `main`
  * tagged releases

---

### G2 — Builds from `main`

* CI SHOULD be capable of producing release-style binaries from `main`
* These builds are intended for:

  * manual testing
  * canary usage

Initial constraints:

* platforms: **Linux + Windows**
* triggered only on pushes to `main`
* **disabled by default**, behind a CI toggle

---

### G3 — Tagged releases

* CI MUST produce release binaries for tagged releases
* platforms: **Linux + Windows**
* enabled immediately

Releases should:

* be reproducible
* have stable naming
* attach binaries and checksums

---

### G4 — Documentation

* `README.md` MUST describe:

  * CI structure at a high level
  * builds from `main`
  * how releases are created

---

## Non-Goals

* macOS builds (may be added later)
* package managers (Homebrew, winget, etc.)
* signing / notarisation

---

## Detailed Design

## G1 — Version reporting

### Requirements

* `atproto --version` MUST print:

  * the semantic version (if present)
  * otherwise the most recent release version plus commit SHA

Examples:

```
atproto 0.2.0
```

```
atproto 0.2.0+abc1234
```

Where:

* `0.2.0` is the latest tagged release reachable from `HEAD`
* `abc1234` is the short git SHA

---

### Source of version information

* For tagged releases:

  * version derived from the git tag
* For `main` builds:

  * latest reachable release tag
  * plus short commit SHA

If no release tag exists yet, the version MAY fall back to:

```
0.0.0+<sha>
```

---

## G2 — Builds from `main`

### Behaviour

* Triggered on pushes to `main`

* Build matrix:

  * Linux (x86_64)
  * Windows (x86_64)

* Artifacts:

  * uploaded as CI artifacts
  * clearly labelled as "main" builds

### Toggle

* Builds from `main` MUST be guarded by a CI-level toggle (e.g. workflow input or env flag)
* Default state: **disabled**

This allows enabling these builds without restructuring CI.

---

## G3 — Tagged releases

### Tag scheme

Use annotated git tags of the form:

```
vX.Y.Z
```

Examples:

* `v0.1.0`
* `v0.2.1`

Rationale:

* matches Rust ecosystem conventions
* integrates cleanly with GitHub Releases
* avoids inventing a parallel namespace

---

### Release process

1. Ensure `main` is green
2. Create and push a tag:

```
git tag -a vX.Y.Z -m "vX.Y.Z"
git push origin vX.Y.Z
```

3. CI:

   * builds Linux + Windows binaries
   * creates a GitHub Release
   * attaches binaries and checksums
   * generates release notes automatically

---

## Platform testing policy

### Integration tests

* Integration tests:

  * MUST run on **Linux**
  * MAY be skipped on Windows

Rationale:

* integration tests hit a real PDS over the network
* behaviour is not OS-specific
* skipping on Windows:

  * reduces CI cost and complexity
  * avoids platform-specific flakiness

Windows builds are validated by:

* successful compilation
* unit tests

---

## G4 — README updates

`README.md` MUST include a section describing:

* CI stages (fmt, clippy, tests, builds)
* builds produced from `main`
* how to create a release via tagging
* which platforms are supported

---

## Definition of Done

* `atproto --version` implemented
* CI supports Linux + Windows builds from `main`
* Tagged releases produce binaries automatically
* README updated to reflect the above
