use tokio::time::{timeout, Duration};
use wind_core::{WindValue, SubscriptionMode, QosParams};
use wind_client::WindClient;
use wind_server::Publisher;
use wind_registry::RegistryServer;
use std::sync::Arc;

#[tokio::test]
async fn test_basic_pub_sub() {
    let _ = tracing_subscriber::fmt().try_init();
    
    let registry_addr = "127.0.0.1:7010";

    // Start registry
    let registry = RegistryServer::new(registry_addr.to_string());
    tokio::spawn(async move {
        let _ = registry.run().await;
    });
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Start publisher
    let publisher = Arc::new(Publisher::new(
        "TEST/SERVICE".to_string(),
        "127.0.0.1:0".to_string(),
        registry_addr.to_string(),
    ));

    tokio::spawn({
        let pub_ref = publisher.clone();
        async move {
            let _ = pub_ref.start().await;
        }
    });
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Create subscriber
    let mut client = WindClient::new(registry_addr.to_string());
    let mut subscription = client.subscribe("TEST/SERVICE").await.unwrap();

    // Publish a value
    publisher.publish(WindValue::String("Hello WIND!".to_string())).await.unwrap();

    // Receive the value
    let received = timeout(Duration::from_secs(5), subscription.next()).await
        .expect("Timeout waiting for message")
        .expect("Expected message");

    assert_eq!(received, WindValue::String("Hello WIND!".to_string()));
}

#[tokio::test]
async fn test_service_discovery() {
    let _ = tracing_subscriber::fmt().try_init();
    
    let registry_addr = "127.0.0.1:7011";

    // Start registry
    let registry = RegistryServer::new(registry_addr.to_string());
    tokio::spawn(async move {
        let _ = registry.run().await;
    });
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Start multiple services
    let services = vec![
        ("SENSOR/ROOM_A/TEMP", vec!["sensor", "temperature"]),
        ("SENSOR/ROOM_B/TEMP", vec!["sensor", "temperature"]),
        ("DETECTOR/HALL_1/STATUS", vec!["detector", "status"]),
    ];

    for (service_name, tags) in services {
        let publisher = Arc::new(Publisher::new(
            service_name.to_string(),
            "127.0.0.1:0".to_string(),
            registry_addr.to_string(),
        ).with_tags(tags.into_iter().map(String::from).collect()));

        tokio::spawn(async move {
            let _ = publisher.start().await;
        });
    }
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Test discovery patterns
    let mut client = WindClient::new(registry_addr.to_string());
    
    // Discover all temperature sensors
    let temp_services = client.discover("SENSOR/*/TEMP").await.unwrap();
    assert_eq!(temp_services.len(), 2);
    
    // Discover all services in SENSOR namespace
    let sensor_services = client.discover("SENSOR/*").await.unwrap();
    assert_eq!(sensor_services.len(), 2);
    
    // Discover all services
    let all_services = client.discover("*").await.unwrap();
    assert_eq!(all_services.len(), 3);
}

#[tokio::test]
async fn test_type_safety() {
    // Test WindValue conversions
    let bool_val: WindValue = true.into();
    assert_eq!(bool_val, WindValue::Bool(true));
    
    let converted_bool: bool = bool_val.try_into().unwrap();
    assert_eq!(converted_bool, true);
    
    let int_val: WindValue = 42i32.into();
    let converted_int: i32 = int_val.try_into().unwrap();
    assert_eq!(converted_int, 42);
    
    let string_val: WindValue = "test".into();
    let converted_string: String = string_val.try_into().unwrap();
    assert_eq!(converted_string, "test");
    
    // Test type mismatch error
    let bool_val = WindValue::Bool(true);
    let result: Result<i32, _> = bool_val.try_into();
    assert!(result.is_err());
}

#[tokio::test] 
async fn test_subscription_modes() {
    let _ = tracing_subscriber::fmt().try_init();
    
    let registry_addr = "127.0.0.1:7012";

    // Start registry  
    let registry = RegistryServer::new(registry_addr.to_string());
    tokio::spawn(async move {
        let _ = registry.run().await;
    });
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Start publisher
    let publisher = Arc::new(Publisher::new(
        "TEST/MODES".to_string(),
        "127.0.0.1:0".to_string(),
        registry_addr.to_string(),
    ));

    tokio::spawn({
        let pub_ref = publisher.clone();
        async move {
            let _ = pub_ref.start().await;
        }
    });
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Test ONCE mode
    let mut client = WindClient::new(registry_addr.to_string());
    let mut once_sub = client.subscribe_with_options(
        "TEST/MODES",
        SubscriptionMode::Once,
        QosParams::default(),
    ).await.unwrap();

    // Publish initial value
    publisher.publish(WindValue::I32(1)).await.unwrap();
    
    // Should receive exactly one value
    let value = timeout(Duration::from_secs(2), once_sub.next()).await.unwrap().unwrap();
    assert_eq!(value, WindValue::I32(1));

    // Test periodic mode
    let mut periodic_sub = client.subscribe_with_options(
        "TEST/MODES", 
        SubscriptionMode::Periodic { interval_ms: 100 },
        QosParams::default(),
    ).await.unwrap();

    // Should receive periodic updates even if value doesn't change
    let _val1 = timeout(Duration::from_millis(200), periodic_sub.next()).await.unwrap();
    let _val2 = timeout(Duration::from_millis(200), periodic_sub.next()).await.unwrap();
}
