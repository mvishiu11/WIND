use std::collections::HashMap;
use std::sync::Arc;
use tokio::time::{interval, Duration};
use tracing::{error, info};
use wind_client::WindClient;
use wind_core::{QosParams, SubscriptionMode, WindValue};
use wind_registry::RegistryServer;
use wind_server::Publisher;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt().with_env_filter("info").init();

    // Start the registry in the background
    let registry_addr = "127.0.0.1:7015";
    let registry = RegistryServer::new(registry_addr.to_string());
    tokio::spawn(async move {
        if let Err(e) = registry.run().await {
            error!("Registry error: {}", e);
        }
    });

    // Give registry time to start
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Start temperature sensor publisher
    let publisher = Arc::new(
        Publisher::new(
            "SENSOR/ROOM_A/TEMPERATURE".to_string(),
            "127.0.0.1:0".to_string(),
            registry_addr.to_string(),
        )
        .with_tags(vec!["sensor".to_string(), "temperature".to_string()]),
    );

    let _pub_handle = {
        let pub_ref = publisher.clone();
        tokio::spawn(async move {
            if let Err(e) = pub_ref.start().await {
                error!("Publisher error: {}", e);
            }
        })
    };

    // Start publishing temperature data
    let data_publisher = publisher.clone();
    tokio::spawn(async move {
        let mut temp_interval = interval(Duration::from_secs(1));
        let mut temperature = 20.0f64;
        let mut seq = 0u64;

        loop {
            temp_interval.tick().await;

            // Simulate temperature variation
            temperature += (rand::random::<f64>() - 0.5) * 2.0;
            temperature = temperature.max(15.0).min(30.0);

            // Create temperature reading with multiple fields
            let mut reading = HashMap::new();
            reading.insert("temperature".to_string(), WindValue::F64(temperature));
            reading.insert(
                "timestamp".to_string(),
                WindValue::I64(
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs() as i64,
                ),
            );
            reading.insert(
                "sensor_id".to_string(),
                WindValue::String("TEMP_001".to_string()),
            );
            reading.insert("sequence".to_string(), WindValue::I64(seq as i64));

            let value = WindValue::Map(reading);

            if let Err(e) = data_publisher.publish(value).await {
                error!("Failed to publish: {}", e);
            } else {
                info!("Published temperature: {:.2}°C (seq: {})", temperature, seq);
            }

            seq += 1;
        }
    });

    // Give publisher time to register
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Start a subscriber
    let mut client = WindClient::new(registry_addr.to_string());

    // Discover available sensors
    info!("Discovering sensors...");
    let services = client.discover("SENSOR/*/TEMPERATURE").await?;
    info!("Found {} temperature sensors", services.len());
    for service in &services {
        info!("  - {}", service.name);
    }

    // Subscribe to temperature updates
    info!("Subscribing to temperature sensor...");
    let mut subscription = client
        .subscribe_with_options(
            "SENSOR/ROOM_A/TEMPERATURE",
            SubscriptionMode::OnChange,
            QosParams {
                reliability: wind_core::ReliabilityLevel::Reliable,
                durability: true,
                max_queue_size: 100,
            },
        )
        .await?;

    info!("Receiving temperature data... (Ctrl+C to stop)");

    let mut last_temp = 0.0f64;
    let mut sample_count = 0u64;
    let start_time = std::time::Instant::now();

    // Process incoming temperature data
    while let Some(value) = subscription.next().await {
        sample_count += 1;

        if let WindValue::Map(reading) = value {
            let temp = if let Some(WindValue::F64(t)) = reading.get("temperature") {
                *t
            } else {
                continue;
            };

            let _timestamp = if let Some(WindValue::I64(ts)) = reading.get("timestamp") {
                *ts
            } else {
                0
            };

            let sensor_id = if let Some(WindValue::String(id)) = reading.get("sensor_id") {
                id.clone()
            } else {
                "unknown".to_string()
            };

            let change = temp - last_temp;
            last_temp = temp;

            info!(
                "🌡️  {} | {:.2}°C | Change: {:.2}°C | Samples: {} | Rate: {:.1}/s",
                sensor_id,
                temp,
                change,
                sample_count,
                sample_count as f64 / start_time.elapsed().as_secs_f64()
            );

            // Stop after 30 seconds for demo
            if start_time.elapsed().as_secs() > 30 {
                info!("Demo completed after 30 seconds");
                break;
            }
        }
    }

    info!("Shutting down...");
    Ok(())
}
