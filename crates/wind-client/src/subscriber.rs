use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, oneshot, RwLock};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::Connection;
use wind_core::{
    Message, MessagePayload, QosParams, Result, ServiceInfo, SubscriptionMode, WindError, WindValue,
};

/// Subscription handle for managing individual subscriptions
#[derive(Debug)]
pub struct Subscription {
    pub id: Uuid,
    pub service_name: String,
    pub mode: SubscriptionMode,
    pub qos: QosParams,
    pub receiver: broadcast::Receiver<WindValue>,
    cancel_sender: oneshot::Sender<()>,
}

impl Subscription {
    pub async fn next(&mut self) -> Option<WindValue> {
        match self.receiver.recv().await {
            Ok(value) => Some(value),
            Err(_) => None,
        }
    }

    pub fn cancel(self) {
        let _ = self.cancel_sender.send(());
    }
}

/// High-level subscriber client with automatic reconnection and type safety
pub struct Subscriber {
    active_subscriptions: Arc<RwLock<HashMap<Uuid, (String, broadcast::Sender<WindValue>)>>>,
    registry_connection: Connection,
}

impl Subscriber {
    pub fn new(registry_address: String) -> Self {
        Self {
            active_subscriptions: Arc::new(RwLock::new(HashMap::new())),
            registry_connection: Connection::new(registry_address),
        }
    }

    /// Subscribe to a service with type-safe value delivery
    pub async fn subscribe(
        &mut self,
        service_name: &str,
        mode: SubscriptionMode,
        qos: QosParams,
    ) -> Result<Subscription> {
        // First, discover the service
        let service_info = self.discover_service(service_name).await?;

        // Connect to the service provider
        let mut service_connection = Connection::new(service_info.address);
        service_connection.connect().await?;

        // Create broadcast channel for this subscription
        let (tx, rx) = broadcast::channel(qos.max_queue_size as usize);
        let subscription_id = Uuid::new_v4();

        // Send subscription request
        let subscribe_msg = Message::new(MessagePayload::Subscribe {
            service: service_name.to_string(),
            mode: mode.clone(),
            qos: qos.clone(),
            schema_id: service_info.schema_id.clone(),
        });

        service_connection.send(&subscribe_msg).await?;

        // Wait for subscription acknowledgment
        let ack_msg = service_connection.receive().await?;
        match ack_msg.payload {
            MessagePayload::SubscribeAck {
                subscription_id: _ack_id,
                success,
                error,
                current_value,
            } => {
                if !success {
                    return Err(WindError::Protocol(
                        error.unwrap_or("Subscription failed".to_string()),
                    ));
                }

                // Send current value if available
                if let Some(value) = current_value {
                    let _ = tx.send(value);
                }

                info!("Successfully subscribed to service: {}", service_name);
            }
            _ => {
                return Err(WindError::Protocol(
                    "Expected SubscribeAck message".to_string(),
                ));
            }
        }

        // Store subscription info
        {
            let mut subs = self.active_subscriptions.write().await;
            subs.insert(subscription_id, (service_name.to_string(), tx.clone()));
        }

        // Create cancel channel
        let (cancel_tx, mut cancel_rx) = oneshot::channel();

        // Spawn background task to handle incoming data
        let subs_map = self.active_subscriptions.clone();
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    // Handle cancellation
                    _ = &mut cancel_rx => {
                        debug!("Subscription {} cancelled", subscription_id);
                        break;
                    }

                    // Handle incoming messages
                    msg_result = service_connection.receive() => {
                        match msg_result {
                            Ok(msg) => {
                                match msg.payload {
                                    MessagePayload::Publish { value, .. } => {
                                        if let Err(_) = tx.send(value) {
                                            warn!("No active receivers for subscription {}", subscription_id);
                                        }
                                    }
                                    MessagePayload::Error { error, .. } => {
                                        error!("Service error: {}", error);
                                        break;
                                    }
                                    _ => {
                                        debug!("Unexpected message: {:?}", msg.payload);
                                    }
                                }
                            }
                            Err(e) => {
                                error!("Connection error: {}. Attempting to reconnect...", e);
                                // TODO: Implement reconnection logic here
                                break;
                            }
                        }
                    }
                }
            }

            // Cleanup subscription
            let mut subs = subs_map.write().await;
            subs.remove(&subscription_id);
        });

        Ok(Subscription {
            id: subscription_id,
            service_name: service_name.to_string(),
            mode,
            qos,
            receiver: rx,
            cancel_sender: cancel_tx,
        })
    }

    /// Discover a specific service by name
    pub async fn discover_service(&mut self, service_name: &str) -> Result<ServiceInfo> {
        self.registry_connection.connect().await?;

        let discover_msg = Message::new(MessagePayload::DiscoverServices {
            pattern: service_name.to_string(), // Exact match
        });

        self.registry_connection.send(&discover_msg).await?;
        let response = self.registry_connection.receive().await?;

        match response.payload {
            MessagePayload::ServicesDiscovered { services } => {
                if let Some(service) = services.into_iter().find(|s| s.name == service_name) {
                    Ok(service)
                } else {
                    Err(WindError::ServiceNotFound(service_name.to_string()))
                }
            }
            MessagePayload::Error { error, .. } => Err(WindError::Registry(error)),
            _ => Err(WindError::Protocol("Unexpected response".to_string())),
        }
    }

    /// Discover services matching a pattern
    pub async fn discover_services(&mut self, pattern: &str) -> Result<Vec<ServiceInfo>> {
        self.registry_connection.connect().await?;

        let discover_msg = Message::new(MessagePayload::DiscoverServices {
            pattern: pattern.to_string(),
        });

        self.registry_connection.send(&discover_msg).await?;
        let response = self.registry_connection.receive().await?;

        match response.payload {
            MessagePayload::ServicesDiscovered { services } => Ok(services),
            MessagePayload::Error { error, .. } => Err(WindError::Registry(error)),
            _ => Err(WindError::Protocol("Unexpected response".to_string())),
        }
    }

    /// Get the number of active subscriptions
    pub async fn subscription_count(&self) -> usize {
        self.active_subscriptions.read().await.len()
    }
}
