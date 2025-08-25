use anyhow::{Result, anyhow};
use clap::Parser;
use rand::{Rng, rngs::StdRng, SeedableRng};
use rim_common::{Frame, Mode, write_frame, read_frame};
use std::{net::SocketAddr, time::Duration, sync::{Arc, atomic::{AtomicU64, Ordering}}};
use tokio::{net::{TcpListener, TcpStream}, io::{AsyncBufReadExt, AsyncWriteExt, BufReader}, sync::{broadcast, mpsc}};

#[derive(Parser, Debug)]
struct Opts {
    #[arg(long, default_value="SENSOR/TEMP")] service: String,
    #[arg(long, default_value="127.0.0.1:4500")] registry: String,
    #[arg(long, default_value="127.0.0.1:0")] bind: String,
    #[arg(long, default_value="128")] payload_bytes: usize,
    #[arg(long, default_value="200")] hz: u64,
    #[arg(long, default_value="5000")] ttl_ms: u64,
}

#[tokio::main]
async fn main() -> Result<()> {
    let opts = Opts::parse();
    let listener = TcpListener::bind(&opts.bind).await?;
    let addr = listener.local_addr()?;
    println!("Publisher listening on {}", addr);
    register_with_registry(&opts.registry, &opts.service, addr, opts.ttl_ms).await?;

    { // heartbeat
        let registry = opts.registry.clone();
        let service = opts.service.clone();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_millis(opts.ttl_ms/2)).await;
                let _ = renew(&registry, &service, addr, opts.ttl_ms).await;
            }
        });
    }

    let (tx, _rx) = broadcast::channel::<Vec<u8>>(1024);
    let (sensor_tx, mut sensor_rx) = mpsc::channel::<Vec<u8>>(16);
    let payload_size = opts.payload_bytes;

    tokio::spawn(async move {
        let period = Duration::from_secs_f64(1.0/(opts.hz as f64));
        let mut rng = StdRng::seed_from_u64(42);
        loop {
            let mut data = vec![0u8; payload_size];
            rng.fill(&mut data[..]);
            if sensor_tx.send(data).await.is_err() { break; }
            tokio::time::sleep(period).await;
        }
    });

    let seq = Arc::new(AtomicU64::new(0));
    {
        let tx = tx.clone();
        tokio::spawn(async move {
            while let Some(p) = sensor_rx.recv().await {
                let _ = tx.send(p);
            }
        });
    }

    loop {
        let (sock, peer) = listener.accept().await?;
        let tx = tx.clone();
        let seq = seq.clone();
        tokio::spawn(async move {
            if let Err(e) = handle_client(tx, seq, sock).await {
                eprintln!("client {} error: {}", peer, e);
            }
        });
    }
}

async fn handle_client(tx: broadcast::Sender<Vec<u8>>, seq: Arc<AtomicU64>, sock: TcpStream) -> Result<()> {
    let (r, mut w) = sock.into_split();
    let mut rd = tokio::io::BufReader::new(r);
    let hello = read_frame(&mut rd).await?;
    let mode = match hello {
        Frame::HelloSub { topic: _, mode } => mode,
        _ => return Err(anyhow!("expected HelloSub")),
    };
    let ack = Frame::HelloAck { ok: true, message: "ok".into() };
    write_frame(&mut w, &ack).await?;

    match mode {
        Mode::Once => {
            let payload = tokio::time::timeout(Duration::from_secs(1), tx.subscribe().recv()).await??;
            let n = seq.fetch_add(1, Ordering::Relaxed) + 1;
            let frame = Frame::Data { seq: n, ts_micros: rim_common::now_micros(), payload };
            write_frame(&mut w, &frame).await?;
        }
        Mode::Periodic { period_ms } => {
            let mut rx = tx.subscribe();
            let mut ticker = tokio::time::interval(Duration::from_millis(period_ms));
            loop {
                tokio::select! {
                    _ = ticker.tick() => {
                        let mut latest = None;
                        while let Ok(p) = rx.try_recv() { latest = Some(p); }
                        if let Some(p) = latest {
                            let n = seq.fetch_add(1, Ordering::Relaxed) + 1;
                            let frame = Frame::Data { seq: n, ts_micros: rim_common::now_micros(), payload: p };
                            if let Err(e) = write_frame(&mut w, &frame).await { eprintln!("write error: {}", e); break; }
                        }
                    }
                }
            }
        }
        Mode::Monitored => {
            let mut rx = tx.subscribe();
            let mut prev: Option<Vec<u8>> = None;
            loop {
                let p = rx.recv().await?;
                if prev.as_ref().map(|q| q != &p).unwrap_or(true) {
                    let n = seq.fetch_add(1, Ordering::Relaxed) + 1;
                    let frame = Frame::Data { seq: n, ts_micros: rim_common::now_micros(), payload: p.clone() };
                    if let Err(e) = write_frame(&mut w, &frame).await { eprintln!("write error: {}", e); break; }
                    prev = Some(p);
                }
            }
        }
    }
    Ok(())
}

async fn register_with_registry(registry: &str, service: &str, addr: SocketAddr, ttl_ms: u64) -> Result<()> {
    let mut stream = TcpStream::connect(registry).await?;
    let mut req = serde_json::json!({
        "kind": "Register",
        "service": service,
        "addr": addr.to_string(),
        "ttl_ms": ttl_ms
    }).to_string();
    req.push('\n');
    stream.write_all(req.as_bytes()).await?;
    let mut r = BufReader::new(stream);
    let mut line = String::new();
    r.read_line(&mut line).await?;
    println!("Registry response: {}", line.trim());
    Ok(())
}

async fn renew(registry: &str, service: &str, addr: SocketAddr, ttl_ms: u64) -> Result<()> {
    let mut stream = TcpStream::connect(registry).await?;
    let mut req = serde_json::json!({
        "kind": "Renew",
        "service": service,
        "addr": addr.to_string(),
        "ttl_ms": ttl_ms
    }).to_string();
    req.push('\n');
    stream.write_all(req.as_bytes()).await?;
    Ok(())
}
