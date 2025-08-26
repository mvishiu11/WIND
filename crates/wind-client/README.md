# wind-client  

Client library for subscribing to WIND services and making RPC calls.

## Features

- **Type-Safe Subscriptions**: Compile-time type checking
- **Auto-Reconnection**: Handles network failures gracefully  
- **Multiple Modes**: Once, Periodic, and OnChange subscriptions
- **RPC Client**: Type-safe remote procedure calls
- **Pattern Discovery**: Find services using glob patterns

## Examples

### Simple Subscription
```rust
use wind_client::WindClient;

let mut client = WindClient::new("127.0.0.1:7001".to_string());
let mut sub = client.subscribe("SENSOR/TEMP").await?;

while let Some(value) = sub.next().await {
    println!("Temperature: {:?}", value);
}
```

### RPC Call
```rust
let result = client.call("CALCULATOR", "add", params).await?;
```
