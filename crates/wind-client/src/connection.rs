use tokio::net::TcpStream;
use tokio::time::Duration;
use tracing::{error, info, warn};

use wind_core::{Message, MessageCodec, Result, WindError};

/// Connection manager with automatic reconnection
pub struct Connection {
    address: String,
    stream: Option<TcpStream>,
    reconnect_attempts: u32,
    max_reconnect_attempts: u32,
    reconnect_delay: Duration,
}

impl Connection {
    pub fn new(address: String) -> Self {
        Self {
            address,
            stream: None,
            reconnect_attempts: 0,
            max_reconnect_attempts: 10,
            reconnect_delay: Duration::from_millis(1000),
        }
    }

    pub async fn connect(&mut self) -> Result<()> {
        if self.stream.is_some() {
            return Ok(());
        }

        loop {
            match TcpStream::connect(&self.address).await {
                Ok(stream) => {
                    info!("Connected to {}", self.address);
                    self.stream = Some(stream);
                    self.reconnect_attempts = 0;
                    return Ok(());
                }
                Err(e) => {
                    self.reconnect_attempts += 1;
                    if self.reconnect_attempts > self.max_reconnect_attempts {
                        return Err(WindError::Connection(format!(
                            "Failed to connect to {} after {} attempts: {}",
                            self.address, self.max_reconnect_attempts, e
                        )));
                    }

                    warn!(
                        "Connection attempt {} failed: {}. Retrying in {:?}...",
                        self.reconnect_attempts, e, self.reconnect_delay
                    );

                    tokio::time::sleep(self.reconnect_delay).await;
                    // Exponential backoff with jitter
                    self.reconnect_delay =
                        std::cmp::min(self.reconnect_delay * 2, Duration::from_secs(30));
                }
            }
        }
    }

    pub async fn send(&mut self, message: &Message) -> Result<()> {
        if self.stream.is_none() {
            self.connect().await?;
        }

        if let Some(stream) = &mut self.stream {
            match MessageCodec::write(stream, message).await {
                Ok(()) => Ok(()),
                Err(e) => {
                    error!("Send failed: {}. Marking connection as disconnected.", e);
                    self.stream = None;
                    Err(e)
                }
            }
        } else {
            Err(WindError::Connection("No active connection".to_string()))
        }
    }

    pub async fn receive(&mut self) -> Result<Message> {
        if self.stream.is_none() {
            self.connect().await?;
        }

        if let Some(stream) = &mut self.stream {
            match MessageCodec::decode(stream).await {
                Ok(msg) => Ok(msg),
                Err(e) => {
                    error!("Receive failed: {}. Marking connection as disconnected.", e);
                    self.stream = None;
                    Err(e)
                }
            }
        } else {
            Err(WindError::Connection("No active connection".to_string()))
        }
    }

    pub fn is_connected(&self) -> bool {
        self.stream.is_some()
    }

    pub fn disconnect(&mut self) {
        self.stream = None;
        self.reconnect_attempts = 0;
        self.reconnect_delay = Duration::from_millis(1000);
    }
}
