#!/bin/bash

# Manual test script for better-fetch-mcp
# This tests real website fetches

set -e

CACHE_DIR="/tmp/better-fetch-test-$$"

echo "Testing better-fetch-mcp..."
echo "Cache directory: $CACHE_DIR"

# Build the project
cargo build --release

# Test 1: Fetch docs.convex.dev
echo ""
echo "Test 1: Fetching https://docs.convex.dev/"
echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}}}' | ./target/release/better-fetch-mcp "$CACHE_DIR" > /dev/null 2>&1 &
PID=$!
sleep 1

echo '{"jsonrpc":"2.0","method":"notifications/initialized"}' | ./target/release/better-fetch-mcp "$CACHE_DIR" > /dev/null 2>&1

echo '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"fetch","arguments":{"url":"https://docs.convex.dev/"}}}' | ./target/release/better-fetch-mcp "$CACHE_DIR"

kill $PID 2>/dev/null || true

# Check results
if [ -d "$CACHE_DIR/docs.convex.dev" ]; then
    echo "✓ Cache directory created"
    FILE_COUNT=$(find "$CACHE_DIR/docs.convex.dev" -type f | wc -l)
    echo "✓ Found $FILE_COUNT cached file(s)"
else
    echo "✗ Cache directory not created"
fi

if [ -f "$CACHE_DIR/.gitignore" ]; then
    echo "✓ .gitignore created"
else
    echo "✗ .gitignore not created"
fi

# Clean up
rm -rf "$CACHE_DIR"

echo ""
echo "Tests completed!"
