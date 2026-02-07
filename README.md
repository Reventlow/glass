# Glass

MCP server for ManageEngine ServiceDesk Plus.

Glass exposes ServiceDesk Plus operations as [Model Context Protocol (MCP)](https://modelcontextprotocol.io/) tools, enabling AI assistants like Claude to manage help desk tickets through natural language.

## Features

- **List and search tickets** - Filter by status, priority, technician, or date range
- **Get ticket details** - View complete information including description, notes, and history
- **Create tickets** - Open new support requests with full metadata
- **Update tickets** - Modify priority, status, category, and assignments
- **Close tickets** - Close with resolution codes and comments
- **Add notes** - Post internal or public comments to tickets
- **List technicians** - Find technicians by group for assignments
- **Assign tickets** - Route tickets to technicians or support groups
- **Connection health** - Ping tool to verify server connectivity

## Installation

### From source

```bash
git clone https://github.com/Reventlow/glass.git
cd glass
cargo build --release
```

The binary will be at `target/release/glass`.

## Configuration

Glass requires two environment variables:

| Variable | Required | Description |
|----------|----------|-------------|
| `SDP_BASE_URL` | Yes | Base URL of your ServiceDesk Plus instance (e.g., `https://servicedesk.example.com`) |
| `SDP_API_KEY` | Yes | Technician API key for authentication |
| `RUST_LOG` | No | Log level: `error`, `warn`, `info`, `debug`, `trace` (default: `glass=info`) |

### Getting your API key

1. Log into ServiceDesk Plus as a technician
2. Go to **Admin** > **Developers** > **API** (or **Technician API Key**)
3. Generate a new key or copy your existing key
4. Store securely - this key provides full access to your account's permissions

### Using a .env file

Copy `.env.example` to `.env` and fill in your values:

```bash
cp .env.example .env
# Edit .env with your configuration
```

## Usage with Claude Code

Add Glass to your Claude Code configuration (`~/.claude/claude_code_config.json`):

```json
{
  "mcpServers": {
    "glass": {
      "command": "/path/to/glass",
      "env": {
        "SDP_BASE_URL": "https://servicedesk.example.com",
        "SDP_API_KEY": "your-api-key-here"
      }
    }
  }
}
```

Or, if using a `.env` file in the Glass directory:

```json
{
  "mcpServers": {
    "glass": {
      "command": "/path/to/glass",
      "cwd": "/path/to/glass-directory"
    }
  }
}
```

## Usage with Claude Desktop

Add to your Claude Desktop configuration (`~/Library/Application Support/Claude/claude_desktop_config.json` on macOS):

```json
{
  "mcpServers": {
    "glass": {
      "command": "/path/to/glass",
      "env": {
        "SDP_BASE_URL": "https://servicedesk.example.com",
        "SDP_API_KEY": "your-api-key-here"
      }
    }
  }
}
```

## Available Tools

| Tool | Description | Required Parameters |
|------|-------------|---------------------|
| `ping` | Test connectivity to the Glass server | None |
| `list_requests` | List/search tickets with filters | None (all optional filters) |
| `get_request` | Get full ticket details | `request_id` |
| `create_request` | Create a new ticket | `subject` |
| `update_request` | Update ticket properties | `request_id` + at least one field |
| `close_request` | Close a ticket | `request_id` |
| `add_note` | Add a note to a ticket | `request_id`, `content` |
| `list_technicians` | List technicians for assignment | None |
| `assign_request` | Assign ticket to technician/group | `request_id` + `technician_id` or `group` |

## Example Conversations

### Listing open tickets

> "Show me all open high-priority tickets"

Glass will use `list_requests` with `status: "Open"` and `priority: "High"` filters.

### Creating a ticket

> "Create a ticket: Printer on 3rd floor not working. It's urgent and assign it to IT Support."

Glass will use `create_request` with the subject, priority, and group.

### Updating and closing

> "Mark ticket #12345 as resolved and close it with the comment 'Replaced toner cartridge'"

Glass will use `close_request` with the closure comments.

## Retry and Error Handling

Glass automatically retries transient failures:

- **Rate limiting (HTTP 429)**: Exponential backoff starting at 100ms
- **Server errors (502/503/504)**: Single retry after 500ms
- **Timeouts**: Single retry

Non-transient errors (authentication failures, validation errors, not found) are not retried.

## Security

### API Key Protection

- The API key is **never logged** at any log level
- Error messages are sanitized to remove any API key occurrences
- The key is stored only in memory, loaded from environment variables

### Best Practices

1. **Never commit API keys** - Use environment variables or `.env` files
2. **Restrict API key permissions** - Create a dedicated technician account with minimal required permissions
3. **Rotate keys regularly** - Generate new keys periodically and update your configuration
4. **Use HTTPS only** - ServiceDesk Plus should always be accessed over HTTPS

### Network Security

Glass communicates with your ServiceDesk Plus instance over HTTPS. Ensure your SDP instance:

- Has a valid TLS certificate
- Is accessible from where you run Glass
- Has appropriate firewall rules

## Troubleshooting

### Connection test fails on startup

Glass tests connectivity when starting. If you see "Connection test failed":

1. Verify `SDP_BASE_URL` is correct and includes `https://`
2. Check the API key is valid (test in a browser or curl)
3. Ensure network connectivity to the SDP server

### Authentication errors

```
authentication failed - check SDP_API_KEY
```

- Verify the API key is correct
- Check the technician account is active
- Ensure the key hasn't expired

### Request not found

```
request not found: 12345
```

- Verify the ticket ID exists
- Check the technician has permission to view the ticket

### Enable debug logging

```bash
RUST_LOG=glass=debug ./glass
```

This will show API requests and responses (with API key redacted).

## Development

### Building

```bash
cargo build
```

### Running tests

```bash
cargo test
```

### Checking code quality

```bash
cargo clippy
cargo fmt --check
```

### Generating documentation

```bash
cargo doc --no-deps --open
```

## Architecture

```
glass/
├── src/
│   ├── main.rs         # Entry point, environment loading
│   ├── config.rs       # Configuration from environment
│   ├── error.rs        # Error types with sanitization
│   ├── sdp_client.rs   # ServiceDesk Plus HTTP client
│   ├── server.rs       # MCP server and tool implementations
│   ├── models/         # SDP API data models
│   │   ├── common.rs   # Shared types, pagination
│   │   ├── request.rs  # Ticket/request models
│   │   ├── technician.rs
│   │   └── note.rs
│   └── tools/
│       └── inputs.rs   # Tool input parameter structs
```

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes with tests
4. Run `cargo clippy` and `cargo fmt`
5. Submit a pull request

## License

MIT License - see [LICENSE](LICENSE) for details.

## Acknowledgments

- [ManageEngine ServiceDesk Plus](https://www.manageengine.com/products/service-desk/) for the help desk platform
- [Model Context Protocol](https://modelcontextprotocol.io/) for the protocol specification
- [rmcp](https://github.com/modelcontextprotocol/rust-sdk) for the Rust MCP SDK
