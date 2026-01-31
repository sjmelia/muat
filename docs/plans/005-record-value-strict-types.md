# Implementation Plan: PRD-005 RecordValue (Typed Lexicon Record Payload)

## Overview

This plan details the implementation of PRD-005, which introduces a `RecordValue` type that guarantees all record values contain a `$type` field. This makes invalid states unrepresentable by enforcing AT Protocol invariants at the type level.

## Goals Summary

| Goal | Description |
|------|-------------|
| G1 | Introduce `RecordValue` type with `$type` field guarantee |
| G2 | Enforce invariants at deserialization time |
| G3 | Update `Record.value` to use `RecordValue` |
| G4 | Add CLI `create-record` command with validation |
| G5 | Integration tests for valid and invalid cases |

---

## G1: RecordValue Type

### New Type

**File:** `crates/muat/src/repo/record_value.rs`

```rust
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;
use crate::error::{Error, InvalidInputError};

/// A validated AT Protocol record value.
///
/// Guarantees:
/// - The value is a JSON object
/// - The object contains a `$type` field
/// - The `$type` field is a string
#[derive(Debug, Clone, PartialEq)]
pub struct RecordValue(Value);

impl RecordValue {
    /// Create a new RecordValue from a JSON value.
    ///
    /// Returns an error if the value does not satisfy the invariants.
    pub fn new(value: Value) -> Result<Self, Error> {
        Self::validate(&value)?;
        Ok(Self(value))
    }

    /// Get the `$type` field value.
    pub fn record_type(&self) -> &str {
        // Safe: validated at construction
        self.0["$type"].as_str().unwrap()
    }

    /// Get the inner JSON value.
    pub fn as_value(&self) -> &Value {
        &self.0
    }

    /// Consume and return the inner JSON value.
    pub fn into_value(self) -> Value {
        self.0
    }

    fn validate(value: &Value) -> Result<(), Error> {
        let obj = value.as_object().ok_or_else(|| {
            Error::InvalidInput(InvalidInputError::RecordValue(
                "record value must be a JSON object".to_string(),
            ))
        })?;

        let type_field = obj.get("$type").ok_or_else(|| {
            Error::InvalidInput(InvalidInputError::RecordValue(
                "record value must contain a $type field".to_string(),
            ))
        })?;

        if !type_field.is_string() {
            return Err(Error::InvalidInput(InvalidInputError::RecordValue(
                "$type field must be a string".to_string(),
            )));
        }

        Ok(())
    }
}
```

### Serde Implementation

```rust
impl Serialize for RecordValue {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.0.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for RecordValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = Value::deserialize(deserializer)?;
        RecordValue::new(value).map_err(serde::de::Error::custom)
    }
}
```

---

## G2: Error Type Updates

**File:** `crates/muat/src/error.rs`

Add new variant to `InvalidInputError`:

```rust
pub enum InvalidInputError {
    // ... existing variants ...

    /// Invalid record value (missing $type, wrong type, etc.)
    RecordValue(String),
}

impl std::fmt::Display for InvalidInputError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            // ... existing matches ...
            Self::RecordValue(msg) => write!(f, "invalid record value: {}", msg),
        }
    }
}
```

---

## G3: Update Record Type

**File:** `crates/muat/src/repo/types.rs`

```rust
use super::record_value::RecordValue;

/// A record from the repository.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Record {
    /// The AT URI of this record.
    pub uri: AtUri,

    /// The CID (content identifier) of this record.
    pub cid: String,

    /// The record value.
    ///
    /// Guaranteed to be a JSON object with a `$type` field.
    pub value: RecordValue,
}
```

---

## G4: CLI Create Record Command

### New Command

**File:** `crates/atproto-cli/src/commands/pds/create_record.rs`

```
atproto pds create-record <collection> --type <type> [--rkey <rkey>] [--json <file>|-]
```

### Arguments

- `<collection>` - Collection NSID (positional, required)
- `--type <type>` - The `$type` value for the record (required)
- `--rkey <rkey>` - Optional record key (auto-generated if omitted)
- `--json <file>` - JSON file with additional fields (or `-` for stdin)

### Behavior

1. Parse collection NSID
2. Read JSON input (if provided) or start with empty object
3. Ensure `$type` field is set to the provided `--type` value
4. Construct `RecordValue` (validates invariants)
5. Call `session.create_record_raw()`
6. Print created record URI

### Implementation

```rust
#[derive(Args, Debug)]
pub struct CreateRecordArgs {
    /// Collection NSID
    pub collection: String,

    /// Record type ($type field value)
    #[arg(long = "type", short = 't')]
    pub record_type: String,

    /// Optional record key
    #[arg(long)]
    pub rkey: Option<String>,

    /// JSON file with record data (use - for stdin)
    #[arg(long)]
    pub json: Option<String>,
}

pub async fn handle(args: CreateRecordArgs, session: &Session) -> Result<()> {
    let collection = Nsid::new(&args.collection)?;

    // Read base JSON
    let mut value: serde_json::Map<String, Value> = if let Some(ref path) = args.json {
        if path == "-" {
            serde_json::from_reader(std::io::stdin())?
        } else {
            serde_json::from_reader(std::fs::File::open(path)?)?
        }
    } else {
        serde_json::Map::new()
    };

    // Set $type
    value.insert("$type".to_string(), Value::String(args.record_type));

    // Construct RecordValue (validates)
    let record_value = RecordValue::new(Value::Object(value))?;

    // Create record
    let uri = session.create_record(&collection, record_value, args.rkey.as_deref()).await?;

    println!("{}", uri);
    Ok(())
}
```

