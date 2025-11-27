#!/bin/bash
# Quick test to verify MLS commands are available

echo "Testing MLS CLI commands..."
echo ""
echo "=== Creating test account ==="
echo ""

# Start spaceway, send help, then exit
echo -e "help\nexit" | timeout 5 cargo run --release -- --account test.key 2>&1 | grep -A 20 "MLS Encryption"

echo ""
echo "=== Test complete ==="
