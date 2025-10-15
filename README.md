# llms-fetch-mcp

MCP server that fetches web content in LLM-friendly formats. Automatically discovers and uses [llms.txt](https://llmstxt.org/) files when available, tries Markdown versions, and falls back to clean HTML-to-Markdown conversion.

## Quick Start

Add to your MCP client configuration:

### Claude Desktop / Claude Code

```json
{
  "mcpServers": {
    "llms-fetch": {
      "command": "npx",
      "args": ["-y", "llms-fetch-mcp"]
    }
  }
}
```

### Cursor IDE

```json
{
  "mcp.servers": {
    "llms-fetch": {
      "command": "npx",
      "args": ["-y", "llms-fetch-mcp"]
    }
  }
}
```

## How It Works

When you fetch a URL, the server tries multiple sources in parallel:

1. `https://example.com/llms-full.txt` - Comprehensive LLM documentation
2. `https://example.com/llms.txt` - Concise LLM documentation
3. `https://example.com.md` - Markdown version
4. `https://example.com/index.md` - Directory Markdown
5. `https://example.com` - Original URL (converts HTML to Markdown if needed)

Content is cached locally in `.llms-fetch-mcp/` for quick access.

## Why llms.txt?

[llms.txt](https://llmstxt.org/) is an emerging standard for websites to provide LLM-optimized documentation. Sites like FastHTML, Anthropic Docs, and others are adopting it. This server automatically discovers and uses these files when available, giving you cleaner, more concise content than HTML scraping.

## Installation

If you prefer installing instead of using `npx`:

### Shell (macOS/Linux)
```bash
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/Crazytieguy/llms-fetch-mcp/releases/latest/download/llms-fetch-mcp-installer.sh | sh
```

### PowerShell (Windows)
```powershell
irm https://github.com/Crazytieguy/llms-fetch-mcp/releases/latest/download/llms-fetch-mcp-installer.ps1 | iex
```

### Homebrew
```bash
brew install Crazytieguy/tap/llms-fetch-mcp
```

### npm
```bash
npm install -g llms-fetch-mcp
```

### Cargo
```bash
cargo install llms-fetch-mcp
```

Then use the binary directly instead of `npx`:
```json
{
  "mcpServers": {
    "llms-fetch": {
      "command": "llms-fetch-mcp"
    }
  }
}
```

## Custom Cache Directory

```json
{
  "mcpServers": {
    "llms-fetch": {
      "command": "llms-fetch-mcp",
      "args": ["/path/to/custom/cache"]
    }
  }
}
```

## License

MIT
