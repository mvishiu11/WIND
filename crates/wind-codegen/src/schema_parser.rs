use crate::idl::*;
use anyhow::{anyhow, Result};
use serde_json;

/// Parse WIND IDL from JSON format
pub fn parse_idl(idl_json: &str) -> Result<WindIdl> {
    serde_json::from_str(idl_json)
        .map_err(|e| anyhow!("Failed to parse IDL: {}", e))
}

/// Example IDL schema for a temperature sensor
pub fn example_sensor_idl() -> String {
    serde_json::to_string_pretty(&WindIdl {
        name: "TemperatureSensor".to_string(),
        version: "1.0.0".to_string(),
        description: Some("Temperature sensor service".to_string()),
        types: {
            let mut types = std::collections::HashMap::new();
            
            types.insert("Temperature".to_string(), TypeDefinition::Struct {
                fields: {
                    let mut fields = std::collections::HashMap::new();
                    fields.insert("value".to_string(), FieldDefinition {
                        field_type: TypeDefinition::Primitive { 
                            primitive_type: PrimitiveType::F64 
                        },
                        description: Some("Temperature in Celsius".to_string()),
                        optional: false,
                    });
                    fields.insert("timestamp".to_string(), FieldDefinition {
                        field_type: TypeDefinition::Primitive { 
                            primitive_type: PrimitiveType::I64 
                        },
                        description: Some("Unix timestamp in microseconds".to_string()),
                        optional: false,
                    });
                    fields.insert("sensor_id".to_string(), FieldDefinition {
                        field_type: TypeDefinition::Primitive { 
                            primitive_type: PrimitiveType::String 
                        },
                        description: Some("Sensor identifier".to_string()),
                        optional: false,
                    });
                    fields
                }
            });
            
            types.insert("CalibrationCommand".to_string(), TypeDefinition::Struct {
                fields: {
                    let mut fields = std::collections::HashMap::new();
                    fields.insert("offset".to_string(), FieldDefinition {
                        field_type: TypeDefinition::Primitive { 
                            primitive_type: PrimitiveType::F64 
                        },
                        description: Some("Calibration offset".to_string()),
                        optional: false,
                    });
                    fields.insert("scale".to_string(), FieldDefinition {
                        field_type: TypeDefinition::Optional {
                            inner_type: Box::new(TypeDefinition::Primitive { 
                                primitive_type: PrimitiveType::F64 
                            })
                        },
                        description: Some("Optional calibration scale factor".to_string()),
                        optional: true,
                    });
                    fields
                }
            });
            
            types
        },
        services: {
            let mut services = std::collections::HashMap::new();
            
            services.insert("TemperatureSensorService".to_string(), ServiceDefinition {
                description: Some("Main temperature sensor service".to_string()),
                methods: {
                    let mut methods = std::collections::HashMap::new();
                    methods.insert("calibrate".to_string(), MethodDefinition {
                        description: Some("Calibrate the sensor".to_string()),
                        params: TypeDefinition::Struct { fields: std::collections::HashMap::new() },
                        returns: TypeDefinition::Primitive { primitive_type: PrimitiveType::Bool },
                    });
                    methods.insert("get_status".to_string(), MethodDefinition {
                        description: Some("Get sensor status".to_string()),
                        params: TypeDefinition::Struct { fields: std::collections::HashMap::new() },
                        returns: TypeDefinition::Primitive { primitive_type: PrimitiveType::String },
                    });
                    methods
                },
                publications: {
                    let mut pubs = std::collections::HashMap::new();
                    pubs.insert("temperature".to_string(), PublicationDefinition {
                        description: Some("Temperature readings".to_string()),
                        data_type: TypeDefinition::Struct { fields: std::collections::HashMap::new() },
                    });
                    pubs
                },
            });
            
            services
        },
    }).unwrap()
}
