use clap::Parser;
use wind_registry::RegistryServer;

#[derive(Parser)]
#[command(name = "wind-registry")]
#[command(about = "WIND Registry Service for service discovery")]
struct Args {
    #[arg(long, default_value = "127.0.0.1:7001")]
    bind: String,

    #[arg(long, default_value = "info")]
    log_level: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(&args.log_level)
        .init();

    let server = RegistryServer::new(args.bind);
    server.run().await?;

    Ok(())
}
