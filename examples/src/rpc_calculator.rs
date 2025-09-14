use std::collections::HashMap;
use tokio::time::Duration;
use tracing::{error, info};
use wind_client::WindClient;
use wind_core::{Result, WindValue};
use wind_registry::RegistryServer;
use wind_server::RpcServer;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt().with_env_filter("info").init();

    let registry_addr = "127.0.0.1:7015";

    // Start registry
    let registry = RegistryServer::new(registry_addr.to_string());
    tokio::spawn(async move {
        if let Err(e) = registry.run().await {
            error!("Registry error: {}", e);
        }
    });
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Create calculator RPC server
    let calc_server = RpcServer::new(
        "CALCULATOR".to_string(),
        "127.0.0.1:0".to_string(),
        registry_addr.to_string(),
    );

    // Register calculator methods
    calc_server
        .register_function("add".to_string(), |params| async move {
            if let WindValue::Map(map) = params {
                let a = extract_f64(&map, "a")?;
                let b = extract_f64(&map, "b")?;
                Ok(WindValue::F64(a + b))
            } else {
                Err(wind_core::WindError::TypeMismatch {
                    expected: "Map with 'a' and 'b' fields".to_string(),
                    actual: format!("{:?}", params),
                })
            }
        })
        .await?;

    calc_server
        .register_function("multiply".to_string(), |params| async move {
            if let WindValue::Map(map) = params {
                let a = extract_f64(&map, "a")?;
                let b = extract_f64(&map, "b")?;
                Ok(WindValue::F64(a * b))
            } else {
                Err(wind_core::WindError::TypeMismatch {
                    expected: "Map with 'a' and 'b' fields".to_string(),
                    actual: format!("{:?}", params),
                })
            }
        })
        .await?;

    // Start the RPC server
    tokio::spawn(async move {
        if let Err(e) = calc_server.start().await {
            error!("RPC server error: {}", e);
        }
    });

    tokio::time::sleep(Duration::from_secs(2)).await;

    // Test the calculator with RPC calls
    let mut client = WindClient::new(registry_addr.to_string());

    info!("Testing calculator RPC service...");

    // Test addition
    let mut add_params = HashMap::new();
    add_params.insert("a".to_string(), WindValue::F64(10.0));
    add_params.insert("b".to_string(), WindValue::F64(5.0));

    let result = client
        .call("CALCULATOR", "add", WindValue::Map(add_params))
        .await?;
    info!("10 + 5 = {:?}", result);

    // Test multiplication
    let mut mul_params = HashMap::new();
    mul_params.insert("a".to_string(), WindValue::F64(7.0));
    mul_params.insert("b".to_string(), WindValue::F64(3.0));

    let result = client
        .call("CALCULATOR", "multiply", WindValue::Map(mul_params))
        .await?;
    info!("7 * 3 = {:?}", result);

    info!("RPC demo completed");
    Ok(())
}

fn extract_f64(map: &HashMap<String, WindValue>, key: &str) -> Result<f64> {
    map.get(key)
        .and_then(|v| {
            if let WindValue::F64(n) = v {
                Some(*n)
            } else {
                None
            }
        })
        .ok_or_else(|| wind_core::WindError::TypeMismatch {
            expected: format!("f64 field '{}'", key),
            actual: "missing or wrong type".to_string(),
        })
}
