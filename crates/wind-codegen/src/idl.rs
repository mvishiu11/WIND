use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// WIND Interface Definition Language (IDL) schema
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindIdl {
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub types: HashMap<String, TypeDefinition>,
    pub services: HashMap<String, ServiceDefinition>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum TypeDefinition {
    Primitive { 
        primitive_type: PrimitiveType 
    },
    Struct { 
        fields: HashMap<String, FieldDefinition> 
    },
    Enum { 
        variants: Vec<String> 
    },
    Array { 
        element_type: Box<TypeDefinition> 
    },
    Optional { 
        inner_type: Box<TypeDefinition> 
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PrimitiveType {
    Bool,
    I32,
    I64,
    F32,
    F64,
    String,
    Bytes,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldDefinition {
    pub field_type: TypeDefinition,
    pub description: Option<String>,
    pub optional: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceDefinition {
    pub description: Option<String>,
    pub methods: HashMap<String, MethodDefinition>,
    pub publications: HashMap<String, PublicationDefinition>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MethodDefinition {
    pub description: Option<String>,
    pub params: TypeDefinition,
    pub returns: TypeDefinition,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublicationDefinition {
    pub description: Option<String>,
    pub data_type: TypeDefinition,
}
