use crate::{RpcClient, Subscriber, Subscription};
use wind_core::{QosParams, Result, SubscriptionMode, WindValue};

/// High-level WIND client combining subscription and RPC capabilities
pub struct WindClient {
    subscriber: Subscriber,
    rpc_client: RpcClient,
}

impl WindClient {
    /// Create a new WIND client connected to the specified registry
    pub fn new(registry_address: String) -> Self {
        Self {
            subscriber: Subscriber::new(registry_address.clone()),
            rpc_client: RpcClient::new(registry_address),
        }
    }

    /// Subscribe to a service with default QoS
    pub async fn subscribe(&mut self, service_name: &str) -> Result<Subscription> {
        self.subscriber
            .subscribe(
                service_name,
                SubscriptionMode::OnChange,
                QosParams::default(),
            )
            .await
    }

    /// Subscribe with custom mode and QoS
    pub async fn subscribe_with_options(
        &mut self,
        service_name: &str,
        mode: SubscriptionMode,
        qos: QosParams,
    ) -> Result<Subscription> {
        self.subscriber.subscribe(service_name, mode, qos).await
    }

    /// Make a synchronous RPC call with 5 second timeout
    pub async fn call(
        &mut self,
        service_name: &str,
        method: &str,
        params: WindValue,
    ) -> Result<WindValue> {
        self.rpc_client
            .call(
                service_name,
                method,
                params,
                tokio::time::Duration::from_secs(5),
            )
            .await
    }

    /// Make an RPC call with custom timeout
    pub async fn call_with_timeout(
        &mut self,
        service_name: &str,
        method: &str,
        params: WindValue,
        timeout: tokio::time::Duration,
    ) -> Result<WindValue> {
        self.rpc_client
            .call(service_name, method, params, timeout)
            .await
    }

    /// Make an asynchronous RPC call (fire-and-forget)
    pub async fn call_async(
        &mut self,
        service_name: &str,
        method: &str,
        params: WindValue,
    ) -> Result<()> {
        self.rpc_client
            .call_async(service_name, method, params)
            .await
    }

    /// Discover services matching a pattern
    pub async fn discover(&mut self, pattern: &str) -> Result<Vec<wind_core::ServiceInfo>> {
        self.subscriber.discover_services(pattern).await
    }

    /// Get number of active subscriptions
    pub async fn subscription_count(&self) -> usize {
        self.subscriber.subscription_count().await
    }
}