### Update Session API

**File:** `crates/muat/src/auth/session.rs`

Add typed create_record method:

```rust
impl Session {
    /// Create a record with a validated RecordValue.
    pub async fn create_record(
        &self,
        collection: &Nsid,
        value: RecordValue,
        rkey: Option<&str>,
    ) -> Result<AtUri> {
        self.create_record_raw(collection, value.as_value(), rkey).await
    }
}
```

---

## G5: Integration Tests

### Valid Cases

**File:** `crates/muat/tests/record_value_tests.rs`

```rust
#[test]
fn test_record_value_valid() {
    let json = json!({
        "$type": "org.example.test",
        "text": "hello"
    });
    let rv = RecordValue::new(json).unwrap();
    assert_eq!(rv.record_type(), "org.example.test");
}

#[test]
fn test_record_value_deserialize() {
    let json_str = r#"{"$type": "org.example.test", "data": 123}"#;
    let rv: RecordValue = serde_json::from_str(json_str).unwrap();
    assert_eq!(rv.record_type(), "org.example.test");
}

#[test]
fn test_record_value_serialize_roundtrip() {
    let json = json!({"$type": "org.example.test"});
    let rv = RecordValue::new(json.clone()).unwrap();
    let serialized = serde_json::to_value(&rv).unwrap();
    assert_eq!(serialized, json);
}
```

### Invalid Cases

```rust
#[test]
fn test_record_value_missing_type() {
    let json = json!({"text": "hello"});
    let err = RecordValue::new(json).unwrap_err();
    assert!(matches!(err, Error::InvalidInput(_)));
}

#[test]
fn test_record_value_type_not_string() {
    let json = json!({"$type": 123});
    let err = RecordValue::new(json).unwrap_err();
    assert!(matches!(err, Error::InvalidInput(_)));
}

#[test]
fn test_record_value_not_object() {
    let json = json!([1, 2, 3]);
    let err = RecordValue::new(json).unwrap_err();
    assert!(matches!(err, Error::InvalidInput(_)));
}

#[test]
fn test_record_value_null() {
    let json = json!(null);
    let err = RecordValue::new(json).unwrap_err();
    assert!(matches!(err, Error::InvalidInput(_)));
}

#[test]
fn test_deserialize_invalid_fails() {
    let json_str = r#"{"text": "no type field"}"#;
    let result: Result<RecordValue, _> = serde_json::from_str(json_str);
    assert!(result.is_err());
}
```

### CLI Tests

**File:** `crates/atproto-cli/tests/integration.rs` (extend)

```rust
#[test]
fn test_create_record_valid() {
    // Create record with valid $type
    let output = run_cli(&[
        "pds", "create-record", "org.muat.test.record",
        "--type", "org.muat.test.record",
    ]);
    assert!(output.status.success());
    assert!(output.stdout.starts_with("at://"));
}

#[test]
fn test_create_record_with_json() {
    // Create record with JSON payload
    let output = run_cli(&[
        "pds", "create-record", "org.muat.test.record",
        "--type", "org.muat.test.record",
        "--json", "-",  // stdin
    ]);
    // ... provide JSON via stdin
}
```

---

## Files to Create/Modify

| File | Action | Description |
|------|--------|-------------|
| `crates/muat/src/repo/record_value.rs` | Create | RecordValue type |
| `crates/muat/src/repo/mod.rs` | Modify | Export record_value module |
| `crates/muat/src/repo/types.rs` | Modify | Update Record to use RecordValue |
| `crates/muat/src/error.rs` | Modify | Add RecordValue error variant |
| `crates/muat/src/lib.rs` | Modify | Re-export RecordValue |
| `crates/muat/src/auth/session.rs` | Modify | Add typed create_record method |
| `crates/atproto-cli/src/commands/pds/create_record.rs` | Create | CLI command |
| `crates/atproto-cli/src/commands/pds/mod.rs` | Modify | Add create-record command |
| `crates/muat/tests/record_value_tests.rs` | Create | Unit tests |
| `crates/atproto-cli/tests/integration.rs` | Modify | CLI integration tests |

---

## Implementation Order

1. **G1** - Create RecordValue type with validation
2. **G2** - Add error variant
3. **G3** - Update Record type (may require updating existing tests)
4. **G4** - Add CLI create-record command
5. **G5** - Add integration tests

---

## Migration Notes

- All code constructing `Record` instances must now use `RecordValue`
- Existing persisted data without `$type` will fail to deserialize (intentional)
- The `create_record_raw` method remains for backwards compatibility but `create_record` is preferred

---

## Success Criteria

- [ ] `RecordValue` type exists with `$type` field guarantee
- [ ] `Record.value` uses `RecordValue` instead of `serde_json::Value`
- [ ] Deserialization fails for invalid record values
- [ ] CLI `create-record` command works with validation
- [ ] Unit tests cover valid and invalid cases
- [ ] Integration tests verify end-to-end behavior
