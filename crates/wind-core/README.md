# wind-core

Core types and protocol definitions for the WIND (WIND Is Not DIM) protocol.

## Overview

This crate provides the fundamental building blocks for WIND:

- **WindValue**: Type-safe value system for data exchange
- **Message Protocol**: Binary message format with length prefixes
- **Schema System**: Type validation and evolution support
- **Error Types**: Comprehensive error handling

## WindValue Type System

```rust
use wind_core::{WindValue, wind_value};

// Create type-safe values
let temp: WindValue = 23.5f64.into();
let name: WindValue = "sensor_001".into();
let readings: WindValue = vec![temp, name].into();

// Extract with compile-time safety
let extracted_temp: f64 = temp.try_into()?;
```

## Message Protocol

```rust
use wind_core::{Message, MessagePayload, MessageCodec};

// Create a message
let msg = Message::new(MessagePayload::Ping);

// Encode for transmission
let encoded = MessageCodec::encode(&msg)?;

// Decode from bytes
let decoded = MessageCodec::decode(&mut reader).await?;
```

## Schema Validation

```rust
use wind_core::{Schema, SchemaRegistry};

let mut registry = SchemaRegistry::new();
registry.register(schema);

// Validate data against schema
registry.validate("schema_id", &data)?;
```
