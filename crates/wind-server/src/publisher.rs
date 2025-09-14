use std::collections::HashMap;
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{broadcast, RwLock};
use tokio::time::{interval, Duration, Instant};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use wind_core::{
    Message, MessageCodec, MessagePayload, Result, ServiceType, SubscriptionMode, WindError,
    WindValue,
};

/// Subscription tracking for a single client

#[derive(Clone, Debug)]
struct ClientSubscription {
    mode: SubscriptionMode,
    last_sent_at: Option<Instant>,
    last_sent_value: Option<WindValue>,
}

impl ClientSubscription {
    fn new(mode: SubscriptionMode) -> Self {
        Self {
            mode,
            last_sent_at: None,
            last_sent_value: None,
        }
    }

    fn should_send(&mut self, now: Instant, next: &WindValue) -> bool {
        match self.mode {
            SubscriptionMode::Once => {
                // only once if nothing has been sent yet
                self.last_sent_at.is_none()
            }
            SubscriptionMode::OnChange => {
                // send if payload changed
                self.last_sent_value.as_ref() != Some(next)
            }
            SubscriptionMode::Periodic { interval_ms } => {
                let p = Duration::from_millis(interval_ms);
                match self.last_sent_at {
                    None => true,
                    Some(ts) => now.duration_since(ts) >= p,
                }
            }
        }
    }

    fn mark_sent(&mut self, now: Instant, sent: &WindValue) {
        self.last_sent_at = Some(now);
        self.last_sent_value = Some(sent.clone());
    }
}

/// Active client connection state
#[derive(Debug)]
struct ActiveClient {
    stream: TcpStream,
    subscriptions: HashMap<String, ClientSubscription>,
}

/// High-performance publisher for WIND services
pub struct Publisher {
    service_name: String,
    bind_address: String,
    registry_address: String,
    schema_id: Option<String>,

    // Data management
    current_value: Arc<RwLock<Option<WindValue>>>,
    sequence_number: Arc<AtomicU64>,

    // Client management
    clients: Arc<RwLock<HashMap<Uuid, ActiveClient>>>,

    // Update notification
    update_tx: broadcast::Sender<WindValue>,
    _update_rx: broadcast::Receiver<WindValue>,

    // Configuration
    heartbeat_interval: Duration,
    ttl_ms: u64,
    tags: Vec<String>,
}

impl Publisher {
    /// Create a new publisher for the specified service
    pub fn new(service_name: String, bind_address: String, registry_address: String) -> Self {
        let (update_tx, update_rx) = broadcast::channel(1000);

        Self {
            service_name,
            bind_address,
            registry_address,
            schema_id: None,
            current_value: Arc::new(RwLock::new(None)),
            sequence_number: Arc::new(AtomicU64::new(0)),
            clients: Arc::new(RwLock::new(HashMap::new())),
            update_tx,
            _update_rx: update_rx,
            heartbeat_interval: Duration::from_secs(30),
            ttl_ms: 60000, // 1 minute TTL
            tags: Vec::new(),
        }
    }

    /// Set optional schema ID for type validation
    pub fn with_schema(mut self, schema_id: String) -> Self {
        self.schema_id = Some(schema_id);
        self
    }

    /// Set custom TTL for service registration
    pub fn with_ttl_ms(mut self, ttl_ms: u64) -> Self {
        self.ttl_ms = ttl_ms;
        self
    }

    /// Add tags for service discovery
    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    /// Start the publisher server
    pub async fn start(&self) -> Result<()> {
        let listener = TcpListener::bind(&self.bind_address).await?;
        let actual_address = listener.local_addr()?.to_string();

        info!(
            "Publisher '{}' listening on {}",
            self.service_name, actual_address
        );

        // Register with the registry and start heartbeat
        self.register_service(&actual_address).await?;
        self.start_heartbeat_task(actual_address.clone());

        // Start the client handler loop
        self.start_update_sender().await; // Renamed for clarity

        // Accept and handle client connections
        loop {
            match listener.accept().await {
                Ok((stream, addr)) => {
                    info!("New subscriber connected: {}", addr);
                    let client_id = Uuid::new_v4();
                    let mut clients = self.clients.write().await;
                    clients.insert(
                        client_id,
                        ActiveClient {
                            stream,
                            subscriptions: HashMap::new(),
                        },
                    );
                    // Spawn a task to handle this specific client's messages
                    self.spawn_client_listener(client_id).await;
                }
                Err(e) => {
                    error!("Failed to accept subscriber connection: {}", e);
                }
            }
        }
    }

    /// Publish a new value to all subscribers
    pub async fn publish(&self, value: WindValue) -> Result<()> {
        let seq = self.sequence_number.fetch_add(1, Ordering::SeqCst) + 1;

        // Update current value
        {
            let mut current = self.current_value.write().await;
            *current = Some(value.clone());
        }

        // Notify all clients via broadcast
        let _ = self.update_tx.send(value.clone());

        debug!(
            "Published value for '{}' with sequence {}",
            self.service_name, seq
        );

        Ok(())
    }

    /// Get the current published value
    pub async fn current_value(&self) -> Option<WindValue> {
        self.current_value.read().await.clone()
    }

    /// Get number of active subscribers
    pub async fn subscriber_count(&self) -> usize {
        self.clients.read().await.len()
    }

