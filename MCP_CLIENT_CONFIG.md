# Using better-fetch-mcp with MCP Clients

## Claude Code Configuration (Recommended)

Add the server using the CLI:

```bash
# Install the server
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/Crazytieguy/better-fetch-mcp/releases/download/v0.1.0/better-fetch-mcp-installer.sh | sh

# Add to Claude Code
claude mcp add --transport stdio better-fetch -- better-fetch-mcp
```

Or with a custom cache directory:

```bash
claude mcp add --transport stdio better-fetch -- better-fetch-mcp /path/to/custom/cache
```

Verify it's connected:

```bash
claude mcp list
```

## Claude Desktop Configuration

Add this to your Claude Desktop configuration file:

**macOS**: `~/Library/Application Support/Claude/claude_desktop_config.json`
**Windows**: `%APPDATA%\Claude\claude_desktop_config.json`

```json
{
  "mcpServers": {
    "better-fetch": {
      "command": "/Users/yoav/.cargo/bin/better-fetch-mcp"
    }
  }
}
```

Or with a custom cache directory:

```json
{
  "mcpServers": {
    "better-fetch": {
      "command": "/Users/yoav/.cargo/bin/better-fetch-mcp",
      "args": ["/path/to/custom/cache"]
    }
  }
}
```

## Cursor IDE Configuration

Add to your Cursor settings:

```json
{
  "mcp.servers": {
    "better-fetch": {
      "command": "/Users/yoav/.cargo/bin/better-fetch-mcp"
    }
  }
}
```

## Using the Fetch Tool

Once configured, you can use the `fetch` tool in your MCP client:

**Tool**: `fetch`
**Parameters**:
- `url` (string, required): The URL to fetch

**Example usage in Claude Desktop**:
```
Can you fetch https://docs.convex.dev/ for me?
```

Claude will use the better-fetch tool which will:
1. Try multiple URL variations (.md, /index.md, /llms.txt, /llms-full.txt)
2. Cache the content locally
3. Convert HTML to Markdown if needed
4. Return file statistics

**Cache location**: `.better-fetch-mcp/<domain>/<path>`

## Testing the Server Manually

To test the server directly, you can send MCP protocol messages via stdin:

```bash
# Initialize the server
echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0.0"}}}' | better-fetch-mcp

# Call the fetch tool (after initialization)
echo '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"fetch","arguments":{"url":"https://example.com"}}}' | better-fetch-mcp
```

Note: The server maintains a persistent connection in production use with MCP clients.
