# Contributing to Better Fetch MCP

## Development

### Run Tests
```bash
# Unit tests
cargo test

# Integration tests (requires network)
cargo test -- --ignored

# Test HTML cleaning
cargo run --example test_html_cleaning
```

### Code Quality
```bash
# Run clippy with pedantic lints
cargo clippy --all-targets -- -D warnings

# Format code
cargo fmt
```

### Quick Testing Script
```bash
# Test MCP protocol integration
./scripts/quick-test.sh
```

## Architecture

- **Framework**: Built with `rmcp` (official Rust MCP SDK)
- **HTTP Client**: `reqwest` with `rustls-tls`
- **HTML Conversion**: `html2md`
- **Async Runtime**: `tokio`
- **Regex**: Standard library `OnceLock` for compiled patterns
