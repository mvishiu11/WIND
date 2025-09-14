use hdrhistogram::Histogram;
use std::sync::Arc;
use tokio::time::{Duration, Instant};
use tracing::warn;
use rand::RngCore;
use std::collections::HashMap;
use wind_client::WindClient;
use wind_core::{QosParams, SubscriptionMode, WindValue};
use wind_registry::RegistryServer;
use wind_server::Publisher;

pub async fn run(
    registry_addr: &str,
    samples: usize,
    payload_bytes: usize,
    duration_secs: u64,
) -> anyhow::Result<()> {
    println!("=== WIND Latency Benchmark ===");
    println!("Registry: {}", registry_addr);
    println!("Samples: {}", samples);
    println!("Payload size: {} bytes", payload_bytes);
    println!("Duration: {} seconds", duration_secs);
    println!();

    // Start registry
    let registry = Arc::new(RegistryServer::new(registry_addr.to_string()));
    let registry_handle = {
        let registry = registry.clone();
        tokio::spawn(async move {
            if let Err(e) = registry.run().await {
                warn!("Registry error: {}", e);
            }
        })
    };

    // Give registry time to start
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Start publisher
    let publisher = Arc::new(Publisher::new(
        "BENCH/LATENCY".to_string(),
        "127.0.0.1:0".to_string(),
        registry_addr.to_string(),
    ));

    let publisher_handle = {
        let pub_ref = publisher.clone();

        tokio::spawn(async move {
            if let Err(e) = pub_ref.start().await {
                warn!("Publisher error: {}", e);
            }
        })
    };

    // Give publisher time to register
    tokio::time::sleep(Duration::from_millis(1000)).await;

    // Start subscriber
    let mut client = WindClient::new(registry_addr.to_string());
    let mut subscription = client
        .subscribe_with_options(
            "BENCH/LATENCY",
            SubscriptionMode::OnChange,
            QosParams::default(),
        )
        .await?;

    // Start latency measurement
    let mut histogram = Histogram::<u64>::new(3)?;
    let mut samples_collected = 0;
    let start_time = Instant::now();
    let test_duration = Duration::from_secs(duration_secs);

    // Spawn publisher task
    let publisher_ref = publisher.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_millis(10)); // 100 Hz

        loop {
            interval.tick().await;

            let mut payload = vec![0u8; payload_bytes];
            rand::thread_rng().fill_bytes(&mut payload);

            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_micros() as i64;

            let mut value_map = HashMap::new();
            value_map.insert("timestamp".to_string(), WindValue::I64(now));
            value_map.insert("data".to_string(), WindValue::Bytes(payload));

            if let Err(e) = publisher_ref.publish(WindValue::Map(value_map)).await {
                warn!("Publish error: {}", e);
            }
        }
    });

    // Collect latency samples
    while samples_collected < samples && start_time.elapsed() < test_duration {
        if let Some(value) = subscription.next().await {
            if let WindValue::Map(received_map) = value {
                if let Some(WindValue::I64(sent_ts)) = received_map.get("timestamp") {
                    let now = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_micros() as i64;
                    let latency_us = (now - sent_ts) as u64;
                    histogram.record(latency_us).unwrap_or_else(|e| {
                        warn!("Failed to record latency: {}", e);
                    });
                    samples_collected += 1;

                    if samples_collected % 1000 == 0 {
                        println!("Collected {} samples...", samples_collected);
                    }
                }
            }
        }
    }

    // Print results
    let duration = start_time.elapsed().as_secs_f64();
    println!("\n=== Latency Results ===");
    println!("Test duration: {:.2}s", duration);
    println!("Samples collected: {}", samples_collected);
    println!("Sample rate: {:.0} Hz", samples_collected as f64 / duration);
    println!();
    println!("Latency distribution (microseconds):");
    println!("  Min:  {}", histogram.min());
    println!("  p50:  {}", histogram.value_at_quantile(0.50));
    println!("  p90:  {}", histogram.value_at_quantile(0.90));
    println!("  p95:  {}", histogram.value_at_quantile(0.95));
    println!("  p99:  {}", histogram.value_at_quantile(0.99));
    println!("  p99.9:{}", histogram.value_at_quantile(0.999));
    println!("  Max:  {}", histogram.max());

    Ok(())
}
