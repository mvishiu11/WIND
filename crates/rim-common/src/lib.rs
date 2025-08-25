use anyhow::Result;
use bytes::{BufMut, BytesMut};
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Mode {
    Once,
    Periodic { period_ms: u64 },
    Monitored,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Frame {
    HelloSub { topic: String, mode: Mode },
    HelloAck { ok: bool, message: String },
    Data { seq: u64, ts_micros: u128, payload: Vec<u8> },
    Command { name: String },
    CommandAck { ok: bool, message: String },
    Error { message: String },
}

pub async fn write_frame<W: AsyncWriteExt + Unpin>(mut w: W, frame: &Frame) -> Result<()> {
    let data = bincode::serialize(frame)?;
    let len = data.len() as u32;
    let mut buf = BytesMut::with_capacity(4 + data.len());
    buf.put_u32(len);
    buf.extend_from_slice(&data);
    w.write_all(&buf).await?;
    Ok(())
}

pub async fn read_frame<R: AsyncReadExt + Unpin>(mut r: R) -> Result<Frame> {
    let mut len_buf = [0u8; 4];
    r.read_exact(&mut len_buf).await?;
    let len = u32::from_be_bytes(len_buf) as usize;
    let mut data = vec![0u8; len];
    r.read_exact(&mut data).await?;
    let frame: Frame = bincode::deserialize(&data)?;
    Ok(frame)
}

pub fn now_micros() -> u128 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_micros()
}
