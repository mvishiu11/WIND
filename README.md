# WIND (WIND Is Not DIM) Protocol

A modern, high-performance replacement for the DIM protocol, implemented in Rust with full type safety, async I/O, and comprehensive observability.

## 🚀 Quick Start

### Prerequisites
- Rust with Cargo

### Build
```bash
git clone <repo>
cd wind
cargo build --release
```

### Run Example
```bash
# Terminal 1: Start registry
cargo run -p wind-registry

# Terminal 2: Run temperature sensor example  
cargo run --example temperature_sensor

# Terminal 3: Use CLI to discover services
cargo run -p wind-cli discover "SENSOR/*"
```

## 📁 Project Structure

```
wind/
├── crates/
│   ├── wind-core/          # Core types and protocol definitions
│   ├── wind-registry/      # Service discovery registry
│   ├── wind-client/        # Client library (subscribers, RPC clients)
│   ├── wind-server/        # Server library (publishers, RPC servers)
│   ├── wind-codegen/       # Code generation from IDL schemas
│   ├── wind-cli/           # Command-line tools
│   └── wind-bench/         # Performance benchmarking suite
├── examples/               # Working examples and demos
├── tests/                  # Integration tests
└── docs/                   # Additional documentation
```

## 🎯 Key Features

### ✅ Implemented
- **Type Safety**: Compile-time type checking with WindValue system
- **Async I/O**: High-performance async networking with Tokio
- **Service Discovery**: Pattern-based discovery (e.g., `SENSOR/*/TEMP`)
- **Pub/Sub**: One-to-many data distribution with subscription modes
- **RPC**: Type-safe remote procedure calls with timeouts
- **Auto-Reconnection**: Automatic recovery from network failures
- **Cross-Platform**: Works on Linux, Windows, macOS
- **Observability**: Structured logging with tracing
- **CLI Tools**: Command-line interface for debugging and monitoring
- **Benchmarking**: Performance measurement tools

### 🚧 In Progress  
- **Schema Evolution**: IDL-based type generation
- **Metrics**: Prometheus metrics integration
- **Distributed Tracing**: OpenTelemetry support
- **Security**: TLS and authentication
- **Multi-Language**: C/C++, Python, Java bindings

## 🏗️ Architecture

### Components

1. **Registry**: Central service discovery with pattern matching
2. **Publishers**: Services that broadcast data to subscribers
3. **Subscribers**: Clients that receive data updates
4. **RPC Servers**: Services that handle remote procedure calls
5. **RPC Clients**: Clients that make remote procedure calls

### Message Flow

```
┌─────────────┐    ┌──────────┐    ┌─────────────┐
│ Publisher   │───▶│ Registry │◄───│ Subscriber  │
└─────────────┘    └──────────┘    └─────────────┘
       │                                   ▲
       └───────── Direct Connection ───────┘
```

### Type System

WIND provides compile-time type safety through the `WindValue` enum:

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum WindValue {
    Bool(bool),
    I32(i32), I64(i64),
    F32(f32), F64(f64),
    String(String),
    Bytes(Vec<u8>),
    Array(Vec<WindValue>),
    Map(HashMap<String, WindValue>),
}
```

## 📚 API Documentation

### Publisher Example
```rust
use wind_server::Publisher;
use wind_core::WindValue;

let publisher = Publisher::new(
    "SENSOR/ROOM_A/TEMP".to_string(),
    "127.0.0.1:0".to_string(),
    "127.0.0.1:7001".to_string(), // registry
).with_tags(vec!["sensor".to_string()]);

// Start publisher (registers with registry)
tokio::spawn(async move { publisher.start().await });

// Publish data
publisher.publish(WindValue::F64(23.5)).await?;
```

### Subscriber Example
```rust
use wind_client::WindClient;
use wind_core::{SubscriptionMode, QosParams};

let mut client = WindClient::new("127.0.0.1:7001".to_string());

let mut sub = client.subscribe_with_options(
    "SENSOR/ROOM_A/TEMP",
    SubscriptionMode::OnChange,
    QosParams::default(),
).await?;

while let Some(value) = sub.next().await {
    println!("Received: {:?}", value);
}
```

### RPC Server Example
```rust
use wind_server::RpcServer;
use wind_core::WindValue;

let server = RpcServer::new(
    "CALCULATOR".to_string(),
    "127.0.0.1:0".to_string(),
    "127.0.0.1:7001".to_string(),
);

server.register_function("add".to_string(), |params| async move {
    // Extract parameters and return result
    let result = /* calculation */;
    Ok(WindValue::F64(result))
}).await?;

server.start().await?;
```

### RPC Client Example  
```rust
use wind_client::WindClient;
use wind_core::WindValue;
use std::collections::HashMap;

let mut client = WindClient::new("127.0.0.1:7001".to_string());

let mut params = HashMap::new();
params.insert("a".to_string(), WindValue::F64(10.0));
params.insert("b".to_string(), WindValue::F64(5.0));

let result = client.call("CALCULATOR", "add", WindValue::Map(params)).await?;
println!("Result: {:?}", result);
```

## 🛠️ CLI Tools

### Service Discovery
```bash
# Discover all services
wind discover "*"

