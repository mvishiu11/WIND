use clap::{Parser, Subcommand};

mod latency_bench;
mod load_bench;
mod throughput_bench;

#[derive(Parser)]
#[command(name = "wind-bench")]
#[command(about = "WIND protocol benchmarking suite")]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    #[arg(long, default_value = "127.0.0.1:7001", global = true)]
    registry: String,

    #[arg(long, default_value = "warn", global = true)]
    log_level: String,
}

#[derive(Subcommand)]
enum Commands {
    /// Measure end-to-end latency
    Latency {
        #[arg(long, default_value = "10000")]
        samples: usize,

        #[arg(long, default_value = "256")]
        payload_bytes: usize,

        #[arg(long, default_value = "5")]
        duration_secs: u64,
    },
    /// Measure maximum throughput
    Throughput {
        #[arg(long, default_value = "8")]
        subscribers: usize,

        #[arg(long, default_value = "256")]
        payload_bytes: usize,

        #[arg(long, default_value = "10")]
        duration_secs: u64,

        #[arg(long, default_value = "1000")]
        target_hz: u64,
    },
    /// Load testing with multiple services
    Load {
        #[arg(long, default_value = "10")]
        services: usize,

        #[arg(long, default_value = "5")]
        subscribers_per_service: usize,

        #[arg(long, default_value = "30")]
        duration_secs: u64,

        #[arg(long, default_value = "100")]
        publish_hz: u64,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(&cli.log_level)
        .with_target(false)
        .with_thread_ids(false)
        .with_file(false)
        .with_line_number(false)
        .init();

    match cli.command {
        Commands::Latency {
            samples,
            payload_bytes,
            duration_secs,
        } => {
            latency_bench::run(&cli.registry, samples, payload_bytes, duration_secs).await?;
        }
        Commands::Throughput {
            subscribers,
            payload_bytes,
            duration_secs,
            target_hz,
        } => {
            throughput_bench::run(
                &cli.registry,
                subscribers,
                payload_bytes,
                duration_secs,
                target_hz,
            )
            .await?;
        }
        Commands::Load {
            services,
            subscribers_per_service,
            duration_secs,
            publish_hz,
        } => {
            load_bench::run(
                &cli.registry,
                services,
                subscribers_per_service,
                duration_secs,
                publish_hz,
            )
            .await?;
        }
    }

    Ok(())
}
