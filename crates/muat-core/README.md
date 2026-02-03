# muat-core

Core types and traits for the AT Protocol (PDS) ecosystem.

`muat-core` contains:

- Strongly-typed protocol primitives (`Did`, `Nsid`, `AtUri`, `PdsUrl`, `Rkey`)
- `RecordValue` and repository event types
- Shared error types
- Traits for `Pds`, `Session`, and `Firehose`

It does **not** include any networking or filesystem implementation. For concrete PDS implementations:

- Use `muat-xrpc` for real PDS servers over HTTPS
- Use `muat-file` for local filesystem PDS

## Example (Types)

```rust
use muat_core::{Nsid, PdsUrl};

let pds = PdsUrl::new("https://bsky.social")?;
let collection = Nsid::new("app.bsky.feed.post")?;
# Ok::<(), muat_core::Error>(())
```

### Key Design Principles

1. **Session-scoped auth** - All authenticated operations flow through a `Session` object
2. **Strong typing** - Protocol types (`Did`, `Nsid`, `AtUri`, `RecordValue`) are validated at construction
3. **Schema-agnostic** - Record values use `RecordValue` (guarantees `$type` field), not typed lexicons. This keeps µat usable with arbitrary or evolving schemas while still enforcing AT protocol invariants.
4. **Explicit over magic** - No hidden retries, no global state, no implicit defaults
5. **Local-first development** - `file://` URLs enable offline development without a network PDS

## Core Types

| Type          | Description                                                          |
| ------------- | -------------------------------------------------------------------- |
| `Did`         | Decentralized Identifier (`did:plc:...`, `did:web:...`)              |
| `Nsid`        | Namespaced Identifier (`app.bsky.feed.post`)                         |
| `AtUri`       | AT Protocol URI (`at://did/collection/rkey`)                         |
| `PdsUrl`      | PDS URL (HTTPS for network, HTTP for localhost, `file://` for local) |
| `RecordValue` | Validated record payload (JSON object with `$type` field)            |
| `Session`     | Authenticated session with a PDS                                     |
| `Credentials` | Login identifier + password                                          |

## Traits

```rust
use muat_core::traits::{Pds, Session};
```

Implementations live in other crates and conform to these traits.

## Error Handling

`muat-core` exposes a unified `Error` type with variants for transport, auth, protocol, and input validation.

## See Also

- `muat-xrpc` for network PDS access
- `muat-file` for local file-backed PDS
- `atproto-cli` for a reference CLI built on these crates
