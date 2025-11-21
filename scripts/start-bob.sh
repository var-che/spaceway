#!/bin/bash
# Start Bob (Linux - Connecting Node)
# This computer will connect to Alice

echo "============================================================"
echo "  Starting Bob (Connecting Node)"
echo "============================================================"
echo ""

echo "🚀 Starting descord..."
echo ""
echo "After startup:"
echo "  1. Get Alice's multiaddr from her terminal (network command)"
echo "  2. Type: connect /ip4/<ALICE_IP>/tcp/9001/p2p/<ALICE_PEER_ID>"
echo "  3. Get space_id and invite code from Alice"
echo "  4. Type: join <space_id> <invite_code>"
echo ""
echo "Example:"
echo "  connect /ip4/192.168.1.100/tcp/9001/p2p/12D3KooWABC123..."
echo "  join 9d2bf8a78ca50c92... ABC12345"
echo ""
echo "Press Ctrl+C to stop"
echo ""

# Start Bob
./target/release/descord --account bob.key
