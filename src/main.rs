//! Glass - MCP server for ManageEngine ServiceDesk Plus
//!
//! This binary runs as an MCP server using stdio transport, allowing
//! Claude Code or Claude Desktop to interact with ServiceDesk Plus
//! through natural language.
//!
//! # Configuration
//!
//! Set the following environment variables (or use a `.env` file):
//!
//! - `SDP_BASE_URL`: Base URL of your ServiceDesk Plus instance
//! - `SDP_API_KEY`: Technician API key for authentication
//!
//! # Usage
//!
//! ```bash
//! # Direct execution
//! ./glass
//!
//! # With environment variables
//! SDP_BASE_URL=https://servicedesk.example.com SDP_API_KEY=xxx ./glass
//! ```

use anyhow::{Context, Result};
use rmcp::{transport::stdio, ServiceExt};
use tracing_subscriber::{fmt, EnvFilter};

use glass::{config, sdp_client, server};

#[tokio::main]
async fn main() -> Result<()> {
    // Load .env file if present (ignore errors if not found)
    dotenvy::dotenv().ok();

    // Initialize logging to stderr (critical for stdio transport!)
    // stdout is reserved for MCP JSON-RPC messages
    fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("glass=info")),
        )
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .init();

    tracing::info!("Starting Glass MCP server v{}", env!("CARGO_PKG_VERSION"));

    // Load configuration from environment
    let config = config::Config::from_env().context("Failed to load configuration")?;

    tracing::debug!("Configuration loaded, base_url: {}", config.base_url);

    // Create the SDP client
    let sdp_client = sdp_client::SdpClient::new(&config).context("Failed to create SDP client")?;

    tracing::debug!("SDP client initialized");

    // Test connection to SDP server before starting
    tracing::info!("Testing connection to ServiceDesk Plus...");
    if let Err(e) = sdp_client.test_connection().await {
        tracing::error!(error = %e, "Connection test failed");
        // Continue anyway - the server might become available later
        // But warn the user clearly
        tracing::warn!(
            "Server will start but may not be able to reach ServiceDesk Plus. \
             Check configuration and network connectivity."
        );
    }

    // Create the MCP server
    let server = server::GlassServer::new(sdp_client);

    tracing::info!("Server initialized, starting stdio transport");

    // Serve on stdio transport
    let service = server
        .serve(stdio())
        .await
        .inspect_err(|e| {
            tracing::error!("serving error: {:?}", e);
        })
        .context("Failed to start server")?;

    tracing::info!("Server running, waiting for requests");

    // Wait for the service to complete (shutdown signal)
    service
        .waiting()
        .await
        .context("Server error during operation")?;

    tracing::info!("Server shutting down");

    Ok(())
}
