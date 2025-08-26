use clap::{Parser, Subcommand};
use tracing_subscriber;

mod commands;

#[derive(Parser)]
#[command(name = "wind")]
#[command(about = "WIND protocol command-line interface")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    #[arg(long, default_value = "127.0.0.1:7001", global = true)]
    registry: String,

    #[arg(long, default_value = "info", global = true)]
    log_level: String,
}

#[derive(Subcommand)]
enum Commands {
    /// Discover services matching a pattern
    Discover {
        /// Pattern to match (supports glob syntax like SENSOR/*/TEMP)
        pattern: String,

        #[arg(long)]
        json: bool,
    },
    /// Subscribe to a service and print received values
    Subscribe {
        /// Service name to subscribe to
        service: String,

        #[arg(long, default_value = "on-change")]
        mode: String,

        #[arg(long)]
        period_ms: Option<u64>,

        #[arg(long)]
        once: bool,
    },
    /// Make an RPC call to a service
    Call {
        /// Service name
        service: String,

        /// Method name
        method: String,

        /// Parameters (JSON format)
        #[arg(default_value = "{}")]
        params: String,

        #[arg(long, default_value = "5")]
        timeout_secs: u64,
    },
    /// List all active services
    List {
        #[arg(long)]
        json: bool,
    },
    /// Publish test data to a service pattern
    Publish {
        /// Service name pattern
        service: String,

        /// Value to publish (JSON format)
        value: String,

        #[arg(long)]
        repeat: Option<u64>,

        #[arg(long, default_value = "1000")]
        interval_ms: u64,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(&cli.log_level)
        .init();

    match cli.command {
        Commands::Discover { pattern, json } => {
            commands::discover(&cli.registry, &pattern, json).await?;
        }
        Commands::Subscribe {
            service,
            mode,
            period_ms,
            once,
        } => {
            commands::subscribe(&cli.registry, &service, &mode, period_ms, once).await?;
        }
        Commands::Call {
            service,
            method,
            params,
            timeout_secs,
        } => {
            commands::call(&cli.registry, &service, &method, &params, timeout_secs).await?;
        }
        Commands::List { json } => {
            commands::list(&cli.registry, json).await?;
        }
        Commands::Publish {
            service,
            value,
            repeat,
            interval_ms,
        } => {
            commands::publish(&cli.registry, &service, &value, repeat, interval_ms).await?;
        }
    }

    Ok(())
}
