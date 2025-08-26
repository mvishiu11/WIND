use crate::{Result, WindType, WindValue};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Schema definition for type validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Schema {
    pub id: String,
    pub version: u32,
    pub name: String,
    pub description: Option<String>,
    pub fields: HashMap<String, WindType>,
}

impl Schema {
    pub fn validate(&self, value: &WindValue) -> Result<()> {
        match value {
            WindValue::Map(map) => {
                // Validate all required fields are present and have correct types
                for (field_name, expected_type) in &self.fields {
                    if let Some(field_value) = map.get(field_name) {
                        self.validate_type(field_value, expected_type)?;
                    } else {
                        return Err(crate::WindError::Schema(format!(
                            "Missing required field: {}",
                            field_name
                        )));
                    }
                }
                Ok(())
            }
            _ => Err(crate::WindError::Schema(
                "Schema validation requires a Map value".to_string(),
            )),
        }
    }

    fn validate_type(&self, value: &WindValue, expected: &WindType) -> Result<()> {
        let matches = match (value, expected) {
            (WindValue::Bool(_), WindType::Bool) => true,
            (WindValue::I32(_), WindType::I32) => true,
            (WindValue::I64(_), WindType::I64) => true,
            (WindValue::F32(_), WindType::F32) => true,
            (WindValue::F64(_), WindType::F64) => true,
            (WindValue::String(_), WindType::String) => true,
            (WindValue::Bytes(_), WindType::Bytes) => true,
            (WindValue::Array(arr), WindType::Array(inner)) => {
                // Validate all array elements
                return arr.iter().try_for_each(|v| self.validate_type(v, inner));
            }
            (WindValue::Map(_), WindType::Map(_)) => true, // TODO: Validate map values
            _ => false,
        };

        if matches {
            Ok(())
        } else {
            Err(crate::WindError::TypeMismatch {
                expected: format!("{:?}", expected),
                actual: format!("{:?}", value),
            })
        }
    }
}

/// Schema registry for managing schemas
#[derive(Debug, Default)]
pub struct SchemaRegistry {
    schemas: HashMap<String, Schema>,
}

impl SchemaRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, schema: Schema) {
        self.schemas.insert(schema.id.clone(), schema);
    }

    pub fn get(&self, id: &str) -> Option<&Schema> {
        self.schemas.get(id)
    }

    pub fn validate(&self, schema_id: &str, value: &WindValue) -> Result<()> {
        let schema = self
            .get(schema_id)
            .ok_or_else(|| crate::WindError::Schema(format!("Schema not found: {}", schema_id)))?;
        schema.validate(value)
    }
}