# Find temperature sensors
wind discover "SENSOR/*/TEMP"

# List all active services
wind list
```

### Data Subscription
```bash
# Subscribe to a service
wind subscribe SENSOR/ROOM_A/TEMP

# Subscribe with periodic mode
wind subscribe SENSOR/ROOM_A/TEMP --mode periodic --period-ms 1000

# Get single value
wind subscribe SENSOR/ROOM_A/TEMP --once
```

### RPC Calls
```bash
# Make RPC call
wind call CALCULATOR add '{"a": 10, "b": 5}'

# With custom timeout
wind call CALCULATOR multiply '{"a": 7, "b": 3}' --timeout-secs 10
```

## 📊 Performance

### Benchmarks
```bash
# Latency benchmark
cargo run -p wind-bench latency --samples 10000 --payload-bytes 256

# Throughput benchmark  
cargo run -p wind-bench throughput --subscribers 8 --duration-secs 10

# Load test
cargo run -p wind-bench load --services 10 --subscribers-per-service 5
```

### Expected Performance
- **Latency**: Sub-millisecond on LAN
- **Throughput**: >100k messages/sec per core
- **Scalability**: 1000+ topics per node, 10k+ total subscribers
- **Memory**: Low allocation overhead with zero-copy optimizations

## 🔧 Configuration

### Environment Variables
```bash
# Registry address
export WIND_REGISTRY_ADDR=127.0.0.1:7001

# Log level  
export RUST_LOG=wind=info

# Network interface selection
export WIND_BIND_INTERFACE=eth0
```

### Registry Configuration
```bash
cargo run -p wind-registry -- --bind 0.0.0.0:7001 --log-level debug
```

## 📈 Monitoring & Observability

### Structured Logging
```rust
use tracing::{info, warn, error};

// Automatic correlation IDs and structured fields
info!("Published temperature reading", 
    temperature = 23.5, 
    sensor_id = "TEMP_001",
    sequence = 1234
);
```

### Metrics (Planned)
- Message rates and latencies
- Connection counts and states
- Registry service counts
- Error rates by service

## 🧪 Testing

### Unit Tests
```bash
cargo test
```

### Integration Tests
```bash
cargo test --test integration_test
```

### Benchmarks
```bash
cargo bench
```

## 🐳 Docker Deployment

### Registry Service
```dockerfile
FROM rust:1.70-slim as builder
COPY . .
RUN cargo build --release -p wind-registry

FROM debian:bookworm-slim
COPY --from=builder target/release/wind-registry /usr/local/bin/
EXPOSE 7001
CMD ["wind-registry", "--bind", "0.0.0.0:7001"]
```

### Docker Compose Example
```yaml
version: '3.8'
services:
  wind-registry:
    image: wind-registry:latest
    ports:
      - "7001:7001"
    environment:
      - RUST_LOG=info
      
  temperature-sensor:
    image: wind-temperature-sensor:latest
    depends_on:
      - wind-registry
    environment:
      - WIND_REGISTRY_ADDR=wind-registry:7001
```

## 🔄 Migration from DIM

### Compatibility Matrix

| DIM Feature | WIND Equivalent | Status |
|-------------|----------------|---------|
| Name Server | Registry | ✅ Complete |
| Service Registration | Service Registration | ✅ Complete |
| Data Services | Publishers | ✅ Complete |
| Command Services | RPC Servers | ✅ Complete |
| Client Subscribe | Subscribers | ✅ Complete |
| Client Commands | RPC Clients | ✅ Complete |
| Auto-Recovery | Auto-Reconnection | ✅ Complete |
| Cross-Platform | Cross-Platform | ✅ Complete |
| Pattern Discovery | Pattern Discovery | ✅ Complete |

### Migration Steps

1. **Assess Current DIM Usage**: Identify services and clients
2. **Define WIND Schemas**: Create IDL definitions for your data types
3. **Implement Publishers**: Convert DIM servers to WIND publishers
4. **Implement Subscribers**: Convert DIM clients to WIND subscribers
5. **Test & Validate**: Use integration tests to verify behavior
6. **Deploy Gradually**: Run WIND alongside DIM during transition

### Key Differences

- **Type Safety**: WIND enforces types at compile time
- **Async I/O**: WIND uses async/await vs DIM's callback model
- **Error Handling**: WIND uses Result types vs DIM's error codes
- **Configuration**: WIND uses environment variables vs DIM's globals

## 🤝 Contributing

### Development Setup
```bash
git clone <repo>
cd wind
cargo build
cargo test
```

### Code Style
- Use `cargo fmt` for formatting
- Use `cargo clippy` for linting  
- Add tests for new features
- Update documentation

### Architecture Guidelines
- Favor composition over inheritance
- Use async/await consistently
- Handle errors explicitly with Result types
- Minimize allocations in hot paths
- Add tracing to all public APIs

## 📄 License

Licensed under Apache License, Version 2.0

## 🔗 See Also

- [DIM Protocol Analysis](docs/dim_analysis.md)
- [WIND Design Goals](docs/design_goals.md)
- [Performance Benchmarks](docs/benchmarks.md)
- [API Reference](docs/api_reference.md)
