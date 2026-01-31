# PRD-005: Introduce `RecordValue` (Typed Lexicon Record Payload)

## Status

Done

## Motivation

`Record.value` is currently represented as `serde_json::Value`, which permits arbitrary JSON. This is weaker than the guarantees required by the AT Protocol, which mandates that **all records include a `$type` field identifying their lexicon schema**.

This PRD introduces a new type, `RecordValue`, whose sole responsibility is to encode this invariant into the type system:

> If a value is a `RecordValue`, it is guaranteed to be a valid AT lexicon record value in the minimal sense: a JSON object containing a `$type` field.

This change reduces runtime checks, clarifies trust boundaries, and aligns with the project principle of making invalid states unrepresentable.

---

## Goals

1. Replace `serde_json::Value` in `Record.value` with a stricter type.
2. Guarantee that all record values:

   * are JSON objects
   * contain a `$type` field
   * where `$type` is a JSON string
3. Enforce these guarantees at **deserialization time**.
4. Provide a canonical, validated construction path for record values.
5. Introduce a CLI command for creating records that respects these invariants.
6. Add integration tests covering both valid and invalid cases.

---

## Non-Goals

* Introducing statically typed lexicon records (e.g. `Post`, `Like`)
* Lexicon schema validation beyond `$type` presence
* Routing or dispatch based on `$type`

---

## Proposed Design

### New Type: `RecordValue`

Introduce a new wrapper type:

```
pub struct RecordValue(serde_json::Value);
```

#### Invariants

A `RecordValue` MUST satisfy all of the following:

1. The top-level JSON value is an object
2. The object contains a `$type` field
3. The `$type` field is a JSON string

These invariants MUST hold for all instances of `RecordValue`.

---

## Serde Semantics (Normative)

### Deserialization

`RecordValue` MUST implement a custom `Deserialize` implementation such that:

* Deserialization fails if the JSON value is not an object
* Deserialization fails if `$type` is missing
* Deserialization fails if `$type` is not a string
* It is impossible to deserialize invalid JSON into a `RecordValue`

This applies to all deserialization paths, including:

* loading records from disk
* reading records from a local PDS store
* ingesting records via CLI or API

### Serialization

Serialization MUST be transparent:

* Serializing a `RecordValue` produces the underlying JSON value unchanged
* No additional wrapper structure is introduced

---

## API Surface

Construction MUST be impossible without passing invariant checks.

---

## Update Existing `Record` Type

The existing `Record` type MUST be updated to:

```
pub struct Record {
    pub uri: AtUri,
    pub cid: String,
    pub value: RecordValue,
}
```

This ensures that **all records in the system are guaranteed to contain a `$type` field**.

---

## CLI: Create Record Command

### Description

Introduce a CLI command for creating records that enforces `RecordValue` invariants.

Example (exact syntax is flexible):

```
orbit record create \
  --collection org.example.foo \
  --type org.example.fooRecord \
  --json payload.json
```

### Behaviour (Normative)

The command MUST:

1. Construct a JSON object representing the record value
2. Ensure a `$type` field is present and matches the provided `--type`
3. Construct a `RecordValue` using the validated JSON
4. Refuse to create a record if:

   * the value is not a JSON object
   * `$type` is missing
   * `$type` is not a string

All record creation paths MUST go through `RecordValue` construction.

---

## Integration Tests

Integration tests MUST be added covering at least the following cases.

### Valid Cases

* Creating a record with a valid JSON object containing `$type`
* Serializing and deserializing a `Record` preserves validity
* CLI `record create` produces a persisted record with `$type`

### Invalid Cases

* Deserializing a record value without `$type` fails
* Deserializing a record value where `$type` is not a string fails
* Deserializing a record value that is not a JSON object fails
* CLI record creation without `$type` fails
* CLI record creation with non-object JSON fails

Failures MUST be asserted explicitly.

---

## Migration and Compatibility

* All existing code that constructs `Record` instances MUST be updated to construct `RecordValue`
* Any existing persisted data without `$type` will fail to load; this is intentional and desired

---

## Success Criteria

This PRD is complete when:

* `Record.value` cannot represent invalid record payloads
* `$type` presence is enforced at deserialization boundaries
* Record creation is impossible without satisfying the invariant
* Integration tests cover both acceptance and rejection paths
