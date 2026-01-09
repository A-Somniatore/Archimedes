//! Configuration module demonstrating archimedes-config usage.

use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

/// Application configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    /// Server listen address
    pub listen_addr: SocketAddr,
    
    /// Database configuration
    pub database: DatabaseConfig,
    
    /// Rate limiting configuration
    pub rate_limit: RateLimitConfig,
    
    /// CORS configuration
    pub cors: CorsConfig,
    
    /// Logging configuration
    pub logging: LoggingConfig,
}

/// Database configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
    pub min_connections: u32,
}

/// Rate limiting configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    /// Requests per window
    pub requests_per_window: u32,
    /// Window duration in seconds
    pub window_seconds: u64,
}

/// CORS configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorsConfig {
    /// Allowed origins
    pub allowed_origins: Vec<String>,
    /// Allowed methods
    pub allowed_methods: Vec<String>,
    /// Allowed headers
    pub allowed_headers: Vec<String>,
    /// Allow credentials
    pub allow_credentials: bool,
    /// Max age in seconds
    pub max_age_seconds: u64,
}

/// Logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// Log level (trace, debug, info, warn, error)
    pub level: String,
    /// JSON format
    pub json: bool,
}

impl AppConfig {
    /// Load configuration from file and environment.
    ///
    /// # Configuration Sources (in order of precedence)
    /// 1. Environment variables (ARCHIMEDES_*)
    /// 2. archimedes.toml in current directory
    /// 3. Default values
    pub fn load() -> Result<Self, Box<dyn std::error::Error>> {
        // For this example, use defaults
        // In production, use archimedes-config to load from file
        Ok(Self::default())
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            listen_addr: "127.0.0.1:8080".parse().unwrap(),
            database: DatabaseConfig {
                url: "postgres://localhost/showcase".to_string(),
                max_connections: 10,
                min_connections: 1,
            },
            rate_limit: RateLimitConfig {
                requests_per_window: 100,
                window_seconds: 60,
            },
            cors: CorsConfig {
                allowed_origins: vec!["*".to_string()],
                allowed_methods: vec![
                    "GET".to_string(),
                    "POST".to_string(),
                    "PUT".to_string(),
                    "DELETE".to_string(),
                    "OPTIONS".to_string(),
                ],
                allowed_headers: vec!["*".to_string()],
                allow_credentials: true,
                max_age_seconds: 3600,
            },
            logging: LoggingConfig {
                level: "info".to_string(),
                json: false,
            },
        }
    }
}

/// Initialize logging based on configuration.
pub fn init_logging(config: &AppConfig) {
    let level = match config.logging.level.as_str() {
        "trace" => tracing::Level::TRACE,
        "debug" => tracing::Level::DEBUG,
        "info" => tracing::Level::INFO,
        "warn" => tracing::Level::WARN,
        "error" => tracing::Level::ERROR,
        _ => tracing::Level::INFO,
    };

    if config.logging.json {
        // JSON logging for production
        tracing_subscriber::fmt()
            .with_max_level(level)
            .json()
            .init();
    } else {
        // Pretty logging for development
        tracing_subscriber::fmt()
            .with_max_level(level)
            .pretty()
            .init();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = AppConfig::default();
        assert_eq!(config.listen_addr.port(), 8080);
        assert_eq!(config.rate_limit.requests_per_window, 100);
    }

    #[test]
    fn test_config_load() {
        let config = AppConfig::load().expect("Should load config");
        assert!(config.cors.allowed_origins.contains(&"*".to_string()));
    }
}
