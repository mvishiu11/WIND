use crate::{QosParams, SubscriptionMode, WindValue};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// WIND protocol messages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: Uuid,
    pub timestamp_us: u64,
    pub payload: MessagePayload,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessagePayload {
    // Registry messages
    RegisterService {
        service: String,
        address: String,
        service_type: crate::ServiceType,
        schema_id: Option<String>,
        ttl_ms: u64,
        tags: Vec<String>,
    },
    ServiceRegistered {
        service: String,
        success: bool,
        error: Option<String>,
    },

    DiscoverServices {
        pattern: String, // Glob pattern like "SENSOR/*/TEMP"
    },
    ServicesDiscovered {
        services: Vec<crate::ServiceInfo>,
    },

    // Subscription messages
    Subscribe {
        service: String,
        mode: SubscriptionMode,
        qos: QosParams,
        schema_id: Option<String>,
    },
    SubscribeAck {
        subscription_id: Uuid,
        success: bool,
        error: Option<String>,
        current_value: Option<WindValue>,
    },

    Unsubscribe {
        subscription_id: Uuid,
    },

    // Data messages
    Publish {
        service: String,
        sequence: u64,
        value: WindValue,
        schema_id: Option<String>,
    },

    // RPC messages
    RpcCall {
        service: String,
        method: String,
        params: WindValue,
        schema_id: Option<String>,
    },
    RpcResponse {
        call_id: Uuid,
        result: Result<WindValue, String>,
        schema_id: Option<String>,
    },

    // Control messages
    Heartbeat,
    Ping,
    Pong,
    Error {
        error: String,
        context: Option<String>,
    },
}

impl Message {
    pub fn new(payload: MessagePayload) -> Self {
        Self {
            id: Uuid::new_v4(),
            timestamp_us: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_micros() as u64,
            payload,
        }
    }
}
