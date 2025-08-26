use tokio::time::{sleep, Duration};
use tracing::{error, info};
use wind_client::WindClient;
use wind_core::{QosParams, SubscriptionMode, WindValue};

pub async fn discover(registry: &str, pattern: &str, json: bool) -> anyhow::Result<()> {
    let mut client = WindClient::new(registry.to_string());
    let services = client.discover(pattern).await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&services)?);
    } else {
        if services.is_empty() {
            println!("No services found matching pattern: {}", pattern);
        } else {
            println!(
                "Found {} service(s) matching '{}':",
                services.len(),
                pattern
            );
            for service in services {
                println!(
                    "  {} -> {} ({})",
                    service.name,
                    service.address,
                    format!("{:?}", service.service_type)
                );
                if let Some(schema) = &service.schema_id {
                    println!("    Schema: {}", schema);
                }
                if !service.tags.is_empty() {
                    println!("    Tags: {}", service.tags.join(", "));
                }
            }
        }
    }

    Ok(())
}

pub async fn subscribe(
    registry: &str,
    service: &str,
    mode: &str,
    period_ms: Option<u64>,
    once: bool,
) -> anyhow::Result<()> {
    let mut client = WindClient::new(registry.to_string());

    let subscription_mode = if once {
        SubscriptionMode::Once
    } else {
        match mode {
            "on-change" => SubscriptionMode::OnChange,
            "periodic" => SubscriptionMode::Periodic {
                interval_ms: period_ms.unwrap_or(1000),
            },
            _ => {
                error!("Invalid mode: {}. Use 'on-change' or 'periodic'", mode);
                return Ok(());
            }
        }
    };

    info!(
        "Subscribing to service '{}' with mode {:?}",
        service, subscription_mode
    );

    let mut subscription = client
        .subscribe_with_options(service, subscription_mode, QosParams::default())
        .await?;

    println!(
        "Subscribed to '{}'. Waiting for data... (Ctrl+C to stop)",
        service
    );

    while let Some(value) = subscription.next().await {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis();
        println!("[{}] {}: {:?}", timestamp, service, value);

        if once {
            break;
        }
    }

    Ok(())
}

pub async fn call(
    registry: &str,
    service: &str,
    method: &str,
    params: &str,
    timeout_secs: u64,
) -> anyhow::Result<()> {
    let mut client = WindClient::new(registry.to_string());

    // Parse JSON parameters
    let params_value: serde_json::Value = serde_json::from_str(params)?;
    let wind_params = json_to_wind_value(params_value);

    info!(
        "Calling {}::{} with params: {:?}",
        service, method, wind_params
    );

    let result = client
        .call_with_timeout(
            service,
            method,
            wind_params,
            Duration::from_secs(timeout_secs),
        )
        .await?;

    println!("RPC result: {:?}", result);
    Ok(())
}

pub async fn list(registry: &str, json: bool) -> anyhow::Result<()> {
    let mut client = WindClient::new(registry.to_string());
    let services = client.discover("*").await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&services)?);
    } else {
        println!("Active services ({}):", services.len());
        for service in services {
            println!(
                "  {} -> {} ({:?})",
                service.name, service.address, service.service_type
            );
        }
    }

    Ok(())
}

pub async fn publish(
    registry: &str,
    service: &str,
    value: &str,
    repeat: Option<u64>,
    interval_ms: u64,
) -> anyhow::Result<()> {
    // This is a simplified publish command that would work with a test publisher
    // In practice, this would need to start a temporary publisher
    info!("Publish command not fully implemented yet");
    println!("Would publish to '{}': {}", service, value);

    if let Some(count) = repeat {
        println!(
            "Would repeat {} times with {}ms intervals",
            count, interval_ms
        );
    }

    Ok(())
}

fn json_to_wind_value(json: serde_json::Value) -> WindValue {
    use serde_json::Value;
    match json {
        Value::Null => WindValue::String("null".to_string()),
        Value::Bool(b) => WindValue::Bool(b),
        Value::Number(n) => {
            if n.is_i64() {
                WindValue::I64(n.as_i64().unwrap())
            } else if n.is_f64() {
                WindValue::F64(n.as_f64().unwrap())
            } else {
                WindValue::String(n.to_string())
            }
        }
        Value::String(s) => WindValue::String(s),
        Value::Array(arr) => WindValue::Array(arr.into_iter().map(json_to_wind_value).collect()),
        Value::Object(obj) => {
            let mut map = std::collections::HashMap::new();
            for (k, v) in obj {
                map.insert(k, json_to_wind_value(v));
            }
            WindValue::Map(map)
        }
    }
}
