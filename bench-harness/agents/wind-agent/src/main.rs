use anyhow::Context;
use clap::{Parser, Subcommand, ValueEnum};
use hdrhistogram::Histogram;
use rand::{Rng, RngCore, SeedableRng};
use rand::rngs::StdRng;
use serde::Serialize;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::time::Instant;

use wind_client::WindClient;
use wind_core::{QosParams, SubscriptionMode, WindValue};
use wind_server::Publisher;
use std::sync::Arc;

#[derive(Parser)]
#[command(name = "wind-agent")]
#[command(about = "Benchmark agent for WIND (publisher/subscriber)")]
struct Cli {
    #[command(subcommand)]
    cmd: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Publish payloads to a service name.
    Publisher {
        #[arg(long)]
        service: String,

        #[arg(long)]
        registry: String,

        #[arg(long, default_value = "127.0.0.1:0")]
        bind: String,

        #[arg(long, default_value_t = 5)]
        duration_secs: u64,

        #[arg(long, value_enum, default_value_t = PublishMode::Deterministic)]
        mode: PublishMode,

        #[arg(long, default_value_t = 1000.0)]
        hz: f64,

        #[arg(long, default_value_t = 256)]
        payload_bytes: usize,

        #[arg(long, value_enum, default_value_t = PayloadProfile::Fixed)]
        payload_profile: PayloadProfile,

        #[arg(long, default_value_t = 1)]
        seed: u64,
    },

    /// Subscribe to a service name (or discover by pattern) and measure latency.
    Subscriber {
        #[arg(long)]
        registry: String,

        #[arg(long, conflicts_with = "pattern")]
        service: Vec<String>,

        #[arg(long, conflicts_with = "service")]
        pattern: Option<String>,

        #[arg(long, default_value_t = 5)]
        duration_secs: u64,

        #[arg(long)]
        max_samples: Option<u64>,

        #[arg(long, default_value_t = 1)]
        seed: u64,
    },
}

#[derive(Copy, Clone, Debug, ValueEnum)]
enum PublishMode {
    Deterministic,
    Poisson,
}

#[derive(Copy, Clone, Debug, ValueEnum)]
enum PayloadProfile {
    Fixed,
    Iot,
}

#[derive(Serialize)]
struct PublisherSummary {
    role: &'static str,
    service: String,
    registry: String,
    mode: String,
    hz: f64,
    payload_bytes: usize,
    duration_secs: u64,
    published: u64,
    publish_errors: u64,
}

#[derive(Serialize)]
struct SubscriberSummary {
    role: &'static str,
    registry: String,
    services: Vec<String>,
    duration_secs: u64,
    received: u64,
    received_bytes: u64,
    decode_errors: u64,
    latency_hist: Vec<(u64, u64)>,
    latency: LatencySummary,
}

#[derive(Serialize)]
struct LatencySummary {
    min_us: u64,
    p50_us: u64,
    p90_us: u64,
    p95_us: u64,
    p99_us: u64,
    p999_us: u64,
    max_us: u64,
}

fn now_micros_i64() -> i64 {
    let d = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_else(|_| Duration::from_secs(0));
    d.as_micros() as i64
}

fn encode_payload(payload_bytes: usize, rng: &mut StdRng) -> Vec<u8> {
    let mut payload = vec![0u8; payload_bytes.max(8)];

    let ts = now_micros_i64().to_le_bytes();
    payload[0..8].copy_from_slice(&ts);

    if payload.len() > 8 {
        rng.fill_bytes(&mut payload[8..]);
    }

    payload
}

fn choose_payload_bytes(profile: PayloadProfile, fixed: usize, rng: &mut StdRng) -> usize {
    match profile {
        PayloadProfile::Fixed => fixed,
        PayloadProfile::Iot => {
            let r: f64 = rng.gen();
            if r < 0.70 {
                rng.gen_range(64..=256)
            } else if r < 0.95 {
                rng.gen_range(1024..=4096)
            } else {
                rng.gen_range(16 * 1024..=32 * 1024)
            }
        }
    }
}

fn decode_latency_us(payload: &[u8]) -> Option<u64> {
    if payload.len() < 8 {
        return None;
    }
    let mut ts_bytes = [0u8; 8];
    ts_bytes.copy_from_slice(&payload[0..8]);
    let sent_us = i64::from_le_bytes(ts_bytes);
    let now_us = now_micros_i64();
    if now_us < sent_us {
        return None;
    }
    Some((now_us - sent_us) as u64)
}

