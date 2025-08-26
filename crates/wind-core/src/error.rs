use thiserror::Error;

#[derive(Error, Debug)]
pub enum WindError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Serialization error: {0}")]
    Serialization(#[from] bincode::Error),
    
    #[error("Service not found: {0}")]
    ServiceNotFound(String),
    
    #[error("Type mismatch: expected {expected}, got {actual}")]
    TypeMismatch { expected: String, actual: String },
    
    #[error("Connection error: {0}")]
    Connection(String),
    
    #[error("Registry error: {0}")]
    Registry(String),
    
    #[error("Protocol error: {0}")]
    Protocol(String),
    
    #[error("Schema error: {0}")]
    Schema(String),
    
    #[error("Timeout: {0}")]
    Timeout(String),
}

pub type Result<T> = std::result::Result<T, WindError>;
