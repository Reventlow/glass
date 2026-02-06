# CLAUDE.md — Glass

## Project Overview

**Glass** is an MCP (Model Context Protocol) server written in Rust that exposes ManageEngine ServiceDesk Plus as a set of tools for AI assistants like Claude.

The target instance is `servicedesk.fynbus.dk` running ServiceDesk Plus (Enterprise Service Management) by ManageEngine.

## Goal

Build a Rust CLI binary that runs as an MCP server (stdio transport), allowing Claude Code or Claude Desktop to query and interact with ServiceDesk Plus tickets through natural language.

## Tech Stack

- **Language:** Rust (latest stable)
- **MCP SDK:** Use the official MCP Rust SDK from `modelcontextprotocol/rust-sdk` (crate: `rmcp`). Fallback: `rust-mcp-sdk` from crates.io if the official SDK lacks needed features.
- **HTTP client:** `reqwest` with JSON support for calling the ServiceDesk Plus REST API
- **Async runtime:** `tokio`
- **Serialization:** `serde` / `serde_json`
- **Config:** `dotenv` or similar for loading API keys and base URL from environment

## ServiceDesk Plus API Details

- **Base URL:** `https://servicedesk.fynbus.dk/api/v3/`
- **Auth:** Technician API key passed as header (`authtoken` or `technician_key` depending on version). The key should be loaded from an environment variable `SDP_API_KEY` — never hardcoded.
- **API version:** v3 (REST, JSON input/output)
- **Docs reference:** https://help.servicedeskplus.com/api/rest-api.html and https://ui.servicedeskplus.com/APIDocs3/index.html

### Key API Endpoints to Implement

| MCP Tool Name         | SDP API Endpoint            | Description                          |
|-----------------------|-----------------------------|--------------------------------------|
| `list_requests`       | `GET /api/v3/requests`      | List/search tickets with filters     |
| `get_request`         | `GET /api/v3/requests/{id}` | Get full details of a single ticket  |
| `create_request`      | `POST /api/v3/requests`     | Create a new ticket                  |
| `update_request`      | `PUT /api/v3/requests/{id}` | Update an existing ticket            |
| `close_request`       | `PUT /api/v3/requests/{id}` | Close a ticket (status update)       |
| `add_note`            | `POST /api/v3/requests/{id}/notes` | Add a note/comment to a ticket |
| `list_technicians`    | `GET /api/v3/technicians`   | List available technicians           |
| `assign_request`      | `PUT /api/v3/requests/{id}` | Assign a ticket to a technician      |

### SDP API Request Format

Requests use an `input_data` JSON parameter. Example for listing requests with filters:

```json
{
  "list_info": {
    "row_count": 20,
    "start_index": 1,
    "sort_field": "created_time",
    "sort_order": "desc",
    "get_total_count": true,
    "search_criteria": [
      {
        "field": "status.name",
        "condition": "is",
        "value": "Open"
      }
    ]
  }
}
```

## Project Structure

```
glass/
├── CLAUDE.md              # This file
├── Cargo.toml
├── .env.example           # SDP_API_KEY=your_key_here, SDP_BASE_URL=https://servicedesk.fynbus.dk
├── src/
│   ├── main.rs            # Entry point, starts MCP server on stdio
│   ├── server.rs          # MCP server setup, tool registration
│   ├── tools/             # One module per MCP tool
│   │   ├── mod.rs
│   │   ├── list_requests.rs
│   │   ├── get_request.rs
│   │   ├── create_request.rs
│   │   ├── update_request.rs
│   │   ├── add_note.rs
│   │   └── list_technicians.rs
│   ├── sdp_client.rs      # ServiceDesk Plus API client (reqwest wrapper)
│   └── models.rs          # Shared types/structs for SDP API responses
```

## Architecture Decisions

- **Stdio transport only** for now — this is the simplest and works with both Claude Code and Claude Desktop.
- Each MCP tool should have a clear, descriptive name and JSON schema for its parameters so Claude understands what arguments to pass.
- Error handling: Return meaningful error messages from the SDP API back through MCP tool responses so Claude can tell the user what went wrong.
- Keep the SDP client (`sdp_client.rs`) decoupled from MCP — it should be a clean async Rust HTTP client that could be reused independently.

## Development Guidelines

- Write idiomatic Rust — use `Result<T, E>` for error handling, avoid `unwrap()` in production code.
- Use `thiserror` for custom error types.
- Each tool handler should validate inputs before making API calls.
- Add `#[cfg(test)]` unit tests for the SDP client using mock responses.
- Use `tracing` for structured logging.

## Configuration

The binary should read config from environment variables (with `.env` support):

```env
SDP_BASE_URL=https://servicedesk.fynbus.dk
SDP_API_KEY=<technician_api_key>
```

## Claude Code Integration

Once built, register in Claude Code config (`~/.claude/claude_code_config.json` or similar):

```json
{
  "mcpServers": {
    "glass": {
      "command": "/path/to/glass",
      "env": {
        "SDP_BASE_URL": "https://servicedesk.fynbus.dk",
        "SDP_API_KEY": "your_key_here"
      }
    }
  }
}
```

## MVP Milestones

1. **M1:** Scaffold project, get MCP stdio server running with a single `ping` tool
2. **M2:** Implement `sdp_client.rs` with auth and basic `list_requests` / `get_request`
3. **M3:** Wire up all read-only MCP tools (list, get, search)
4. **M4:** Add write tools (create, update, close, add note, assign)
5. **M5:** Error handling polish, input validation, logging
6. **M6:** README, `.env.example`, usage docs

## Important Notes

- Never log or expose the API key in tool responses or error messages.
- The SDP instance is on-premises (not cloud), so the API may differ slightly from ManageEngine's cloud docs. Prefer the on-prem API reference.
- Test against the actual instance carefully — start with read-only operations.
