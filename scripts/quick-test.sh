#!/bin/bash
#
# Quick test script for llms-fetch-mcp
# Tests the MCP server with a simple HTTP request
#

set -e

echo "Building release binary..."
cargo build --release --quiet

echo ""
echo "Testing fetch functionality..."
echo ""

# Create a temp directory
TEMP_CACHE="/tmp/llms-fetch-test-$$"

echo "Cache directory: $TEMP_CACHE"
echo ""

# Run the server with a simple test
echo "Fetching https://httpbin.org/html..."

# This sends a minimal MCP protocol sequence
{
    # Initialize
    echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}}}'
    sleep 0.5

    # Initialized notification
    echo '{"jsonrpc":"2.0","method":"notifications/initialized"}'
    sleep 0.5

    # Call fetch tool
    echo '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"fetch","arguments":{"url":"https://httpbin.org/html"}}}'
    sleep 2
} | ./target/release/llms-fetch-mcp "$TEMP_CACHE" 2>&1 | head -20

echo ""
echo "Checking cached files..."
if [ -d "$TEMP_CACHE" ]; then
    echo "✓ Cache directory created"
    find "$TEMP_CACHE" -type f ! -name ".gitignore" -exec sh -c 'echo "  File: {}"; wc -l {} | awk "{print \"    Lines:\", \$1}"' \;
else
    echo "✗ Cache directory not found"
fi

# Cleanup
rm -rf "$TEMP_CACHE"

echo ""
echo "Test complete!"
echo ""
echo "To run the server:"
echo "  ./target/release/llms-fetch-mcp [cache-dir]"
echo ""
echo "Default cache directory: .llms-fetch-mcp"
