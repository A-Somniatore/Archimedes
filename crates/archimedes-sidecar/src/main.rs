//! Archimedes Sidecar - Entry point
//!
//! This is the main binary for the Archimedes sidecar proxy.

use std::path::PathBuf;

use tracing::{error, info};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use archimedes_sidecar::{SidecarConfig, SidecarServer};

/// Command-line arguments.
struct Args {
    /// Path to configuration file.
    config: Option<PathBuf>,
}

impl Args {
    fn parse() -> Self {
        let mut args = std::env::args().skip(1);
        let mut config = None;

        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--config" | "-c" => {
                    config = args.next().map(PathBuf::from);
                }
                "--help" | "-h" => {
                    print_help();
                    std::process::exit(0);
                }
                "--version" | "-v" => {
                    println!("archimedes-sidecar {}", archimedes_sidecar::VERSION);
                    std::process::exit(0);
                }
                other => {
                    eprintln!("Unknown argument: {other}");
                    eprintln!("Use --help for usage information");
                    std::process::exit(1);
                }
            }
        }

        Self { config }
    }
}

fn print_help() {
    println!(
        r"Archimedes Sidecar - Multi-language service proxy

USAGE:
    archimedes-sidecar [OPTIONS]

OPTIONS:
    -c, --config <PATH>    Path to configuration file (TOML or JSON)
    -h, --help             Print help information
    -v, --version          Print version information

ENVIRONMENT VARIABLES:
    ARCHIMEDES_SIDECAR_LISTEN_PORT        Sidecar listen port (default: 8080)
    ARCHIMEDES_SIDECAR_UPSTREAM_URL       Upstream service URL (required)
    ARCHIMEDES_SIDECAR_UPSTREAM_TIMEOUT   Request timeout in seconds (default: 30)
    ARCHIMEDES_SIDECAR_CONTRACT_PATH      Path to Themis contract artifact
    ARCHIMEDES_SIDECAR_POLICY_BUNDLE_PATH Path to OPA policy bundle
    ARCHIMEDES_SIDECAR_OTLP_ENDPOINT      OpenTelemetry collector endpoint
    ARCHIMEDES_SIDECAR_METRICS_PORT       Prometheus metrics port (default: 9090)

EXAMPLES:
    # Run with configuration file
    archimedes-sidecar --config /etc/archimedes/sidecar.toml

    # Run with environment variables
    ARCHIMEDES_SIDECAR_UPSTREAM_URL=http://localhost:3000 archimedes-sidecar

For more information, see https://docs.themisplatform.io/archimedes/sidecar
"
    );
}

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "archimedes_sidecar=info,warn".into()),
        )
        .with(tracing_subscriber::fmt::layer().json())
        .init();

    // Parse arguments
    let args = Args::parse();

    // Load configuration
    let config = match args.config {
        Some(path) => {
            info!("Loading configuration from {:?}", path);
            match SidecarConfig::from_file(&path) {
                Ok(config) => config.with_env_overrides(),
                Err(e) => {
                    error!("Failed to load configuration: {}", e);
                    std::process::exit(1);
                }
            }
        }
        None => {
            info!("Using default configuration with environment overrides");
            SidecarConfig::default().with_env_overrides()
        }
    };

    // Validate configuration
    if let Err(e) = config.validate() {
        error!("Invalid configuration: {}", e);
        std::process::exit(1);
    }

    info!(
        "Starting Archimedes sidecar v{}",
        archimedes_sidecar::VERSION
    );
    info!("Listening on {}:{}", config.sidecar.listen_addr, config.sidecar.listen_port);
    info!("Upstream: {}", config.sidecar.upstream_url);

    // Create and run server
    let server = match SidecarServer::new(config) {
        Ok(server) => server,
        Err(e) => {
            error!("Failed to create server: {}", e);
            std::process::exit(1);
        }
    };

    if let Err(e) = server.run().await {
        error!("Server error: {}", e);
        std::process::exit(1);
    }
}
