# Glass Setup Guide — New Machine

How to set up Glass on a new computer with Claude Code.

## Prerequisites

- Rust toolchain (1.91+) — install via [rustup](https://rustup.rs/)
- Network access to your ServiceDesk Plus instance (HTTPS)
- A technician API key from ServiceDesk Plus

## Step 1: Install Rust (if needed)

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env
```

## Step 2: Clone and build

```bash
git clone git@github.com:Reventlow/glass.git
cd glass
cargo build --release
```

The binary will be at `target/release/glass` (~9 MB).

## Step 3: Get your SDP API key

1. Log into ServiceDesk Plus as a technician
2. Go to **Admin** > **Developers** > **API** (or **Technician API Key**)
3. Generate a new key or copy your existing key

Keep this key secure — it provides the same access as the technician account.

## Step 4: Configure environment

Option A — `.env` file:

```bash
cd /path/to/glass
cp .env.example .env
```

Edit `.env`:

```env
SDP_BASE_URL=https://servicedesk.example.com
SDP_API_KEY=your-api-key-here
```

Option B — pass env vars directly (see Step 5).

## Step 5: Register as MCP server in Claude Code

```bash
claude mcp add glass \
  --scope user \
  -e SDP_BASE_URL="https://servicedesk.example.com" \
  -e SDP_API_KEY="your-api-key-here" \
  -- /path/to/glass/target/release/glass
```

Or if using a `.env` file, set the working directory instead:

```bash
claude mcp add glass \
  --scope user \
  -- /path/to/glass/target/release/glass
```

Then edit the MCP config to add `"cwd": "/path/to/glass"`.

Alternatively, add manually to `~/.claude.json`:

```json
{
  "mcpServers": {
    "glass": {
      "command": "/path/to/glass/target/release/glass",
      "env": {
        "SDP_BASE_URL": "https://servicedesk.example.com",
        "SDP_API_KEY": "your-api-key-here"
      }
    }
  }
}
```

## Step 6: Verify

Start a new Claude Code session and test:

1. Use the `ping` tool — should return "pong"
2. Try `list_requests` — should return tickets from your SDP instance
3. Try `list_technicians` — should return technician names and IDs

## Pre-built binary (alternative)

If you don't want to install Rust on the target machine, you can cross-compile or copy the binary:

```bash
# On the build machine
cargo build --release
scp target/release/glass user@target-machine:/usr/local/bin/
```

The binary is statically linked enough to run on most Linux systems without extra dependencies. For other platforms (macOS, Windows), build on that platform or use `cross`:

```bash
cargo install cross
cross build --release --target x86_64-unknown-linux-gnu
```

## Troubleshooting

### "Connection test failed" at startup

Glass tests connectivity on start. If this appears:

- Verify `SDP_BASE_URL` is correct and includes `https://`
- Check network access: `curl -I https://servicedesk.example.com`
- Verify the API key: `curl -s -H "technician_key: YOUR_KEY" "https://servicedesk.example.com/api/v3/requests?input_data={}" | head`
- If behind a VPN, make sure it's connected

### Authentication errors

- Check the API key is valid and the technician account is active
- Keys can expire — generate a new one if needed
- Ensure the technician has sufficient permissions for the operations you need

### Glass not showing in Claude Code

- Restart Claude Code after changing MCP config
- Check the binary path is absolute and correct
- Verify the binary is executable: `chmod +x /path/to/glass`
- Check `claude mcp list` to see if Glass is registered

### Enable debug logging

Add `RUST_LOG` to the MCP env config:

```json
{
  "env": {
    "SDP_BASE_URL": "...",
    "SDP_API_KEY": "...",
    "RUST_LOG": "glass=debug"
  }
}
```

## Security notes

- Never commit `.env` files or API keys to version control
- Use a dedicated technician account with minimal permissions
- Always use HTTPS — Glass warns if HTTP is configured
- The API key is never logged, even at debug/trace level
- Rotate keys periodically
