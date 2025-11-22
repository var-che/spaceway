#!/bin/bash
# Helper script to start a Spaceway peer with proper configuration

set -e

# Colors
CYAN='\033[0;36m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# Default values
NAME=""
PORT=""
CONNECT_TO=""

function show_usage() {
    echo -e "${CYAN}╔════════════════════════════════════════╗${NC}"
    echo -e "${CYAN}║   Spaceway Peer Starter                ║${NC}"
    echo -e "${CYAN}╚════════════════════════════════════════╝${NC}"
    echo ""
    echo "Usage: $0 --name <name> --port <port> [--connect <multiaddr>]"
    echo ""
    echo "Examples:"
    echo "  # Start Alice (first peer)"
    echo "  $0 --name alice --port 9001"
    echo ""
    echo "  # Start Bob (connect to Alice)"
    echo "  $0 --name bob --port 9002 --connect /ip4/127.0.0.1/tcp/9001"
    echo ""
    echo "  # Start Charlie"
    echo "  $0 --name charlie --port 9003 --connect /ip4/127.0.0.1/tcp/9001"
    echo ""
    exit 1
}

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --name)
            NAME="$2"
            shift 2
            ;;
        --port)
            PORT="$2"
            shift 2
            ;;
        --connect)
            CONNECT_TO="$2"
            shift 2
            ;;
        -h|--help)
            show_usage
            ;;
        *)
            echo -e "${RED}Unknown option: $1${NC}"
            show_usage
            ;;
    esac
done

# Validate required arguments
if [ -z "$NAME" ] || [ -z "$PORT" ]; then
    echo -e "${RED}Error: --name and --port are required${NC}"
    echo ""
    show_usage
fi

# Display configuration
echo -e "${CYAN}╔════════════════════════════════════════╗${NC}"
echo -e "${CYAN}║   Starting Spaceway Peer               ║${NC}"
echo -e "${CYAN}╚════════════════════════════════════════╝${NC}"
echo ""
echo -e "${GREEN}Name:${NC}     $NAME"
echo -e "${GREEN}Port:${NC}     $PORT"
echo -e "${GREEN}Account:${NC}  ./${NAME}.key"
if [ -n "$CONNECT_TO" ]; then
    echo -e "${GREEN}Connect:${NC}  $CONNECT_TO"
fi
echo ""
echo -e "${YELLOW}Press Ctrl+C to stop${NC}"
echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

# Check if we're in Nix shell, if not enter it
if [ -z "$IN_NIX_SHELL" ]; then
    echo -e "${YELLOW}Entering Nix development shell for proper dependencies...${NC}"
    echo ""
    exec nix develop --command bash -c "
        cargo +nightly run --bin spaceway -- \
            --account './${NAME}.key' \
            --port '$PORT'
    "
else
    # Already in Nix shell
    exec cargo +nightly run --bin spaceway -- \
        --account "./${NAME}.key" \
        --port "$PORT"
fi
