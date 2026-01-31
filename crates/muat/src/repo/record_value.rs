//! Validated record value type for AT Protocol records.
//!
//! This module provides [`RecordValue`], a type that guarantees the value
//! is a valid AT Protocol record payload (a JSON object with a `$type` field).

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;

use crate::error::{Error, InvalidInputError};

/// A validated AT Protocol record value.
///
/// This type guarantees that:
/// - The value is a JSON object
/// - The object contains a `$type` field
/// - The `$type` field is a string
///
/// These invariants are enforced at construction and deserialization time,
/// making it impossible to create an invalid `RecordValue`.
///
/// # Example
///
/// ```
/// use muat::repo::RecordValue;
/// use serde_json::json;
///
/// let value = RecordValue::new(json!({
///     "$type": "app.bsky.feed.post",
///     "text": "Hello, world!"
/// })).unwrap();
///
/// assert_eq!(value.record_type(), "app.bsky.feed.post");
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct RecordValue(Value);

impl RecordValue {
    /// Create a new `RecordValue` from a JSON value.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The value is not a JSON object
    /// - The object does not contain a `$type` field
    /// - The `$type` field is not a string
    pub fn new(value: Value) -> Result<Self, Error> {
        Self::validate(&value)?;
        Ok(Self(value))
    }

    /// Create a new `RecordValue` with the given type and additional fields.
    ///
    /// This is a convenience constructor that ensures `$type` is set correctly.
    ///
    /// # Example
    ///
    /// ```
    /// use muat::repo::RecordValue;
    /// use serde_json::json;
    ///
    /// let value = RecordValue::with_type("org.example.record", json!({
    ///     "text": "hello"
    /// })).unwrap();
    ///
    /// assert_eq!(value.record_type(), "org.example.record");
    /// ```
    pub fn with_type(record_type: &str, mut value: Value) -> Result<Self, Error> {
        // Ensure it's an object
        if !value.is_object() {
            return Err(Error::InvalidInput(InvalidInputError::RecordValue {
                reason: "record value must be a JSON object".to_string(),
            }));
        }

        // Set/override the $type field
        value
            .as_object_mut()
            .unwrap()
            .insert("$type".to_string(), Value::String(record_type.to_string()));

        Self::new(value)
    }

    /// Get the `$type` field value.
    ///
    /// This is guaranteed to return a valid string due to construction invariants.
    pub fn record_type(&self) -> &str {
        // Safe: validated at construction
        self.0["$type"].as_str().unwrap()
    }

    /// Get a reference to the inner JSON value.
    pub fn as_value(&self) -> &Value {
        &self.0
    }

    /// Consume and return the inner JSON value.
    pub fn into_value(self) -> Value {
        self.0
    }

    /// Get a field from the record value.
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.0.get(key)
    }

    fn validate(value: &Value) -> Result<(), Error> {
        let obj = value.as_object().ok_or_else(|| {
            Error::InvalidInput(InvalidInputError::RecordValue {
                reason: "record value must be a JSON object".to_string(),
            })
        })?;

        let type_field = obj.get("$type").ok_or_else(|| {
            Error::InvalidInput(InvalidInputError::RecordValue {
                reason: "record value must contain a $type field".to_string(),
            })
        })?;

        if !type_field.is_string() {
            return Err(Error::InvalidInput(InvalidInputError::RecordValue {
                reason: "$type field must be a string".to_string(),
            }));
        }

        Ok(())
    }
}

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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_valid_record_value() {
        let value = RecordValue::new(json!({
            "$type": "org.example.test",
            "text": "hello"
        }))
        .unwrap();

        assert_eq!(value.record_type(), "org.example.test");
    }

    #[test]
    fn test_with_type() {
        let value = RecordValue::with_type(
            "org.example.test",
            json!({
                "text": "hello"
            }),
        )
        .unwrap();

        assert_eq!(value.record_type(), "org.example.test");
        assert_eq!(value.get("text").unwrap(), "hello");
    }

    #[test]
    fn test_with_type_overrides_existing() {
        let value = RecordValue::with_type(
            "org.example.new",
            json!({
                "$type": "org.example.old",
                "text": "hello"
            }),
        )
        .unwrap();

        assert_eq!(value.record_type(), "org.example.new");
    }

    #[test]
    fn test_missing_type_fails() {
        let result = RecordValue::new(json!({
            "text": "hello"
        }));

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, Error::InvalidInput(InvalidInputError::RecordValue { .. })));
    }

    #[test]
    fn test_type_not_string_fails() {
        let result = RecordValue::new(json!({
            "$type": 123
        }));

        assert!(result.is_err());
    }

    #[test]
    fn test_not_object_fails() {
        let result = RecordValue::new(json!([1, 2, 3]));
        assert!(result.is_err());

        let result = RecordValue::new(json!(null));
        assert!(result.is_err());

        let result = RecordValue::new(json!("string"));
        assert!(result.is_err());
    }

    #[test]
    fn test_deserialize_valid() {
        let json_str = r#"{"$type": "org.example.test", "data": 123}"#;
        let value: RecordValue = serde_json::from_str(json_str).unwrap();
        assert_eq!(value.record_type(), "org.example.test");
    }

    #[test]
    fn test_deserialize_invalid_fails() {
        let json_str = r#"{"text": "no type field"}"#;
        let result: Result<RecordValue, _> = serde_json::from_str(json_str);
        assert!(result.is_err());
    }

    #[test]
    fn test_serialize_roundtrip() {
        let original = json!({
            "$type": "org.example.test",
            "number": 42
        });
        let value = RecordValue::new(original.clone()).unwrap();
        let serialized = serde_json::to_value(&value).unwrap();
        assert_eq!(serialized, original);
    }
}
