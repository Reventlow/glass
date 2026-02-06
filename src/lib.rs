//! # Glass
//!
//! Glass is an MCP (Model Context Protocol) server for ManageEngine ServiceDesk Plus.
//!
//! It exposes ServiceDesk Plus operations as MCP tools, enabling AI assistants
//! like Claude to manage help desk tickets through natural language.
//!
//! ## Features
//!
//! - **Read operations**: List, search, and view ticket details
//! - **Write operations**: Create, update, close tickets and add notes
//! - **Assignment**: Assign tickets to technicians and support groups
//! - **Error handling**: Automatic retry for transient failures with exponential backoff
//! - **Security**: API keys are never logged or exposed in error messages
//!
//! ## Architecture
//!
//! The crate is organized into several modules:
//!
//! - [`config`] - Configuration loading from environment variables
//! - [`error`] - Error types with security-conscious message sanitization
//! - [`sdp_client`] - HTTP client for the ServiceDesk Plus API
//! - [`server`] - MCP server implementation with tool routing
//! - [`models`] - Data models for SDP API requests and responses
//! - [`tools`] - Tool input parameter structs
//!
//! ## Usage
//!
//! Glass is primarily used as a binary. To run:
//!
//! ```bash
//! # Set required environment variables
//! export SDP_BASE_URL=https://servicedesk.example.com
//! export SDP_API_KEY=your-api-key
//!
//! # Run the server
//! ./glass
//! ```
//!
//! ## Configuration
//!
//! Glass requires two environment variables:
//!
//! - `SDP_BASE_URL`: Base URL of your ServiceDesk Plus instance
//! - `SDP_API_KEY`: Technician API key for authentication
//!
//! Optional:
//! - `RUST_LOG`: Log level (e.g., `glass=debug`)
//!
//! ## Security Considerations
//!
//! The API key is stored only in memory and is:
//! - Never logged at any log level
//! - Sanitized from all error messages
//! - Not included in any tool responses
//!
//! ## Example
//!
//! Using the [`SdpClient`](sdp_client::SdpClient) directly:
//!
//! ```ignore
//! use glass::config::Config;
//! use glass::sdp_client::{SdpClient, ListParams};
//!
//! async fn example() -> Result<(), glass::error::GlassError> {
//!     let config = Config::from_env()?;
//!     let client = SdpClient::new(&config)?;
//!
//!     // List open high-priority tickets
//!     let params = ListParams::new()
//!         .with_status("Open")
//!         .with_priority("High")
//!         .with_limit(10);
//!
//!     let tickets = client.list_requests(params).await?;
//!     for ticket in tickets {
//!         println!("#{}: {}", ticket.id, ticket.display_subject());
//!     }
//!
//!     Ok(())
//! }
//! ```

#![warn(missing_docs)]
#![warn(rustdoc::missing_crate_level_docs)]

pub mod config;
pub mod error;
pub mod models;
pub mod sdp_client;
pub mod server;
pub mod tools;