    async fn register_service(&self, actual_address: &str) -> Result<()> {
        let mut registry_conn = tokio::net::TcpStream::connect(&self.registry_address).await?;

        let register_msg = Message::new(MessagePayload::RegisterService {
            service: self.service_name.clone(),
            address: actual_address.to_string(),
            service_type: ServiceType::Publisher,
            schema_id: self.schema_id.clone(),
            ttl_ms: self.ttl_ms,
            tags: self.tags.clone(),
        });

        MessageCodec::write(&mut registry_conn, &register_msg).await?;
        let response = MessageCodec::decode(&mut registry_conn).await?;

        match response.payload {
            MessagePayload::ServiceRegistered { success, error, .. } => {
                if success {
                    info!(
                        "Successfully registered service '{}' with registry",
                        self.service_name
                    );
                    Ok(())
                } else {
                    Err(WindError::Registry(
                        error.unwrap_or("Registration failed".to_string()),
                    ))
                }
            }
            _ => Err(WindError::Protocol(
                "Unexpected registry response".to_string(),
            )),
        }
    }

    fn start_heartbeat_task(&self, address: String) {
        let registry_address = self.registry_address.clone();
        let service_name = self.service_name.clone();
        let ttl_ms = self.ttl_ms;
        let heartbeat_duration = self.heartbeat_interval;

        tokio::spawn(async move {
            let mut heartbeat_timer = interval(heartbeat_duration);
            loop {
                heartbeat_timer.tick().await;

                // Renew registration (simplified - would need proper renewal message)
                match tokio::net::TcpStream::connect(&registry_address).await {
                    Ok(mut conn) => {
                        let renew_msg = Message::new(MessagePayload::RegisterService {
                            service: service_name.clone(),
                            address: address.clone(),
                            service_type: ServiceType::Publisher,
                            schema_id: None,
                            ttl_ms,
                            tags: Vec::new(),
                        });

                        if let Err(e) = MessageCodec::write(&mut conn, &renew_msg).await {
                            warn!("Failed to send heartbeat: {}", e);
                        } else {
                            debug!("Sent heartbeat for service '{}'", service_name);
                        }
                    }
                    Err(e) => {
                        warn!("Failed to connect to registry for heartbeat: {}", e);
                    }
                }
            }
        });
    }

    async fn start_update_sender(&self) {
        let clients = self.clients.clone();
        let mut update_rx = self.update_tx.subscribe();
        let sequence_number = self.sequence_number.clone();

        tokio::spawn(async move {
            loop {
                let new_value = match update_rx.recv().await {
                    Ok(val) => val,
                    Err(_) => continue, // Channel lagged or closed
                };
                let seq = sequence_number.load(Ordering::SeqCst);

                let mut clients_guard = clients.write().await;
                let mut clients_to_remove = Vec::new();

                for (client_id, client) in clients_guard.iter_mut() {
                    for (service, subscription) in client.subscriptions.iter_mut() {
                        if subscription.should_send(Instant::now(), &new_value) {
                            let publish_msg = Message::new(MessagePayload::Publish {
                                service: service.clone(),
                                sequence: seq,
                                value: new_value.clone(),
                                schema_id: None,
                            });

                            match MessageCodec::write(&mut client.stream, &publish_msg).await {
                                Ok(()) => {
                                    subscription.mark_sent(Instant::now(), &new_value);
                                    debug!("Sent update to client {}", client_id);
                                }
                                Err(e) => {
                                    warn!("Failed to send to client {}: {}", client_id, e);
                                    clients_to_remove.push(*client_id);
                                    break; // Stop trying to send to this broken client
                                }
                            }
                        }
                    }
                }

                for client_id in clients_to_remove {
                    clients_guard.remove(&client_id);
                    info!("Removed disconnected client {}", client_id);
                }
            }
        });
    }

    async fn spawn_client_listener(&self, client_id: Uuid) {
        let clients = self.clients.clone();
        let current_value = self.current_value.clone();

        tokio::spawn(async move {
            // The stream is inside the client map, so we need to lock it to read
            // This is a bit tricky. A better design would be to pass the stream here.
            // For now, we'll read one message and then the update loop takes over.
            let mut clients_guard = clients.write().await;
            let client = if let Some(c) = clients_guard.get_mut(&client_id) {
                c
            } else {
                return; // Client disconnected before we could handle it
            };

            let msg = match MessageCodec::decode(&mut client.stream).await {
                Ok(m) => m,
                Err(e) => {
                    warn!(
                        "Failed to decode subscription from client {}: {}",
                        client_id, e
                    );
                    clients_guard.remove(&client_id);
                    return;
                }
            };

            match msg.payload {
                MessagePayload::Subscribe { service, mode, .. } => {
                    client
                        .subscriptions
                        .insert(service, ClientSubscription::new(mode));

                    let ack = Message::new(MessagePayload::SubscribeAck {
                        subscription_id: client_id,
                        success: true,
                        error: None,
                        current_value: current_value.read().await.clone(),
                    });

                    if let Err(e) = MessageCodec::write(&mut client.stream, &ack).await {
                        warn!("Failed to send SubscribeAck to client {}: {}", client_id, e);
                        clients_guard.remove(&client_id);
                    } else {
                        info!("Client {} subscribed successfully", client_id);
                    }
                }
                _ => {
                    warn!(
                        "Unexpected message from client {}: {:?}",
                        client_id, msg.payload
                    );
                    clients_guard.remove(&client_id);
                }
            }
        });
    }
}
