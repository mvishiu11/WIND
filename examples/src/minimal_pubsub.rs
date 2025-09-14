use std::sync::Arc;
use tokio::time::{sleep, Duration};
use tracing::error;
use wind_client::WindClient;
use wind_core::WindValue;
use wind_registry::RegistryServer;
use wind_server::Publisher;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Start registry
    let registry_addr = "127.0.0.1:7015";
    let registry = RegistryServer::new(registry_addr.to_string());
    tokio::spawn(async move {
        if let Err(e) = registry.run().await {
            error!("Registry error: {}", e);
        }
    });
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Start publisher
    let publisher = Arc::new(Publisher::new(
        "TEST/MINIMAL".to_string(),
        "127.0.0.1:0".to_string(),
        registry_addr.to_string(),
    ));
    let pub_ref = publisher.clone();
    tokio::spawn(async move {
        pub_ref.start().await.unwrap();
    });
    sleep(Duration::from_millis(200)).await;

    // Start subscriber
    let mut client = WindClient::new(registry_addr.to_string());
    let mut sub = client.subscribe("TEST/MINIMAL").await?;

    // Publish a value
    publisher
        .publish(WindValue::String("Hello, WIND!".to_string()))
        .await?;

    // Receive the value
    if let Some(val) = sub.next().await {
        println!("Received: {:?}", val);
    } else {
        println!("No value received");
    }

    Ok(())
}
