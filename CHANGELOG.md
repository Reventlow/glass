# Changelog

All notable changes to Glass will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-02-06

Initial release of Glass, an MCP server for ManageEngine ServiceDesk Plus.

### Added

#### MCP Tools
- `ping` - Test connectivity to the Glass MCP server
- `list_requests` - List and filter service desk tickets
  - Filter by status, priority, technician, requester
  - Pagination with limit and offset
- `get_request` - Get full details of a single ticket
  - Includes description, notes, history, resolution
- `create_request` - Create new tickets
  - Required: subject
  - Optional: description, requester, priority, category, subcategory, item, group, technician
- `update_request` - Update existing tickets
  - Modify subject, description, priority, status, category, group, technician
- `close_request` - Close tickets with resolution
  - Optional closure code and comments
- `add_note` - Add notes to tickets
  - Internal or visible to requester
  - Optional technician notification
- `list_technicians` - List technicians for assignment
  - Filter by support group
- `assign_request` - Assign tickets to technicians or groups

#### Core Features
- ServiceDesk Plus API v3 client with automatic retry logic
- Exponential backoff for rate limiting (HTTP 429)
- Automatic retry for server errors (502/503/504)
- Configurable request timeout (30 seconds default)
- Connection test on startup

#### Security
- API key never logged or exposed in error messages
- All error messages sanitized before output
- Environment variable configuration (no hardcoded credentials)

#### Developer Experience
- Comprehensive rustdoc documentation
- Unit tests for all modules
- Input validation with helpful error messages
- Structured logging with tracing (to stderr, not stdout)

### Technical Details

- **Runtime**: Tokio async runtime
- **MCP SDK**: rmcp (official Rust MCP SDK)
- **HTTP Client**: reqwest with timeout and retry
- **Transport**: stdio (for Claude Code and Claude Desktop)

[0.1.0]: https://github.com/your-org/glass/releases/tag/v0.1.0
