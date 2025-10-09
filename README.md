# Better Fetch MCP Server

A high-quality Model Context Protocol (MCP) server for fetching and caching web content, with intelligent URL variation detection and automatic HTML-to-Markdown conversion.

## Features

- **Smart URL Variations**: Automatically tries multiple content variations:
  - Original URL
  - `.md` suffix
  - `/index.md`
  - `/llms.txt`
  - `/llms-full.txt`

- **Intelligent Caching**:
  - Saves content to `.better-fetch-mcp/<domain>/<path>`
  - Only saves HTML-to-Markdown if single HTML response received
  - Preserves native Markdown when server returns it
  - Saves all successful variations otherwise
  - Automatic `.gitignore` creation on first use

- **Content Processing**:
  - Prefers Markdown/text content via Accept headers
  - Automatic HTML-to-Markdown conversion (only when needed)
  - Detects and preserves Markdown content-type responses
  - Returns file statistics (lines, words, characters)
  - Concurrent fetching for optimal performance

- **Configurable**:
  - Optional custom cache directory via command-line argument
  - Respects `.md` and `.txt` URLs (no variations tried)

## Installation

```bash
cargo build --release
```

## Usage

### As an MCP Server

Run with default cache directory (`.better-fetch-mcp`):
```bash
./target/release/better-fetch-mcp
```

Run with custom cache directory:
```bash
./target/release/better-fetch-mcp /path/to/cache
```

### Tool: `fetch`

Fetches content from a URL and caches it locally.

**Parameters:**
- `url` (string, required): The URL to fetch

**Returns:**
```json
{
  "files": [
    {
      "path": ".better-fetch-mcp/example.com/docs",
      "lines": 150,
      "words": 1200,
      "characters": 8500
    }
  ]
}
```

## Behavior

### URL Ending with `.md` or `.txt`
Only the exact URL is fetched:
```
https://example.com/page.md → only fetches this URL
```

### Regular URLs
Multiple variations are tried concurrently:
```
https://example.com/docs →
  - https://example.com/docs
  - https://example.com/docs.md
  - https://example.com/docs/index.md
  - https://example.com/docs/llms.txt
  - https://example.com/docs/llms-full.txt
```

### Content Processing

**Single HTML Response:**
- Converts HTML to Markdown
- Saves as original URL path

**Multiple Successful Responses:**
- Saves each variation to its own file
- Preserves original content format

## Development

### Run Tests
```bash
# Unit tests
cargo test

# Integration tests (requires network)
cargo test -- --ignored

# Test real website fetches
cargo run --example test_fetch
```

### Code Quality
```bash
# Run clippy with pedantic lints
cargo clippy --all-targets -- -D warnings

# Format code
cargo fmt
```

## Architecture

- **Framework**: Built with `rmcp` (official Rust MCP SDK)
- **HTTP Client**: `reqwest` with `rustls-tls`
- **HTML Conversion**: `html2md`
- **Async Runtime**: `tokio`

## License

MIT
