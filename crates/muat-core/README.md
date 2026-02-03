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
