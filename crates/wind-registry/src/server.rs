use tokio::net::{TcpListener, TcpStream};
use tokio::time::{interval, Duration};
use tracing::{error, info, warn};
use std::sync::Arc;

use wind_core::{Message, MessagePayload, MessageCodec};
use crate::Registry;

/// Registry server that handles client connections
pub struct RegistryServer {
    registry: Arc<Registry>,
    bind_address: String,
}

impl RegistryServer {
    pub fn new(bind_address: String) -> Self {
        Self {
            registry: Arc::new(Registry::new()),
            bind_address,
        }
    }
    
    pub async fn run(&self) -> wind_core::Result<()> {
        let listener = TcpListener::bind(&self.bind_address).await?;
        info!("WIND Registry listening on {}", self.bind_address);
        
        // Start cleanup task
        {
            let registry = self.registry.clone();
            tokio::spawn(async move {
                let mut cleanup_interval = interval(Duration::from_secs(10));
                loop {
                    cleanup_interval.tick().await;
                    registry.cleanup_expired().await;
                }
            });
        }
        
        // Start metrics reporting task  
        {
            let registry = self.registry.clone();
            tokio::spawn(async move {
                let mut metrics_interval = interval(Duration::from_secs(30));
                loop {
                    metrics_interval.tick().await;
                    let metrics = registry.metrics();
                    info!(
                        "Registry metrics: {} active services, {} total registrations, {} total lookups, {} active watches",
                        metrics.active_services.load(std::sync::atomic::Ordering::Relaxed),
                        metrics.total_registrations.load(std::sync::atomic::Ordering::Relaxed),
                        metrics.total_lookups.load(std::sync::atomic::Ordering::Relaxed),
                        metrics.active_watches.load(std::sync::atomic::Ordering::Relaxed),
                    );
                }
            });
        }
        
        loop {
            match listener.accept().await {
                Ok((socket, addr)) => {
                    info!("New client connected: {}", addr);
                    let registry = self.registry.clone();
                    tokio::spawn(async move {
                        if let Err(e) = Self::handle_client(registry, socket).await {
                            error!("Client {} error: {}", addr, e);
                        }
                    });
                }
                Err(e) => {
                    error!("Failed to accept connection: {}", e);
                }
            }
        }
    }
    
    async fn handle_client(registry: Arc<Registry>, mut socket: TcpStream) -> wind_core::Result<()> {
        loop {
            let msg = MessageCodec::decode(&mut socket).await?;
            let response = Self::handle_message(&registry, msg).await;
            
            if let Some(response) = response {
                MessageCodec::write(&mut socket, &response).await?;
            }
        }
    }
    
    async fn handle_message(registry: &Arc<Registry>, msg: Message) -> Option<Message> {
        match msg.payload {
            MessagePayload::RegisterService { 
                service, 
                address, 
                service_type, 
                schema_id, 
                ttl_ms, 
                tags 
            } => {
                let info = wind_core::ServiceInfo {
                    name: service.clone(),
                    address,
                    service_type,
                    schema_id,
                    ttl_ms,
                    tags,
                };
                
                match registry.register_service(info, ttl_ms).await {
                    Ok(()) => Some(Message::new(MessagePayload::ServiceRegistered {
                        service,
                        success: true,
                        error: None,
                    })),
                    Err(e) => Some(Message::new(MessagePayload::ServiceRegistered {
                        service,
                        success: false,
                        error: Some(e.to_string()),
                    })),
                }
            }
            
            MessagePayload::DiscoverServices { pattern } => {
                match registry.discover_services(&pattern) {
                    Ok(services) => Some(Message::new(MessagePayload::ServicesDiscovered {
                        services,
                    })),
                    Err(e) => Some(Message::new(MessagePayload::Error {
                        error: e.to_string(),
                        context: Some(format!("Discovering pattern: {}", pattern)),
                    })),
                }
            }
            
            MessagePayload::Ping => {
                Some(Message::new(MessagePayload::Pong))
            }
            
            _ => {
                warn!("Unhandled message type: {:?}", msg.payload);
                None
            }
        }
    }
    
    pub fn registry(&self) -> Arc<Registry> {
        self.registry.clone()
    }
}
