# muat-xrpc

XRPC-backed PDS implementation for AT Protocol.

This crate provides:

- `XrpcPds` (implements `muat_core::traits::Pds`)
- `XrpcSession` (implements `muat_core::traits::Session`)
- `XrpcFirehose` (implements `muat_core::traits::Firehose`)

## Example

```rust
use muat_core::traits::{Pds, Session};
use muat_core::{Credentials, Nsid, PdsUrl};
use muat_xrpc::XrpcPds;

# async fn example() -> Result<(), muat_core::Error> {
let pds = XrpcPds::new(PdsUrl::new("https://bsky.social")?);
let session = pds.login(Credentials::new("alice.bsky.social", "app-password")).await?;

let collection = Nsid::new("app.bsky.feed.post")?;
let records = session.list_records(session.did(), &collection, None, None).await?;
# Ok(())
# }
```

## Notes

- Token refresh is explicit via `XrpcSession::refresh()`.
- Firehose streaming uses WebSocket `com.atproto.sync.subscribeRepos`.
