use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Core WIND value types with compile-time type safety
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum WindValue {
    Bool(bool),
    I32(i32),
    I64(i64),
    F32(f32),
    F64(f64),
    String(String),
    Bytes(Vec<u8>),
    Array(Vec<WindValue>),
    Map(HashMap<String, WindValue>),
}

/// Type definitions for schema validation
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum WindType {
    Bool,
    I32,
    I64,
    F32,
    F64,
    String,
    Bytes,
    Array(Box<WindType>),
    Map(Box<WindType>),
    Struct(String), // Named struct type
}

/// Service metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceInfo {
    pub name: String,
    pub address: String,
    pub service_type: ServiceType,
    pub schema_id: Option<String>,
    pub ttl_ms: u64,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServiceType {
    Publisher,
    RpcServer,
    Both,
}

/// Subscription modes matching DIM functionality
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SubscriptionMode {
    Once,                          // Single value fetch
    Periodic { interval_ms: u64 }, // Periodic updates
    OnChange,                      // On-change updates (like DIM monitored)
}

/// QoS parameters for subscriptions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QosParams {
    pub reliability: ReliabilityLevel,
    pub durability: bool,    // Retain last value for late joiners
    pub max_queue_size: u32, // Backpressure control
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReliabilityLevel {
    BestEffort, // May drop messages
    Reliable,   // Guaranteed delivery
}

impl Default for QosParams {
    fn default() -> Self {
        Self {
            reliability: ReliabilityLevel::BestEffort,
            durability: false,
            max_queue_size: 1000,
        }
    }
}

// Type-safe macro for defining WIND types
#[macro_export]
macro_rules! wind_value {
    ($val:expr) => {
        WindValue::from($val)
    };
}

// Conversions from Rust types to WindValue
impl From<bool> for WindValue {
    fn from(v: bool) -> Self {
        WindValue::Bool(v)
    }
}

impl From<i32> for WindValue {
    fn from(v: i32) -> Self {
        WindValue::I32(v)
    }
}

impl From<i64> for WindValue {
    fn from(v: i64) -> Self {
        WindValue::I64(v)
    }
}

impl From<f32> for WindValue {
    fn from(v: f32) -> Self {
        WindValue::F32(v)
    }
}

impl From<f64> for WindValue {
    fn from(v: f64) -> Self {
        WindValue::F64(v)
    }
}

impl From<String> for WindValue {
    fn from(v: String) -> Self {
        WindValue::String(v)
    }
}

impl From<&str> for WindValue {
    fn from(v: &str) -> Self {
        WindValue::String(v.to_string())
    }
}

impl From<Vec<u8>> for WindValue {
    fn from(v: Vec<u8>) -> Self {
        WindValue::Bytes(v)
    }
}

// Conversions from WindValue to Rust types
impl TryFrom<WindValue> for bool {
    type Error = crate::WindError;
    fn try_from(v: WindValue) -> std::result::Result<bool, Self::Error> {
        match v {
            WindValue::Bool(b) => Ok(b),
            _ => Err(crate::WindError::TypeMismatch {
                expected: "bool".to_string(),
                actual: format!("{:?}", v),
            }),
        }
    }
}

impl TryFrom<WindValue> for i32 {
    type Error = crate::WindError;
    fn try_from(v: WindValue) -> std::result::Result<i32, Self::Error> {
        match v {
            WindValue::I32(i) => Ok(i),
            _ => Err(crate::WindError::TypeMismatch {
                expected: "i32".to_string(),
                actual: format!("{:?}", v),
            }),
        }
    }
}

impl TryFrom<WindValue> for String {
    type Error = crate::WindError;
    fn try_from(v: WindValue) -> std::result::Result<String, Self::Error> {
        match v {
            WindValue::String(s) => Ok(s),
            _ => Err(crate::WindError::TypeMismatch {
                expected: "String".to_string(),
                actual: format!("{:?}", v),
            }),
        }
    }
}
