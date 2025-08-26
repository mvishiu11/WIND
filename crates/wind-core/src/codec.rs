use bytes::{BufMut, BytesMut};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use crate::{Message, Result};

const MAX_MESSAGE_SIZE: usize = 16 * 1024 * 1024; // 16MB limit

pub struct MessageCodec;

impl MessageCodec {
    /// Encode message to bytes with length prefix
    pub fn encode(msg: &Message) -> Result<BytesMut> {
        let data = bincode::serialize(msg)?;
        if data.len() > MAX_MESSAGE_SIZE {
            return Err(crate::WindError::Protocol(
                format!("Message too large: {} bytes", data.len())
            ));
        }
        
        let mut buf = BytesMut::with_capacity(4 + data.len());
        buf.put_u32(data.len() as u32);
        buf.extend_from_slice(&data);
        Ok(buf)
    }
    
    /// Decode message from reader
    pub async fn decode<R: AsyncRead + Unpin>(reader: &mut R) -> Result<Message> {
        // Read length prefix
        let len = reader.read_u32().await? as usize;
        if len > MAX_MESSAGE_SIZE {
            return Err(crate::WindError::Protocol(
                format!("Message too large: {} bytes", len)
            ));
        }
        
        // Read message data
        let mut data = vec![0u8; len];
        reader.read_exact(&mut data).await?;
        
        let msg = bincode::deserialize(&data)?;
        Ok(msg)
    }
    
    /// Write encoded message to writer
    pub async fn write<W: AsyncWrite + Unpin>(writer: &mut W, msg: &Message) -> Result<()> {
        let encoded = Self::encode(msg)?;
        writer.write_all(&encoded).await?;
        writer.flush().await?;
        Ok(())
    }
}
