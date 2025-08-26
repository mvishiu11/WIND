# wind-server

Server library for publishing data and serving RPC calls in WIND protocol.

## Features

- **High-Performance Publishing**: Async I/O with efficient fan-out
- **RPC Server**: Type-safe method registration and handling
- **Auto-Registration**: Automatic service discovery registration
- **QoS Support**: Reliability and durability options
- **Heartbeat**: Automatic service renewal

## Examples

### Publisher
```rust
use wind_server::Publisher;

let publisher = Publisher::new(
    "SENSOR/TEMP".to_string(),
    "127.0.0.1:0".to_string(),
    "127.0.0.1:7001".to_string(),
);

publisher.start().await?;
publisher.publish(WindValue::F64(23.5)).await?;
```

### RPC Server
```rust
use wind_server::RpcServer;

let server = RpcServer::new(
    "CALCULATOR".to_string(),
    "127.0.0.1:0".to_string(),
    "127.0.0.1:7001".to_string(),
);

server.register_function("add", |params| async {
    // Handle addition
    Ok(WindValue::F64(result))
}).await?;

server.start().await?;
```
