use anyhow::Result;
use clap::Parser;
use std::process::Stdio;
use tokio::{process::Command, time::Duration, net::TcpStream, io::{AsyncBufReadExt, BufReader}};
use rim_common::{Frame, Mode, write_frame, read_frame};
use hdrhistogram::Histogram;

#[derive(Parser, Debug)]
struct Opts {
    #[arg(long, default_value="127.0.0.1:4500")] registry: String,
    #[arg(long, default_value="SENSOR/TEMP")] service: String,
    #[arg(long, default_value="8")] subs: usize,
    #[arg(long, default_value="256")] payload_bytes: usize,
    #[arg(long, default_value="200")] hz: u64,
    #[arg(long, default_value="5")] secs: u64,
}

#[tokio::main]
async fn main() -> Result<()> {
    let opts = Opts::parse();
    let _reg = spawn_child("rim-registry", &[]).await;
    let pub_args = &[
        "--service", &opts.service,
        "--registry", &opts.registry,
        "--payload-bytes", &opts.payload_bytes.to_string(),
        "--hz", &opts.hz.to_string(),
    ];
    let _pub = spawn_child("rim-pub", pub_args).await;
    tokio::time::sleep(Duration::from_millis(600)).await;

    let mut handles = vec![];
    for _ in 0..opts.subs {
        let svc = opts.service.clone();
        let reg = opts.registry.clone();
        let secs = opts.secs;
        handles.push(tokio::spawn(async move {
            sub_bench(&reg, &svc, secs).await.unwrap_or_else(|_| (0, 0, hdrhistogram::Histogram::<u64>::new(3).unwrap()))
        }));
    }

    let mut total_msgs: u64 = 0;
    let mut total_bytes: u64 = 0;
    let mut hist = Histogram::<u64>::new(3).unwrap();
    for h in handles {
        let (msgs, bytes, hst) = h.await?;
        total_msgs += msgs;
        total_bytes += bytes;
        hist.add(&hst).unwrap();
    }

    let dur_s = opts.secs as f64;
    println!("=== AGGREGATE RESULTS ===");
    println!("subs: {}", opts.subs);
    println!("msgs/s: {:.0}", (total_msgs as f64) / dur_s);
    println!("MB/s:   {:.1}", (total_bytes as f64) / (1024.0*1024.0) / dur_s);
    println!("latency us: p50={} p95={} p99={} max={}",
        hist.value_at_quantile(0.50),
        hist.value_at_quantile(0.95),
        hist.value_at_quantile(0.99),
        hist.max()
    );
    Ok(())
}

async fn sub_bench(registry: &str, service: &str, secs: u64) -> Result<(u64, u64, Histogram<u64>)> {
    let pub_addr = lookup_or_watch(registry, service).await?;
    let mut stream = TcpStream::connect(pub_addr).await?;
    let hello = Frame::HelloSub { topic: service.to_string(), mode: Mode::Monitored };
    write_frame(&mut stream, &hello).await?;
    let _ = read_frame(&mut stream).await?;

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
    Ok((count, bytes, hist))
}

async fn spawn_child(bin: &str, args: &[&str]) -> Result<tokio::process::Child> {
    let mut cmd = Command::new("cargo");
    cmd.arg("run").arg("-p").arg(bin);
    if !args.is_empty() {
        cmd.arg("--").args(args);
    }
    cmd.stdout(Stdio::null()).stderr(Stdio::null());
    let child = cmd.spawn()?;
    Ok(child)
}

use std::net::SocketAddr;
async fn lookup_or_watch(registry: &str, service: &str) -> Result<SocketAddr> {
    if let Some(a) = try_lookup(registry, service).await? { return Ok(a); }
    let mut stream = TcpStream::connect(registry).await?;
    let mut req = serde_json::json!({
        "kind":"Watch",
        "service": service
    }).to_string();
    req.push('\n');
    use tokio::io::AsyncWriteExt;
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
    use tokio::io::AsyncWriteExt;
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
