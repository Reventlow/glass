# Glass Implementation Plan

A detailed phased implementation plan for building the Glass MCP server.

## Table of Contents

1. [Project Overview](#project-overview)
2. [Dependencies](#dependencies)
3. [Architecture](#architecture)
4. [Milestone Breakdown](#milestone-breakdown)
5. [MCP Tool Schemas](#mcp-tool-schemas)
6. [Key Code Patterns](#key-code-patterns)
7. [Architectural Considerations](#architectural-considerations)
8. [Risk Assessment](#risk-assessment)

---

## Project Overview

**Glass** is an MCP (Model Context Protocol) server that exposes ManageEngine ServiceDesk Plus operations as tools for AI assistants. It uses stdio transport for communication with Claude Code and Claude Desktop.

**Target:** `servicedesk.fynbus.dk` (on-premises ServiceDesk Plus)

---

## Dependencies

### Cargo.toml

```toml
[package]
name = "glass"
version = "0.1.0"
edition = "2021"
description = "MCP server for ManageEngine ServiceDesk Plus"
license = "MIT"

[dependencies]
# MCP SDK - official Rust implementation
rmcp = { git = "https://github.com/modelcontextprotocol/rust-sdk", features = [
    "server",
    "transport-io",
    "macros",
] }

# Async runtime
tokio = { version = "1", features = ["full"] }

# HTTP client
reqwest = { version = "0.12", features = ["json"] }

# Serialization
serde = { version = "1", features = ["derive"] }
serde_json = "1"

# JSON Schema generation for MCP tool inputs
schemars = "0.8"

# Error handling
thiserror = "2"
anyhow = "1"

# Environment and configuration
dotenvy = "0.15"

# Logging
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }

[dev-dependencies]
# Testing
tokio-test = "0.4"
wiremock = "0.6"
pretty_assertions = "1"
```

### Dependency Rationale

| Crate | Purpose |
|-------|---------|
| `rmcp` | Official MCP Rust SDK with macros for tool definition |
| `tokio` | Async runtime required by rmcp and reqwest |
| `reqwest` | HTTP client for SDP API calls |
| `serde`/`serde_json` | JSON serialization for API payloads |
| `schemars` | Generate JSON Schema for MCP tool inputs |
| `thiserror` | Ergonomic custom error types |
| `anyhow` | Application-level error handling in main |
| `dotenvy` | Load `.env` files for configuration |
| `tracing` | Structured logging (to stderr, not stdout) |
| `wiremock` | Mock HTTP responses in tests |

---

## Architecture

```
glass/
├── Cargo.toml
├── .env.example
├── src/
│   ├── main.rs              # Entry point, environment loading, server startup
│   ├── server.rs            # MCP server struct, ServerHandler impl, tool routing
│   ├── error.rs             # Custom error types
│   ├── config.rs            # Configuration struct from environment
│   ├── sdp_client.rs        # ServiceDesk Plus HTTP client
│   ├── models/              # SDP API data models
│   │   ├── mod.rs
│   │   ├── request.rs       # Request/ticket models
│   │   ├── technician.rs    # Technician models
│   │   ├── note.rs          # Note models
│   │   └── common.rs        # Shared types (pagination, response wrapper)
│   └── tools/               # MCP tool implementations
│       ├── mod.rs           # Re-exports
│       ├── inputs.rs        # Tool input parameter structs (with JsonSchema)
│       ├── list_requests.rs
│       ├── get_request.rs
│       ├── create_request.rs
│       ├── update_request.rs
│       ├── close_request.rs
│       ├── add_note.rs
│       ├── list_technicians.rs
│       └── assign_request.rs
```

### Key Design Principles

1. **Separation of Concerns**: SDP client is independent of MCP; tools bridge the two
2. **Typed Everything**: Use strong types for API payloads, not raw JSON
3. **Error Propagation**: Custom errors flow from SDP client through tools to MCP responses
4. **Testability**: SDP client takes trait for HTTP, allowing mock injection

---

## Milestone Breakdown

### M1: Project Scaffold and Ping Tool

**Goal:** Get a minimal MCP server running with stdio transport.

**Tasks:**

1. **Initialize Cargo project**
   - Create `Cargo.toml` with initial dependencies
   - Set up directory structure
   - Create `.env.example`
   - Add `.gitignore` for Rust and `.env`

2. **Create config module** (`src/config.rs`)
   - Define `Config` struct with `base_url` and `api_key`
   - Load from environment with validation
   - Error if required vars missing

3. **Create error module** (`src/error.rs`)
   - Define `GlassError` enum with `thiserror`
   - Variants: `Config`, `Http`, `SdpApi`, `Serialization`

4. **Create minimal server** (`src/server.rs`)
   - Define `GlassServer` struct
   - Implement `ServerHandler` trait
   - Add single `ping` tool that returns "pong"

5. **Create entry point** (`src/main.rs`)
   - Load environment with `dotenvy`
   - Initialize tracing to stderr
   - Create server and serve on stdio
   - Handle graceful shutdown

**Acceptance Criteria:**
- `cargo build` succeeds
- Running binary responds to MCP initialize
- `ping` tool returns "pong" when invoked
- Logs go to stderr, not stdout (critical for stdio transport)

**Delegation:** rust-async-developer implements; I review.

---

### M2: SDP Client with List/Get Requests

**Goal:** Build the HTTP client and basic read operations.

**Tasks:**

1. **Create SDP client** (`src/sdp_client.rs`)
   - `SdpClient` struct holding `reqwest::Client` and config
   - Implement request signing (authtoken header)
   - Generic `request<T>` method for API calls
   - Handle SDP response envelope (unwrap `response_status`)

2. **Define request models** (`src/models/request.rs`)
   - `Request` struct with all fields from SDP
   - `RequestSummary` for list responses (fewer fields)
   - Handle SDP's timestamp format (`value`/`display_value`)

3. **Define common models** (`src/models/common.rs`)
   - `ListInfo` for pagination parameters
   - `SearchCriteria` for filters
   - `SdpResponse<T>` wrapper
   - `ResponseStatus` for error handling

4. **Implement list_requests**
   - `SdpClient::list_requests(params: ListParams) -> Result<Vec<RequestSummary>>`
   - Build `input_data` JSON for filters
   - Parse paginated response

5. **Implement get_request**
   - `SdpClient::get_request(id: &str) -> Result<Request>`
   - Return full request details

6. **Add unit tests with wiremock**
   - Mock successful list response
   - Mock successful get response
   - Mock 404 not found
   - Mock authentication failure

**Acceptance Criteria:**
- Client correctly formats requests with authtoken
- Successful API calls return typed data
- Errors are properly propagated with context
- No `.unwrap()` in production code paths

**Delegation:** rust-async-developer implements client; sdp-qa-engineer writes tests.

---

### M3: Read-Only MCP Tools

**Goal:** Wire up all read operations as MCP tools.

**Tasks:**

1. **Define tool input structs** (`src/tools/inputs.rs`)
   - `ListRequestsInput` with filter fields
   - `GetRequestInput` with `request_id`
   - `ListTechniciansInput` with optional group filter
   - All derive `Deserialize`, `JsonSchema`

2. **Implement list_requests tool** (`src/tools/list_requests.rs`)
   - Parse input, call SDP client
   - Format response as human-readable text
   - Include ticket ID, subject, status, assignee

3. **Implement get_request tool** (`src/tools/get_request.rs`)
   - Fetch full ticket details
   - Format with description, notes, history

4. **Implement list_technicians tool** (`src/tools/list_technicians.rs`)
   - Add `SdpClient::list_technicians()`
   - Return ID and name for each technician

5. **Register tools in server**
   - Add `#[tool]` annotations
   - Verify tool discovery works

6. **Integration smoke test**
   - Test against real SDP instance (manual or scripted)
   - Verify response formatting is useful

**Acceptance Criteria:**
- All 3 read tools appear in `tools/list` response
- Tool schemas have proper descriptions
- Real API calls succeed (validated manually)
- Error responses include SDP error messages

**Delegation:** rust-async-developer implements tools; sdp-qa-engineer validates schemas.

---

### M4: Write Tools

**Goal:** Add tools that modify data in SDP.

**Tasks:**

1. **Define input structs for write operations**
   - `CreateRequestInput` - subject required, others optional
   - `UpdateRequestInput` - request_id required
   - `CloseRequestInput` - request_id, closure_code, comments
   - `AddNoteInput` - request_id, content, visibility flags
   - `AssignRequestInput` - request_id, technician_id/group

2. **Implement SDP client methods**
   - `create_request(input) -> Result<Request>`
   - `update_request(id, input) -> Result<Request>`
   - `close_request(id, closure) -> Result<Request>`
   - `add_note(request_id, note) -> Result<Note>`
   - `assign_request(id, assignment) -> Result<Request>`

3. **Implement MCP tools**
   - Each tool validates input before calling client
   - Return confirmation with ticket ID and new state
   - Include clear success/failure messaging

4. **Add input validation**
   - Subject max 250 chars for create
   - At least one field required for update
   - At least technician or group for assign

5. **Test with mocks**
   - Mock successful create/update/close
   - Mock validation errors from SDP
   - Mock concurrent modification conflicts

**Acceptance Criteria:**
- All 5 write tools work correctly
- Input validation catches invalid requests before API call
- SDP error messages propagate to tool response
- Confirmation includes actionable information

**Delegation:** rust-async-developer implements; sdp-qa-engineer tests edge cases.

---

### M5: Error Handling and Polish

**Goal:** Production-ready error handling and logging.

**Tasks:**

1. **Enhance error types**
   - Add error codes for common SDP failures
   - Include request ID in error context where applicable
   - Sanitize errors to never leak API key

2. **Add request timeouts**
   - Configure reqwest client timeout (30s default)
   - Handle timeout gracefully with retry hint

3. **Add retry logic for transient failures**
   - Retry on 429 (rate limit) with backoff
   - Retry on 502/503/504 once
   - No retry on 4xx client errors

4. **Improve logging**
   - Log API calls at DEBUG level
   - Log errors at ERROR level with context
   - Add request correlation IDs

5. **Input sanitization**
   - Trim whitespace from string inputs
   - Validate email format where expected
   - Escape HTML in descriptions if needed

6. **Response formatting**
   - Consistent output format across tools
   - Include "what to do next" hints

**Acceptance Criteria:**
- API key never appears in logs or responses
- Transient failures are retried appropriately
- All errors have actionable messages
- Logs provide debugging information

**Delegation:** rust-async-developer implements; sdp-qa-engineer validates resilience.

---

### M6: Documentation and Release

**Goal:** Complete documentation and prepare for use.

**Tasks:**

1. **Create README.md**
   - Project description
   - Installation instructions
   - Configuration guide
   - Usage examples with Claude Code

2. **Create .env.example**
   - Document all environment variables
   - Include example values (redacted)

3. **Add rustdoc comments**
   - Document all public structs and functions
   - Include usage examples in docs

4. **Create CHANGELOG.md**
   - Document initial release

5. **Test Claude Code integration**
   - Add to `claude_code_config.json`
   - Verify all tools work in conversation
   - Document any quirks

6. **Optional: Binary releases**
   - GitHub Actions for cross-compilation
   - Release binaries for Linux/macOS

**Acceptance Criteria:**
- README sufficient for new user to get started
- All public APIs documented
- Works correctly with Claude Code
- No warnings from `cargo clippy`

---

## MCP Tool Schemas

See `/home/gorm/projects/glass/.claude/agent-memory/rust-mcp-tech-lead/mcp-tool-schemas.md` for complete schema definitions.

### Summary

| Tool | Required Params | Description |
|------|-----------------|-------------|
| `list_requests` | none | List/filter tickets |
| `get_request` | `request_id` | Get ticket details |
| `create_request` | `subject` | Create new ticket |
| `update_request` | `request_id` | Update ticket fields |
| `close_request` | `request_id` | Close ticket |
| `add_note` | `request_id`, `content` | Add note to ticket |
| `list_technicians` | none | List technicians |
| `assign_request` | `request_id` | Assign ticket |

---

## Key Code Patterns

### 1. Tool Definition with rmcp

```rust
use rmcp::{
    ServerHandler, tool, tool_router, tool_handler,
    model::{ServerInfo, ServerCapabilities, CallToolResult, Content},
    handler::server::tool::ToolRouter,
    ErrorData as McpError,
};
use schemars::JsonSchema;
use serde::Deserialize;

#[derive(Clone)]
pub struct GlassServer {
    client: SdpClient,
    tool_router: ToolRouter<Self>,
}

#[derive(Deserialize, JsonSchema)]
pub struct GetRequestInput {
    /// The unique ID of the ticket to retrieve
    request_id: String,
}

#[tool_router]
impl GlassServer {
    pub fn new(client: SdpClient) -> Self {
        Self {
            client,
            tool_router: Self::tool_router(),
        }
    }

    #[tool(description = "Get full details of a service desk ticket")]
    async fn get_request(
        &self,
        #[tool(aggr)] input: GetRequestInput,
    ) -> Result<CallToolResult, McpError> {
        let request = self.client
            .get_request(&input.request_id)
            .await
            .map_err(|e| McpError::internal_error(
                "sdp_error",
                Some(serde_json::json!({"message": e.to_string()}))
            ))?;

        Ok(CallToolResult::success(vec![
            Content::text(format_request(&request))
        ]))
    }
}

#[tool_handler]
impl ServerHandler for GlassServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: Default::default(),
            capabilities: ServerCapabilities::builder()
                .enable_tools()
                .build(),
            server_info: rmcp::model::Implementation {
                name: "glass".into(),
                version: env!("CARGO_PKG_VERSION").into(),
            },
            instructions: Some(
                "Glass provides access to ServiceDesk Plus tickets. \
                 Use list_requests to find tickets, get_request for details, \
                 and create/update/close for modifications.".into()
            ),
        }
    }
}
```

### 2. SDP Client Structure

```rust
use reqwest::Client;
use crate::{config::Config, error::GlassError};

pub struct SdpClient {
    http: Client,
    base_url: String,
    api_key: String,  // Never log this!
}

impl SdpClient {
    pub fn new(config: &Config) -> Result<Self, GlassError> {
        let http = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(GlassError::HttpClient)?;

        Ok(Self {
            http,
            base_url: config.base_url.clone(),
            api_key: config.api_key.clone(),
        })
    }

    async fn request<T: serde::de::DeserializeOwned>(
        &self,
        method: reqwest::Method,
        path: &str,
        input_data: Option<serde_json::Value>,
    ) -> Result<T, GlassError> {
        let url = format!("{}{}", self.base_url, path);

        let mut req = self.http
            .request(method, &url)
            .header("authtoken", &self.api_key)
            .header("Accept", "application/vnd.manageengine.sdp.v3+json");

        if let Some(data) = input_data {
            req = req
                .header("Content-Type", "application/x-www-form-urlencoded")
                .body(format!("input_data={}", serde_json::to_string(&data)?));
        }

        let response = req.send().await.map_err(GlassError::Http)?;

        // Handle HTTP errors
        if !response.status().is_success() {
            return Err(GlassError::HttpStatus {
                status: response.status(),
                body: response.text().await.unwrap_or_default(),
            });
        }

        // Parse and check SDP response status
        let sdp_response: SdpResponse<T> = response.json().await?;
        sdp_response.into_result()
    }
}
```

### 3. Error Types

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum GlassError {
    #[error("configuration error: {0}")]
    Config(String),

    #[error("HTTP client error: {0}")]
    HttpClient(#[source] reqwest::Error),

    #[error("HTTP request failed: {0}")]
    Http(#[source] reqwest::Error),

    #[error("HTTP {status}: {body}")]
    HttpStatus {
        status: reqwest::StatusCode,
        body: String,
    },

    #[error("SDP API error {code}: {message}")]
    SdpApi {
        code: u32,
        message: String,
    },

    #[error("JSON serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("request not found: {id}")]
    NotFound { id: String },

    #[error("authentication failed - check SDP_API_KEY")]
    Authentication,

    #[error("validation error: {0}")]
    Validation(String),
}
```

### 4. Main Entry Point

```rust
use anyhow::Result;
use rmcp::transport::stdio;
use tracing_subscriber::{fmt, EnvFilter};

mod config;
mod error;
mod models;
mod sdp_client;
mod server;
mod tools;

#[tokio::main]
async fn main() -> Result<()> {
    // Load .env file if present
    dotenvy::dotenv().ok();

    // Initialize logging to stderr (critical for stdio transport)
    fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .init();

    tracing::info!("Starting Glass MCP server");

    // Load configuration
    let config = config::Config::from_env()?;

    // Create SDP client
    let client = sdp_client::SdpClient::new(&config)?;

    // Create and run server
    let server = server::GlassServer::new(client);
    let service = server.serve(stdio()).await?;

    tracing::info!("Server running, waiting for requests");
    service.waiting().await?;

    Ok(())
}
```

---

## Architectural Considerations

### 1. Stdio Transport Requirements

**Critical:** All logging must go to stderr, never stdout. The stdout stream is reserved for MCP JSON-RPC messages. Using `tracing_subscriber` with `.with_writer(std::io::stderr)` handles this.

### 2. API Key Security

- Never log the API key
- Never include it in error messages
- Never return it in tool responses
- Store only in memory, load from environment

### 3. SDP API Quirks (On-Premises)

The on-premises SDP API has some differences from cloud:

- Uses `authtoken` header (not OAuth)
- Uses `input_data` form parameter (not JSON body)
- Response envelope format differs slightly
- Some endpoints may be version-specific

**Recommendation:** Implement a test mode that validates API connectivity on startup.

### 4. Rate Limiting

SDP may enforce rate limits. Implement exponential backoff:

```rust
async fn with_retry<T, F, Fut>(f: F) -> Result<T, GlassError>
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = Result<T, GlassError>>,
{
    let mut delay = Duration::from_millis(100);
    for attempt in 0..3 {
        match f().await {
            Ok(result) => return Ok(result),
            Err(GlassError::HttpStatus { status, .. })
                if status == StatusCode::TOO_MANY_REQUESTS => {
                tokio::time::sleep(delay).await;
                delay *= 2;
            }
            Err(e) => return Err(e),
        }
    }
    f().await
}
```

### 5. Pagination Handling

For `list_requests`, consider:
- Default to reasonable page size (20)
- Allow user to specify limit
- Return `has_more` indicator in response
- Consider auto-pagination for small result sets

### 6. HTML in Descriptions

SDP stores descriptions as HTML. Options:
- Return raw HTML (let Claude interpret)
- Strip HTML tags for plain text
- Convert to Markdown (best UX)

**Recommendation:** Return raw HTML initially; add Markdown conversion in M5.

---

## Risk Assessment

### High Risk

| Risk | Mitigation |
|------|------------|
| SDP API differs from documentation | Test against real instance early (M2) |
| rmcp crate breaking changes | Pin to specific commit in Cargo.toml |
| API key exposure | Sanitize all error messages, never log key |

### Medium Risk

| Risk | Mitigation |
|------|------------|
| Rate limiting causes failures | Implement retry with backoff |
| Large ticket descriptions | Truncate if necessary, indicate truncation |
| Invalid category/subcategory combos | Return clear error with valid options |

### Low Risk

| Risk | Mitigation |
|------|------------|
| Technician ID lookup tedious | Provide `list_technicians` tool |
| Timestamp format confusion | Use display_value from SDP responses |

---

## Next Steps

1. **Review this plan** - Confirm milestones align with priorities
2. **Begin M1** - Delegate scaffold and ping tool to rust-async-developer
3. **Validate SDP API access** - Test API key and basic connectivity
4. **Set up CI** - GitHub Actions for cargo test/clippy/fmt

---

*Plan created by rust-mcp-tech-lead agent*
*Last updated: 2026-02-06*
