use tokio::time::Duration;
use tracing::info;
// use uuid::Uuid;
// use std::collections::HashMap;
// use std::sync::Arc;
// use tokio::sync::{oneshot, RwLock};

use crate::{Connection, Subscriber};
use wind_core::{Message, MessagePayload, Result, WindError, WindValue};

/// Pending RPC call tracking
// #[derive(Debug)]
// struct PendingCall {
//     sender: oneshot::Sender<Result<WindValue>>,
//     timeout_handle: tokio::task::JoinHandle<()>,
// }

/// RPC client for making type-safe remote procedure calls
pub struct RpcClient {
    subscriber: Subscriber,
    // connections: Arc<RwLock<HashMap<String, Connection>>>,
    // pending_calls: Arc<RwLock<HashMap<Uuid, PendingCall>>>,
}

impl RpcClient {
    pub fn new(registry_address: String) -> Self {
        Self {
            subscriber: Subscriber::new(registry_address),
            // connections: Arc::new(RwLock::new(HashMap::new())),
            // pending_calls: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Make a type-safe RPC call with timeout
    pub async fn call(
        &mut self,
        service_name: &str,
        method: &str,
        params: WindValue,
        _timeout_duration: Duration,
    ) -> Result<WindValue> {
        let service_info = self.subscriber.discover_service(service_name).await?;
        let mut connection = Connection::new(service_info.address);
        connection.connect().await?;

        let call_msg = Message::new(MessagePayload::RpcCall {
            service: service_name.to_string(),
            method: method.to_string(),
            params,
            schema_id: service_info.schema_id,
        });

        connection.send(&call_msg).await?;
        let response = connection.receive().await?;

        match response.payload {
            MessagePayload::RpcResponse { result, .. } => {
                result.map_err(|e| WindError::Protocol(e))
            }
            _ => Err(WindError::Protocol("Unexpected response".to_string())),
        }
    }

    /// Make an async RPC call (fire-and-forget)
    pub async fn call_async(
        &mut self,
        service_name: &str,
        method: &str,
        params: WindValue,
    ) -> Result<()> {
        let service_info = self.subscriber.discover_service(service_name).await?;
        let mut connection = Connection::new(service_info.address);
        connection.connect().await?;

        let call_msg = Message::new(MessagePayload::RpcCall {
            service: service_name.to_string(),
            method: method.to_string(),
            params,
            schema_id: service_info.schema_id,
        });

        connection.send(&call_msg).await?;
        info!("Sent async RPC call to {}::{}", service_name, method);

        Ok(())
    }
}
