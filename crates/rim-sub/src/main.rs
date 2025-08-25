use anyhow::{Result, anyhow};
use clap::Parser;
use rim_common::{Frame, Mode, write_frame, read_frame, now_micros};
use std::{time::Duration, net::SocketAddr};
use tokio::{net::TcpStream, io::{AsyncBufReadExt, AsyncWriteExt, BufReader}};
use hdrhistogram::Histogram;

#[derive(Parser, Debug)]
struct Opts {
    #[arg(long, default_value="SENSOR/TEMP")] service: String,
    #[arg(long, default_value="127.0.0.1:4500")] registry: String,
    #[arg(long, default_value="monitored")] mode: String,
    #[arg(long, default_value="50")] period_ms: u64,
    #[arg(long, default_value="5")] bench_secs: u64,
}

#[tokio::main]
async fn main() -> Result<()> {
    let opts = Opts::parse();
    let pub_addr = lookup_or_watch(&opts.registry, &opts.service).await?;
    println!("Connecting to publisher at {}", pub_addr);
    let mut stream = TcpStream::connect(pub_addr).await?;

    let mode = match opts.mode.as_str() {
        "once" => Mode::Once,
        "periodic" => Mode::Periodic { period_ms: opts.period_ms },
        _ => Mode::Monitored,
    };
    let hello = Frame::HelloSub { topic: opts.service.clone(), mode };
    write_frame(&mut stream, &hello).await?;
    let ack = read_frame(&mut stream).await?;
    match ack {
        Frame::HelloAck { ok, message } if ok => println!("Handshake OK: {}", message),
        _ => return Err(anyhow!("Handshake failed: {:?}", ack)),
    }

    if opts.bench_secs == 0 {
        loop {
            match rim_common::read_frame(&mut stream).await {
                Ok(Frame::Data { seq, ts_micros, payload }) => {
                    println!("seq={} age={}us payload={}B", seq, now_micros() - ts_micros, payload.len());
                }
                Ok(other) => println!("Other frame: {:?}", other),
                Err(e) => { eprintln!("Error: {}", e); break; }
            }
        }
    } else {
        run_bench(stream, opts.bench_secs).await?;
    }
    Ok(())
}

async fn run_bench(mut stream: TcpStream, secs: u64) -> Result<()> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(secs);
    let mut count: u64 = 0;
    let mut bytes: u64 = 0;
    let mut hist = Histogram::<u64>::new(3).unwrap();

    while tokio::time::Instant::now() < deadline {
        match rim_common::read_frame(&mut stream).await {
            Ok(Frame::Data { seq: _, ts_micros, payload }) => {
                let age = (rim_common::now_micros() - ts_micros) as u64;
                let _ = hist.record(age);
                count += 1;
                bytes += payload.len() as u64;
            }
            Ok(_) => {}
            Err(e) => { eprintln!("stream error: {}", e); break; }
        }
    }
    let dur_s = secs as f64;
    println!("--- BENCH RESULTS ---");
    println!("msgs/s: {:.0}", (count as f64) / dur_s);
    println!("MB/s:   {:.1}", (bytes as f64) / (1024.0*1024.0) / dur_s);
    println!("latency us: p50={} p95={} p99={} max={}",
        hist.value_at_quantile(0.50),
        hist.value_at_quantile(0.95),
        hist.value_at_quantile(0.99),
        hist.max()
    );
    Ok(())
}

async fn lookup_or_watch(registry: &str, service: &str) -> Result<SocketAddr> {
    if let Some(a) = try_lookup(registry, service).await? { return Ok(a); }
    let mut stream = TcpStream::connect(registry).await?;
    let mut req = serde_json::json!({
        "kind":"Watch",
        "service": service
    }).to_string();
    req.push('\n');
    stream.write_all(req.as_bytes()).await?;
    let mut r = BufReader::new(stream);
    let mut line = String::new();
    r.read_line(&mut line).await?;
    #[derive(serde::Deserialize)]
    struct Notify { _status: String, _service: String, addr: String }
    let n: Notify = serde_json::from_str(line.trim())?;
    Ok(n.addr.parse()?)
}

async fn try_lookup(registry: &str, service: &str) -> Result<Option<SocketAddr>> {
    let mut stream = TcpStream::connect(registry).await?;
    let mut req = serde_json::json!({
        "kind":"Lookup",
        "service": service
    }).to_string();
    req.push('\n');
    stream.write_all(req.as_bytes()).await?;
    let mut r = BufReader::new(stream);
    let mut line = String::new();
    r.read_line(&mut line).await?;
    #[derive(serde::Deserialize)]
    struct Resp { status: String, addr: Option<String> }
    let resp: Resp = serde_json::from_str(line.trim())?;
    match resp.status.as_str() {
        "Ok" => Ok(Some(resp.addr.unwrap().parse()?)),
        _ => Ok(None)
    }
}
