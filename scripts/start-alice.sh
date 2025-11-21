#!/bin/bash
# Start Alice (Linux - Listening Node)
# This computer will accept incoming connections

echo "============================================================"
echo "  Starting Alice (Listening Node)"
echo "============================================================"
echo ""

# Get local IP address
LOCAL_IP=$(ip -4 addr show | grep -oP '(?<=inet\s)\d+(\.\d+){3}' | grep -v '127.0.0.1' | head -n1)

if [ -n "$LOCAL_IP" ]; then
    echo "📡 Your IP Address: $LOCAL_IP"
    echo ""
else
    echo "⚠️  Could not detect IP address - check manually with 'ip addr'"
    echo ""
fi

echo "🚀 Starting descord on port 9001..."
echo ""
echo "After startup:"
echo "  1. Type 'network' to see your full multiaddr"
echo "  2. Share the multiaddr with Bob (replace 0.0.0.0 with $LOCAL_IP)"
echo "  3. Type 'space MySpace' to create a space"
echo "  4. Type 'invite' to create an invite code"
echo ""
echo "Press Ctrl+C to stop"
echo ""

# Start Alice
./target/release/descord --account alice.key --port 9001
