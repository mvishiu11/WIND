use crate::{Publisher, RpcServer};
use wind_core::Result;

/// Combined WIND server that can serve both pub/sub and RPC
pub struct WindServer {
    service_name: String,
    publisher: Option<Publisher>,
    rpc_server: Option<RpcServer>,
}

impl WindServer {
    /// Create a new WIND server
    pub fn new(service_name: String) -> Self {
        Self {
            service_name,
            publisher: None,
            rpc_server: None,
        }
    }

    /// Add publisher capability
    pub fn with_publisher(mut self, bind_address: String, registry_address: String) -> Self {
        self.publisher = Some(Publisher::new(
            self.service_name.clone(),
            bind_address,
            registry_address,
        ));
        self
    }

    /// Add RPC server capability  
    pub fn with_rpc_server(mut self, bind_address: String, registry_address: String) -> Self {
        self.rpc_server = Some(RpcServer::new(
            self.service_name.clone(),
            bind_address,
            registry_address,
        ));
        self
    }

    /// Get reference to publisher (if enabled)
    pub fn publisher(&self) -> Option<&Publisher> {
        self.publisher.as_ref()
    }

    /// Get reference to RPC server (if enabled)
    pub fn rpc_server(&self) -> Option<&RpcServer> {
        self.rpc_server.as_ref()
    }

    /// Start all enabled server components
    pub async fn start(&self) -> Result<()> {
        match (&self.publisher, &self.rpc_server) {
            (Some(pub_server), Some(rpc_server)) => {
                // Start both publisher and RPC server concurrently
                tokio::try_join!(pub_server.start(), rpc_server.start())?;
            }
            (Some(pub_server), None) => {
                pub_server.start().await?;
            }
            (None, Some(rpc_server)) => {
                rpc_server.start().await?;
            }
            (None, None) => {
                return Err(wind_core::WindError::Protocol(
                    "Server must have at least publisher or RPC capability".to_string(),
                ));
            }
        }

        Ok(())
    }
}
