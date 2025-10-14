# Better Fetch MCP Server

A Model Context Protocol (MCP) server that fetches and caches web content as LLM-friendly Markdown files, with intelligent format detection and conservative HTML cleaning.

## Key Features

### 🎯 Smart Format Detection (Primary Value)
Automatically tries multiple LLM-friendly content variations:
- `/llms-full.txt` - Comprehensive documentation
- `/llms.txt` - Concise documentation
- `.md` suffix - Markdown files
- `/index.md` - Directory markdown
- Original URL - Fallback

### 📦 File-Based Caching
- Saves content to `.better-fetch-mcp/<domain>/<path>`
- Returns file paths (not inline content)
- LLMs can read, search, and analyze cached files
- Automatic `.gitignore` creation
- Optional custom cache directory

### 🧹 Conservative HTML Cleaning
- Removes only clear navigation elements (nav, breadcrumbs, site headers/footers)
- Preserves content over chrome (diagrams, code, tips, warnings, TOCs)
- Converts HTML to Markdown only when needed
- Philosophy: **Better to include too much than remove actual content**

### ⚡ Performance
- Concurrent fetching of URL variations
- Prefers Markdown/text via Accept headers
- Returns file statistics (lines, words, characters)

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
