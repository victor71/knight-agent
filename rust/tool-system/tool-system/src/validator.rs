//! Argument Validator
//!
//! Validates tool arguments against JSON Schema.

use crate::types::{JsonSchema, JsonSchemaProperty, ValidationResult};

/// Validates arguments against a JSON Schema.
pub struct ArgumentValidator;

impl ArgumentValidator {
    /// Validate arguments against schema
    pub fn validate(args: &serde_json::Value, schema: &JsonSchema) -> ValidationResult {
        let mut result = ValidationResult::valid();

        // Check required fields
        for field in &schema.required {
            if args.get(field).is_none() {
                result.add_error(field, &format!("required field '{}' is missing", field));
            }
        }

        // Validate each provided field
        if let Some(obj) = args.as_object() {
            for (key, value) in obj {
                if let Some(prop) = schema.properties.get(key) {
                    Self::validate_property(value, prop, key, &mut result);
                } else if !schema.additional_properties {
                    result.add_error(key, &format!("unknown field '{}' is not allowed", key));
                }
            }
        }

        result
    }

    fn validate_property(
        value: &serde_json::Value,
        prop: &JsonSchemaProperty,
        field_name: &str,
        result: &mut ValidationResult,
    ) {
        // Type checking
        let expected_type = &prop.property_type;
        let actual_type = match value {
            serde_json::Value::Null => "null",
            serde_json::Value::Bool(_) => "boolean",
            serde_json::Value::Number(_) => "number",
            serde_json::Value::String(_) => "string",
            serde_json::Value::Array(_) => "array",
            serde_json::Value::Object(_) => "object",
        };

        // Handle number type specially (JSON Schema allows integer as subtype of number)
        if expected_type == "number" && actual_type == "number" {
            return;
        }
        if expected_type == "integer" && actual_type == "number" {
            // Check if it's actually an integer
            if let Some(n) = value.as_f64() {
                if n.fract() != 0.0 {
                    result.add_error(
                        field_name,
                        &format!("expected integer but got float {}", n),
                    );
                }
            }
            return;
        }

        if expected_type != actual_type {
            result.add_error(
                field_name,
                &format!(
                    "type mismatch: expected {} but got {}",
                    expected_type, actual_type
                ),
            );
            return;
        }

        // Enum check
        if let Some(ref enum_values) = prop.enum_values {
            if let Some(s) = value.as_str() {
                if !enum_values.contains(&s.to_string()) {
                    result.add_error(
                        field_name,
                        &format!(
                            "value '{}' is not one of allowed values: {:?}",
                            s, enum_values
                        ),
                    );
                }
            }
        }
    }

    /// Parse a JSON Schema from JSON value
    pub fn parse_schema(value: &serde_json::Value) -> Option<JsonSchema> {
        serde_json::from_value(value.clone()).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn create_test_schema() -> JsonSchema {
        let mut properties = HashMap::new();
        properties.insert(
            "name".to_string(),
            JsonSchemaProperty {
                property_type: "string".to_string(),
                description: "Name field".to_string(),
                enum_values: None,
            },
        );
        properties.insert(
            "age".to_string(),
            JsonSchemaProperty {
                property_type: "integer".to_string(),
                description: "Age field".to_string(),
                enum_values: None,
            },
        );
        properties.insert(
            "email".to_string(),
            JsonSchemaProperty {
                property_type: "string".to_string(),
                description: "Email field".to_string(),
                enum_values: Some(vec!["work".to_string(), "personal".to_string()]),
            },
        );

        JsonSchema {
            schema_type: "object".to_string(),
            properties,
            required: vec!["name".to_string()],
            additional_properties: false,
        }
    }

    #[test]
    fn test_valid_args() {
        let schema = create_test_schema();
        let args = serde_json::json!({
            "name": "test",
            "age": 25,
            "email": "work"
        });

        let result = ArgumentValidator::validate(&args, &schema);
        assert!(result.valid);
        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_missing_required() {
        let schema = create_test_schema();
        let args = serde_json::json!({
            "age": 25
        });

        let result = ArgumentValidator::validate(&args, &schema);
        assert!(!result.valid);
        assert!(result.errors.iter().any(|e| e.field == "name"));
    }

    #[test]
    fn test_type_mismatch() {
        let schema = create_test_schema();
        let args = serde_json::json!({
            "name": 123,
            "age": 25
        });

        let result = ArgumentValidator::validate(&args, &schema);
        assert!(!result.valid);
        assert!(result.errors.iter().any(|e| e.field == "name"));
    }

    #[test]
    fn test_enum_violation() {
        let schema = create_test_schema();
        let args = serde_json::json!({
            "name": "test",
            "email": "invalid"
        });

        let result = ArgumentValidator::validate(&args, &schema);
        assert!(!result.valid);
        assert!(result.errors.iter().any(|e| e.field == "email"));
    }

    #[test]
    fn test_unknown_field() {
        let schema = create_test_schema();
        let args = serde_json::json!({
            "name": "test",
            "unknown": "value"
        });

        let result = ArgumentValidator::validate(&args, &schema);
        assert!(!result.valid);
        assert!(result.errors.iter().any(|e| e.field == "unknown"));
    }
}
