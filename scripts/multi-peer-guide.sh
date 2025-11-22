#!/bin/bash
# Quick start script for 3-peer local testing
# This script helps you quickly test P2P functionality

CYAN='\033[0;36m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

clear
echo -e "${CYAN}╔══════════════════════════════════════════════════════════════╗${NC}"
echo -e "${CYAN}║        Spaceway Multi-Peer Testing - Quick Start            ║${NC}"
echo -e "${CYAN}╚══════════════════════════════════════════════════════════════╝${NC}"
echo ""
echo -e "${GREEN}This will guide you through testing P2P functionality${NC}"
echo ""
echo "You'll need to open 3 terminal windows/tabs:"
echo ""

echo -e "${YELLOW}┌─ Terminal 1 (Alice) ─────────────────────────────────────┐${NC}"
echo "  cd $PWD"
echo "  ./scripts/start-peer.sh --name alice --port 9001"
echo ""
echo "  Then in Alice's CLI:"
echo "    space create \"TestSpace\""
echo "    channel create \"general\""
echo "    thread create \"Hello World\""
echo "    send \"Hi from Alice!\""
echo "    invite create    # Copy the invite code"
echo "    network          # Note Alice's peer ID"
echo -e "${YELLOW}└──────────────────────────────────────────────────────────┘${NC}"
echo ""

echo -e "${YELLOW}┌─ Terminal 2 (Bob) ───────────────────────────────────────┐${NC}"
echo "  cd $PWD"
echo "  ./scripts/start-peer.sh --name bob --port 9002"
echo ""
echo "  Then in Bob's CLI:"
echo "    connect /ip4/127.0.0.1/tcp/9001"
echo "    join <space_id> <invite_code>    # Use Alice's invite"
echo "    space <space_id>"
echo "    send \"Hi from Bob!\""
echo -e "${YELLOW}└──────────────────────────────────────────────────────────┘${NC}"
echo ""

echo -e "${YELLOW}┌─ Terminal 3 (Charlie) ───────────────────────────────────┐${NC}"
echo "  cd $PWD"
echo "  ./scripts/start-peer.sh --name charlie --port 9003"
echo ""
echo "  Then in Charlie's CLI:"
echo "    connect /ip4/127.0.0.1/tcp/9001"
echo "    join <space_id> <invite_code>    # Use Alice's invite"
echo "    space <space_id>"
echo "    send \"Hi from Charlie!\""
echo -e "${YELLOW}└──────────────────────────────────────────────────────────┘${NC}"
echo ""

echo -e "${GREEN}📝 Helpful Commands:${NC}"
echo "  help      - Show all available commands"
echo "  whoami    - Show current user info"
echo "  network   - Show network status and peers"
echo "  context   - Show current space/channel/thread"
echo "  spaces    - List all spaces"
echo "  messages  - Show messages in current thread"
echo ""

echo -e "${GREEN}🧪 Alternative: Run Automated Test${NC}"
echo "  cargo +nightly test --package spaceway-core --test beta_test -- --ignored --nocapture"
echo ""

echo -e "${CYAN}Ready to start? Open your first terminal now!${NC}"
echo ""
