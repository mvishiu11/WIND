use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::net::{TcpListener, TcpStream};
use tracing::{debug, error, info, warn};
use std::pin::Pin;
use std::future::Future;

use wind_core::{
    Message, MessagePayload, MessageCodec, WindValue, 
    ServiceType, Result, WindError
};

/// RPC method handler trait - using Box<dyn Fn> instead of async trait for object safety
pub type RpcHandlerFn = Box<dyn Fn(WindValue) -> futures::future::BoxFuture<'static, Result<WindValue>> + Send + Sync>;

/// RPC method handler trait - using Pin<Box<Future>> to make it object-safe
pub trait RpcHandler: Send + Sync {
    fn handle(&self, params: WindValue) -> Pin<Box<dyn Future<Output = Result<WindValue>> + Send + '_>>;
}

/// Simple function-based RPC handler
pub struct FunctionHandler<F, Fut>
where
    F: Fn(WindValue) -> Fut + Send + Sync,
    Fut: std::future::Future<Output = Result<WindValue>> + Send,
{
    handler: F,
}

impl<F, Fut> FunctionHandler<F, Fut>
where
    F: Fn(WindValue) -> Fut + Send + Sync,
    Fut: std::future::Future<Output = Result<WindValue>> + Send,
{
    pub fn new(handler: F) -> Self {
        Self { handler }
    }
    
    pub async fn call(&self, params: WindValue) -> Result<WindValue> {
        (self.handler)(params).await
    }
}

impl<F, Fut> RpcHandler for FunctionHandler<F, Fut>
where
    F: Fn(WindValue) -> Fut + Send + Sync,
    Fut: std::future::Future<Output = Result<WindValue>> + Send + 'static,
{
    fn handle(&self, params: WindValue) -> Pin<Box<dyn Future<Output = Result<WindValue>> + Send + '_>> {
        Box::pin((self.handler)(params))
    }
}

/// RPC server for handling remote procedure calls
pub struct RpcServer {
    service_name: String,
    bind_address: String,
    registry_address: String,
    schema_id: Option<String>,
    methods: Arc<RwLock<HashMap<String, Arc<dyn RpcHandler>>>>,
    ttl_ms: u64,
    tags: Vec<String>,
}

impl RpcServer {
    /// Create a new RPC server
    pub fn new(
        service_name: String,
        bind_address: String, 
        registry_address: String,
    ) -> Self {
        Self {
            service_name,
            bind_address,
            registry_address,
            schema_id: None,
            methods: Arc::new(RwLock::new(HashMap::new())),
            ttl_ms: 60000,
            tags: Vec::new(),
        }
    }
    
    /// Set optional schema ID for type validation
    pub fn with_schema(mut self, schema_id: String) -> Self {
        self.schema_id = Some(schema_id);
        self
    }
    
    /// Register an RPC method with a handler
    pub async fn register_method<H>(&self, method_name: String, handler: H) -> Result<()>
    where
        H: RpcHandler + 'static,
    {
        let mut methods = self.methods.write().await;
        methods.insert(method_name.clone(), Arc::new(handler));
        info!("Registered RPC method: {}", method_name);
        Ok(())
    }
    
    /// Register a simple function as an RPC method
    pub async fn register_function<F, Fut>(&self, method_name: String, handler: F) -> Result<()>
    where
        F: Fn(WindValue) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<WindValue>> + Send + 'static,
    {
        self.register_method(method_name, FunctionHandler::new(handler)).await
    }
    
    /// Start the RPC server
    pub async fn start(&self) -> Result<()> {
        let listener = TcpListener::bind(&self.bind_address).await?;
        let actual_address = listener.local_addr()?.to_string();
        
        info!("RPC Server '{}' listening on {}", self.service_name, actual_address);
        
        // Register with the registry
        self.register_service(&actual_address).await?;
        
        // Accept and handle client connections
        loop {
            match listener.accept().await {
                Ok((stream, addr)) => {
                    info!("New RPC client connected: {}", addr);
                    let methods = self.methods.clone();
                    tokio::spawn(async move {
                        if let Err(e) = Self::handle_client(methods, stream).await {
                            error!("RPC client {} error: {}", addr, e);
                        }
                    });
                }
                Err(e) => {
                    error!("Failed to accept RPC connection: {}", e);
                }
            }
        }
    }
    
    async fn register_service(&self, actual_address: &str) -> Result<()> {
        let mut registry_conn = tokio::net::TcpStream::connect(&self.registry_address).await?;
        
        let register_msg = Message::new(MessagePayload::RegisterService {
            service: self.service_name.clone(),
            address: actual_address.to_string(),
            service_type: ServiceType::RpcServer,
            schema_id: self.schema_id.clone(),
            ttl_ms: self.ttl_ms,
            tags: self.tags.clone(),
        });
        
        MessageCodec::write(&mut registry_conn, &register_msg).await?;
        let response = MessageCodec::decode(&mut registry_conn).await?;
        
        match response.payload {
            MessagePayload::ServiceRegistered { success, error, .. } => {
                if success {
                    info!("Successfully registered RPC service '{}' with registry", self.service_name);
                    Ok(())
                } else {
                    Err(WindError::Registry(
                        error.unwrap_or("Registration failed".to_string())
                    ))
                }
            }
            _ => Err(WindError::Protocol("Unexpected registry response".to_string()))
        }
    }
    
    async fn handle_client(
        methods: Arc<RwLock<HashMap<String, Arc<dyn RpcHandler>>>>,
        mut stream: TcpStream,
    ) -> Result<()> {
        loop {
            let request = MessageCodec::decode(&mut stream).await?;
            
            match request.payload {
                MessagePayload::RpcCall { service, method, params, schema_id } => {
                    debug!("Received RPC call: {}::{}", service, method);
                    
                    let response = {
                        let methods_guard = methods.read().await;
                        if let Some(handler) = methods_guard.get(&method) {
                            match handler.handle(params).await {
                                Ok(result) => MessagePayload::RpcResponse {
                                    call_id: request.id,
                                    result: Ok(result),
                                    schema_id,
                                },
                                Err(e) => MessagePayload::RpcResponse {
                                    call_id: request.id,
                                    result: Err(e.to_string()),
                                    schema_id: None,
                                },
                            }
                        } else {
                            MessagePayload::RpcResponse {
                                call_id: request.id,
                                result: Err(format!("Method not found: {}", method)),
                                schema_id: None,
                            }
                        }
                    };
                    
                    let response_msg = Message::new(response);
                    MessageCodec::write(&mut stream, &response_msg).await?;
                }
                MessagePayload::Ping => {
                    let pong = Message::new(MessagePayload::Pong);
                    MessageCodec::write(&mut stream, &pong).await?;
                }
                _ => {
                    warn!("Unexpected message type in RPC server: {:?}", request.payload);
                }
            }
        }
    }
}