async fn run_publisher(
    service: String,
    registry: String,
    bind: String,
    duration_secs: u64,
    mode: PublishMode,
    hz: f64,
    payload_bytes: usize,
    payload_profile: PayloadProfile,
    seed: u64,
) -> anyhow::Result<()> {
    let publisher = Arc::new(Publisher::new(service.clone(), bind, registry.clone()));

    let publisher_task = {
        let publisher = publisher.clone();
        tokio::spawn(async move { publisher.start().await.context("publisher start failed") })
    };

    // Give the server time to bind and register.
    tokio::time::sleep(Duration::from_millis(900)).await;

    let start = Instant::now();
    let deadline = Duration::from_secs(duration_secs);

    let mut rng = StdRng::seed_from_u64(seed);
    let mut published: u64 = 0;
    let mut publish_errors: u64 = 0;

    let mode_str = match mode {
        PublishMode::Deterministic => "deterministic",
        PublishMode::Poisson => "poisson",
    };

    while start.elapsed() < deadline {
        if publisher_task.is_finished() {
            break;
        }

        let sleep_dur = match mode {
            PublishMode::Deterministic => {
                if hz <= 0.0 {
                    Duration::from_millis(1)
                } else {
                    Duration::from_secs_f64(1.0 / hz)
                }
            }
            PublishMode::Poisson => {
                if hz <= 0.0 {
                    Duration::from_millis(1)
                } else {
                    let u: f64 = rng.gen::<f64>().clamp(f64::MIN_POSITIVE, 1.0);
                    let dt = -u.ln() / hz;
                    Duration::from_secs_f64(dt)
                }
            }
        };

        tokio::time::sleep(sleep_dur).await;

        let bytes = choose_payload_bytes(payload_profile, payload_bytes, &mut rng);
        let payload = encode_payload(bytes, &mut rng);
        match publisher.publish(WindValue::Bytes(payload)).await {
            Ok(()) => published += 1,
            Err(_) => publish_errors += 1,
        }
    }

    publisher_task.abort();

    let summary = PublisherSummary {
        role: "publisher",
        service,
        registry,
        mode: mode_str.to_string(),
        hz,
        payload_bytes,
        duration_secs,
        published,
        publish_errors,
    };

    println!("{}", serde_json::to_string(&summary)?);
    Ok(())
}

async fn run_subscriber(
    registry: String,
    service: Vec<String>,
    pattern: Option<String>,
    duration_secs: u64,
    max_samples: Option<u64>,
    seed: u64,
) -> anyhow::Result<()> {
    let mut client = WindClient::new(registry.clone());

    let services: Vec<String> = if !service.is_empty() {
        service
    } else if let Some(pat) = pattern {
        client
            .discover(&pat)
            .await
            .context("discover failed")?
            .into_iter()
            .map(|s| s.name)
            .collect()
    } else {
        anyhow::bail!("either --service or --pattern must be provided")
    };

    let _ = seed; // reserved for future use

    let mut histogram = Histogram::<u64>::new(3).context("histogram init")?;
    let mut received: u64 = 0;
    let mut received_bytes: u64 = 0;
    let mut decode_errors: u64 = 0;

    let mut subs = Vec::with_capacity(services.len());
    for svc in &services {
        let subscribe_deadline = Instant::now() + Duration::from_secs(10);
        loop {
            match client
                .subscribe_with_options(svc, SubscriptionMode::OnChange, QosParams::default())
                .await
            {
                Ok(sub) => {
                    subs.push(sub);
                    break;
                }
                Err(e) => {
                    if Instant::now() >= subscribe_deadline {
                        return Err(e).with_context(|| format!("subscribe failed: {svc}"));
                    }
                    tokio::time::sleep(Duration::from_millis(200)).await;
                }
            }
        }
    }

    let start = Instant::now();
    let deadline = Duration::from_secs(duration_secs);

    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<Vec<u8>>();

    for mut sub in subs {
        let tx = tx.clone();
        tokio::spawn(async move {
            while let Some(value) = sub.next().await {
                if let WindValue::Bytes(b) = value {
                    let _ = tx.send(b);
                }
            }
        });
    }

    drop(tx);

    while start.elapsed() < deadline {
        if let Some(max) = max_samples {
            if received >= max {
                break;
            }
        }

        let msg = tokio::time::timeout(Duration::from_millis(200), rx.recv()).await;
        let Some(payload) = msg.ok().flatten() else {
            continue;
        };

        received += 1;
        received_bytes += payload.len() as u64;

        match decode_latency_us(&payload) {
            Some(us) => {
                let _ = histogram.record(us);
            }
            None => {
                decode_errors += 1;
            }
        }
    }

    let hist_pairs: Vec<(u64, u64)> = histogram
        .iter_recorded()
        .map(|v| (v.value_iterated_to(), v.count_at_value()))
        .collect();

    let has_samples = histogram.len() > 0;
    let latency = LatencySummary {
        min_us: if has_samples { histogram.min() } else { 0 },
        p50_us: if has_samples { histogram.value_at_quantile(0.50) } else { 0 },
        p90_us: if has_samples { histogram.value_at_quantile(0.90) } else { 0 },
        p95_us: if has_samples { histogram.value_at_quantile(0.95) } else { 0 },
        p99_us: if has_samples { histogram.value_at_quantile(0.99) } else { 0 },
        p999_us: if has_samples { histogram.value_at_quantile(0.999) } else { 0 },
        max_us: if has_samples { histogram.max() } else { 0 },
    };

    let summary = SubscriberSummary {
        role: "subscriber",
        registry,
        services,
        duration_secs,
        received,
        received_bytes,
        decode_errors,
        latency_hist: hist_pairs,
        latency,
    };

    println!("{}", serde_json::to_string(&summary)?);
    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.cmd {
        Command::Publisher {
            service,
            registry,
            bind,
            duration_secs,
            mode,
            hz,
            payload_bytes,
            payload_profile,
            seed,
        } => run_publisher(
            service,
            registry,
            bind,
            duration_secs,
            mode,
            hz,
            payload_bytes,
            payload_profile,
            seed,
        )
        .await,

        Command::Subscriber {
            registry,
            service,
            pattern,
            duration_secs,
            max_samples,
            seed,
        } => run_subscriber(registry, service, pattern, duration_secs, max_samples, seed).await,
    }
}
