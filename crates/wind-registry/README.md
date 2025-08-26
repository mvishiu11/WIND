# wind-registry

Service discovery registry for WIND protocol with pattern matching support.

## Features

- **Pattern Matching**: Discover services using glob patterns (`SENSOR/*/TEMP`)
- **Service Registration**: Automatic registration with TTL and heartbeat
- **High Availability**: Cleanup of expired services
- **Metrics**: Built-in statistics and monitoring

## Usage

### Start Registry
```bash
cargo run -p wind-registry -- --bind 127.0.0.1:7001
```

### API

Services register themselves:
```rust
let registry = RegistryServer::new("127.0.0.1:7001".to_string());
registry.run().await?;
```

Clients discover services:
```rust
let services = registry.discover_services("SENSOR/*/TEMP").await?;
```
