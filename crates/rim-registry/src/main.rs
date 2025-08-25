use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, time::{Duration, Instant}};
use tokio::{net::{TcpListener, TcpStream}, io::{AsyncBufReadExt, AsyncWriteExt, BufReader}, sync::{mpsc, RwLock}};
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind")]
enum Req {
    Register { service: String, addr: String, ttl_ms: u64 },
    Renew    { service: String, addr: String, ttl_ms: u64 },
    Lookup   { service: String },
    Watch    { service: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status")]
enum Resp {
    Ok { addr: Option<String> },
    NotFound,
    Subscribed,
    Error { message: String },
    Notify { service: String, addr: String },
}

#[derive(Debug, Clone)]
struct Entry {
    addr: String,
    expires_at: Instant,
}

#[derive(Default)]
struct State {
    entries: HashMap<String, Entry>,
    watchers: HashMap<String, Vec<mpsc::UnboundedSender<Resp>>>,
}

impl State {
    fn upsert(&mut self, service: String, addr: String, ttl: Duration) {
        self.entries.insert(service.clone(), Entry { addr: addr.clone(), expires_at: Instant::now() + ttl });
        if let Some(list) = self.watchers.remove(&service) {
            for tx in list {
                let _ = tx.send(Resp::Notify { service: service.clone(), addr: addr.clone() });
            }
        }
    }
    fn renew(&mut self, service: &str, addr: &str, ttl: Duration) -> bool {
        if let Some(e) = self.entries.get_mut(service) {
            if e.addr == addr {
                e.expires_at = Instant::now() + ttl;
                return true;
            }
        }
        false
    }
    fn lookup(&self, service: &str) -> Option<String> {
        self.entries.get(service).map(|e| e.addr.clone())
    }
    fn prune(&mut self) {
        let now = Instant::now();
        self.entries.retain(|_, e| e.expires_at > now);
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let addr = std::env::args().nth(1).unwrap_or("127.0.0.1:4500".into());
    let listener = TcpListener::bind(&addr).await?;
    println!("RIM registry listening on {}", addr);

    let state = Arc::new(RwLock::new(State::default()));

    {
        let state = state.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_millis(1000));
            loop {
                interval.tick().await;
                let mut s = state.write().await;
                s.prune();
            }
        });
    }

    loop {
        let (sock, peer) = listener.accept().await?;
        let st = state.clone();
        tokio::spawn(async move {
            if let Err(e) = handle(st, sock).await {
                eprintln!("client {} error: {}", peer, e);
            }
        });
    }
}

async fn handle(state: Arc<RwLock<State>>, sock: TcpStream) -> Result<()> {
    let (r, mut w) = sock.into_split();
    let mut br = BufReader::new(r);
    let mut line = String::new();

    while br.read_line(&mut line).await? > 0 {
        let req: Result<Req, _> = serde_json::from_str(line.trim());
        if req.is_err() {
            let resp = Resp::Error { message: "invalid json".into() };
            w.write_all(serde_json::to_string(&resp)?.as_bytes()).await?;
            w.write_all(b"\n").await?;
            line.clear();
            continue;
        }
        match req.unwrap() {
            Req::Register { service, addr, ttl_ms } => {
                let mut s = state.write().await;
                s.upsert(service.clone(), addr.clone(), Duration::from_millis(ttl_ms));
                let resp = Resp::Ok { addr: Some(addr) };
                w.write_all(serde_json::to_string(&resp)?.as_bytes()).await?;
                w.write_all(b"\n").await?;
            }
            Req::Renew { service, addr, ttl_ms } => {
                let mut s = state.write().await;
                let ok = s.renew(&service, &addr, Duration::from_millis(ttl_ms));
                let resp = if ok { Resp::Ok { addr: Some(addr) } } else { Resp::NotFound };
                w.write_all(serde_json::to_string(&resp)?.as_bytes()).await?;
                w.write_all(b"\n").await?;
            }
            Req::Lookup { service } => {
                let s = state.read().await;
                if let Some(addr) = s.lookup(&service) {
                    let resp = Resp::Ok { addr: Some(addr) };
                    w.write_all(serde_json::to_string(&resp)?.as_bytes()).await?;
                } else {
                    let resp = Resp::NotFound;
                    w.write_all(serde_json::to_string(&resp)?.as_bytes()).await?;
                }
                w.write_all(b"\n").await?;
            }
            Req::Watch { service } => {
                let (tx, mut rx) = mpsc::unbounded_channel();
                {
                    let mut s = state.write().await;
                    if let Some(addr) = s.lookup(&service) {
                        let _ = tx.send(Resp::Notify { service: service.clone(), addr });
                    } else {
                        s.watchers.entry(service.clone()).or_default().push(tx);
                    }
                }
                if let Some(resp) = rx.recv().await {
                    w.write_all(serde_json::to_string(&resp)?.as_bytes()).await?;
                    w.write_all(b"\n").await?;
                } else {
                    let resp = Resp::Error { message: "watch closed".into() };
                    w.write_all(serde_json::to_string(&resp)?.as_bytes()).await?;
                    w.write_all(b"\n").await?;
                }
            }
        }
        line.clear();
    }
    Ok(())
}
