# muat-file

Filesystem-backed PDS implementation for local development and testing.

This crate provides:
- `FilePds` (implements `muat_core::traits::Pds`)
- `FileSession` (implements `muat_core::traits::Session`)
- `FileFirehose` (implements `muat_core::traits::Firehose`)

## Example

```rust
use muat_core::traits::{Pds, Session};
use muat_core::{Credentials, Nsid, PdsUrl, RecordValue};
use muat_file::FilePds;
use serde_json::json;

# async fn example() -> Result<(), muat_core::Error> {
let pds_url = PdsUrl::new("file:///tmp/pds")?;
let pds = FilePds::new("/tmp/pds", pds_url);

let _ = pds.create_account("alice.local", Some("password"), None, None).await?;
let session = pds.login(Credentials::new("alice.local", "password")).await?;

let collection = Nsid::new("org.example.record")?;
let value = RecordValue::with_type("org.example.record", json!({"text": "hi"}))?;
let _uri = session.create_record(&collection, &value).await?;
# Ok(())
# }
```

## Notes

- Passwords are hashed with bcrypt and stored in account metadata.
- Tokens are JSON strings containing the DID and password hash.
- Every request validates the token and enforces repo ownership.
