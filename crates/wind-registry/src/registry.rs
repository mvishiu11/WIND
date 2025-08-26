use dashmap::DashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{broadcast, RwLock};
use tracing::{info, debug};
use uuid::Uuid;

use wind_core::{ServiceInfo, Result, WindError};
use crate::pattern::ServicePattern;

/// Service entry with TTL and metadata
#[derive(Debug, Clone)]
pub struct ServiceEntry {
    pub info: ServiceInfo,
    pub registered_at: Instant,
    pub expires_at: Instant,
    pub last_heartbeat: Instant,
}

impl ServiceEntry {
    pub fn new(info: ServiceInfo, ttl: Duration) -> Self {
        let now = Instant::now();
        Self {
            info,
            registered_at: now,
            expires_at: now + ttl,
            last_heartbeat: now,
        }
    }
    
    pub fn renew(&mut self, ttl: Duration) {
        let now = Instant::now();
        self.last_heartbeat = now;
        self.expires_at = now + ttl;
    }
    
    pub fn is_expired(&self) -> bool {
        Instant::now() > self.expires_at
    }
}

/// Watch subscription for service discovery notifications
#[derive(Debug)]
pub struct ServiceWatch {
    pub id: Uuid,
    pub pattern: ServicePattern,
    pub sender: broadcast::Sender<ServiceInfo>,
}

/// Main registry that manages service discovery with pattern matching
#[derive(Debug)]
pub struct Registry {
    /// Active services by name
    services: DashMap<String, ServiceEntry>,
    /// Active watchers for pattern-based discovery
    watches: Arc<RwLock<Vec<ServiceWatch>>>,
    /// Schema registry for type validation
    schemas: DashMap<String, wind_core::Schema>,
    /// Metrics
    metrics: RegistryMetrics,
}

#[derive(Debug, Default)]
pub struct RegistryMetrics {
    pub total_registrations: std::sync::atomic::AtomicU64,
    pub active_services: std::sync::atomic::AtomicU64,
    pub total_lookups: std::sync::atomic::AtomicU64,
    pub active_watches: std::sync::atomic::AtomicU64,
}

impl Registry {
    pub fn new() -> Self {
        Self {
            services: DashMap::new(),
            watches: Arc::new(RwLock::new(Vec::new())),
            schemas: DashMap::new(),
            metrics: RegistryMetrics::default(),
        }
    }
    
    /// Register a service with TTL
    pub async fn register_service(&self, info: ServiceInfo, ttl_ms: u64) -> Result<()> {
        let ttl = Duration::from_millis(ttl_ms);
        let entry = ServiceEntry::new(info.clone(), ttl);
        
        info!("Registering service: {} at {}", info.name, info.address);
        
        // Store the service
        self.services.insert(info.name.clone(), entry);
        
        // Update metrics
        self.metrics.total_registrations.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        self.metrics.active_services.store(
            self.services.len() as u64, 
            std::sync::atomic::Ordering::Relaxed
        );
        
        // Notify watchers
        self.notify_watchers(&info).await;
        
        Ok(())
    }
    
    /// Renew service registration
    pub fn renew_service(&self, name: &str, address: &str, ttl_ms: u64) -> Result<()> {
        let ttl = Duration::from_millis(ttl_ms);
        
        if let Some(mut entry) = self.services.get_mut(name) {
            if entry.info.address == address {
                entry.renew(ttl);
                debug!("Renewed service: {} at {}", name, address);
                return Ok(());
            }
        }
        
        Err(WindError::ServiceNotFound(name.to_string()))
    }
    
    /// Lookup specific service by exact name
    pub fn lookup_service(&self, name: &str) -> Option<ServiceInfo> {
        self.metrics.total_lookups.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        
        self.services.get(name)
            .filter(|entry| !entry.is_expired())
            .map(|entry| entry.info.clone())
    }
    
    /// Discover services matching a pattern
    pub fn discover_services(&self, pattern: &str) -> Result<Vec<ServiceInfo>> {
        let matcher = ServicePattern::new(pattern)
            .map_err(|e| WindError::Registry(format!("Invalid pattern: {}", e)))?;
        
        let services = self.services
            .iter()
            .filter(|entry| !entry.value().is_expired())
            .filter(|entry| matcher.matches(&entry.key()))
            .map(|entry| entry.value().info.clone())
            .collect();
        
        Ok(services)
    }
    
    /// Watch for services matching a pattern
    pub async fn watch_services(&self, pattern: &str) -> Result<broadcast::Receiver<ServiceInfo>> {
        let matcher = ServicePattern::new(pattern)
            .map_err(|e| WindError::Registry(format!("Invalid pattern: {}", e)))?;
        
        let (tx, rx) = broadcast::channel(1000);
        
        let watch = ServiceWatch {
            id: Uuid::new_v4(),
            pattern: matcher.clone(),
            sender: tx,
        };
        
        // Send current matching services
        let current_services = self.discover_services(pattern)?;
        for service in current_services {
            let _ = watch.sender.send(service);
        }
        
        // Add to active watches
        {
            let mut watches = self.watches.write().await;
            watches.push(watch);
        }
        
        self.metrics.active_watches.store(
            self.watches.read().await.len() as u64,
            std::sync::atomic::Ordering::Relaxed
        );
        
        Ok(rx)
    }
    
    /// Remove expired services (called periodically)
    pub async fn cleanup_expired(&self) {
        let before = self.services.len();
        self.services.retain(|_, entry| !entry.is_expired());
        let after = self.services.len();
        
        if before != after {
            info!("Cleaned up {} expired services", before - after);
            self.metrics.active_services.store(
                after as u64,
                std::sync::atomic::Ordering::Relaxed
            );
        }
        
        // Clean up closed watchers
        {
            let mut watches = self.watches.write().await;
            watches.retain(|watch| !watch.sender.receiver_count() == 0);
            self.metrics.active_watches.store(
                watches.len() as u64,
                std::sync::atomic::Ordering::Relaxed
            );
        }
    }
    
    /// Register a schema for type validation
    pub fn register_schema(&self, schema: wind_core::Schema) {
        info!("Registering schema: {} v{}", schema.name, schema.version);
        self.schemas.insert(schema.id.clone(), schema);
    }
    
    /// Get schema by ID
    pub fn get_schema(&self, id: &str) -> Option<wind_core::Schema> {
        self.schemas.get(id).map(|entry| entry.value().clone())
    }
    
    /// List all active services (for debugging/monitoring)
    pub fn list_services(&self) -> Vec<ServiceInfo> {
        self.services
            .iter()
            .filter(|entry| !entry.value().is_expired())
            .map(|entry| entry.value().info.clone())
            .collect()
    }
    
    /// Get registry metrics
    pub fn metrics(&self) -> &RegistryMetrics {
        &self.metrics
    }
    
    async fn notify_watchers(&self, service: &ServiceInfo) {
        let watches = self.watches.read().await;
        for watch in watches.iter() {
            if watch.pattern.matches(&service.name) {
                let _ = watch.sender.send(service.clone());
            }
        }
    }
}

impl Default for Registry {
    fn default() -> Self {
        Self::new()
    }
}
