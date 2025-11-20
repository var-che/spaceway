#!/bin/bash
# Descord Beta Test Runner (Linux/macOS)

set -e

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
GRAY='\033[0;37m'
NC='\033[0m' # No Color

function print_header() {
    echo ""
    echo -e "${CYAN}╔═══════════════════════════════════════════════════════════════════╗${NC}"
    echo -e "${CYAN}║           DESCORD BETA TEST AUTOMATION SCRIPT                    ║${NC}"
    echo -e "${CYAN}╚═══════════════════════════════════════════════════════════════════╝${NC}"
    echo ""
}

function check_relay() {
    if curl -s http://localhost:8080/stats > /dev/null 2>&1; then
        return 0
    else
        return 1
    fi
}

function show_relay_status() {
    if check_relay; then
        echo -e "${GREEN}✅ Relay server is running${NC}"
        echo ""
        
        stats=$(curl -s http://localhost:8080/stats)
        echo -e "${CYAN}📊 Relay Statistics:${NC}"
        echo "$stats" | jq '.'
        echo ""
    else
        echo -e "${RED}❌ Relay server is not running${NC}"
        echo ""
        echo -e "${YELLOW}To start relay server:${NC}"
        echo -e "${GRAY}  cargo run --package descord-relay --release${NC}"
        echo ""
    fi
}

function run_beta_test() {
    if ! check_relay; then
        echo -e "${YELLOW}⚠️  WARNING: Relay server not detected!${NC}"
        echo ""
        echo -e "${YELLOW}Beta test requires relay server running.${NC}"
        echo ""
        echo "Start relay in another terminal:"
        echo "  cargo run --package descord-relay --release"
        echo ""
        read -p "Continue anyway? (y/n) " -n 1 -r
        echo ""
        if [[ ! $REPLY =~ ^[Yy]$ ]]; then
            return
        fi
    fi
    
    echo -e "${YELLOW}🧪 Running automated beta test...${NC}"
    echo ""
    echo -e "${GRAY}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo ""
    
    if cargo test --package descord-core --test beta_test -- --ignored --nocapture; then
        echo ""
        echo -e "${GRAY}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
        echo ""
        echo -e "${GREEN}✅ BETA TEST PASSED!${NC}"
    else
        echo ""
        echo -e "${GRAY}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
        echo ""
        echo -e "${RED}❌ BETA TEST FAILED${NC}"
    fi
    
    echo ""
}

function run_unit_tests() {
    echo -e "${YELLOW}🧪 Running all unit tests...${NC}"
    echo ""
    
    if cargo test --package descord-core --lib -- --test-threads=1; then
        echo ""
        echo -e "${GREEN}✅ ALL UNIT TESTS PASSED!${NC}"
    else
        echo ""
        echo -e "${RED}❌ SOME TESTS FAILED${NC}"
    fi
    
    echo ""
}

function run_integration_tests() {
    echo -e "${YELLOW}🧪 Running all integration tests...${NC}"
    echo ""
    
    if cargo test --package descord-core --test '*' --test-threads=1; then
        echo ""
        echo -e "${GREEN}✅ ALL INTEGRATION TESTS PASSED!${NC}"
    else
        echo ""
        echo -e "${RED}❌ SOME TESTS FAILED${NC}"
    fi
    
    echo ""
}

function show_menu() {
    echo -e "${NC}📋 Select Beta Test Option:${NC}"
    echo ""
    echo "  1. Run Beta Test (automated 3-user simulation)"
    echo "  2. Run All Unit Tests (107 tests)"
    echo "  3. Run All Integration Tests"
    echo "  4. Check Relay Status"
    echo "  5. View Relay Stats (JSON)"
    echo "  6. Start Relay Server (background)"
    echo "  7. Exit"
    echo ""
}

# Main loop
while true; do
    clear
    print_header
    show_menu
    
    read -p "Enter choice (1-7): " choice
    echo ""
    
    case $choice in
        1)
            run_beta_test
            read -p "Press Enter to continue..."
            ;;
        2)
            run_unit_tests
            read -p "Press Enter to continue..."
            ;;
        3)
            run_integration_tests
            read -p "Press Enter to continue..."
            ;;
        4)
            show_relay_status
            read -p "Press Enter to continue..."
            ;;
        5)
            if check_relay; then
                curl -s http://localhost:8080/stats | jq '.'
                echo ""
            else
                echo -e "${RED}❌ Relay not running${NC}"
                echo ""
            fi
            read -p "Press Enter to continue..."
            ;;
        6)
            echo -e "${YELLOW}🚀 Starting relay server in background...${NC}"
            cargo run --package descord-relay --release > relay.log 2>&1 &
            RELAY_PID=$!
            echo "Relay PID: $RELAY_PID"
            echo "Log file: relay.log"
            echo ""
            sleep 3
            show_relay_status
            read -p "Press Enter to continue..."
            ;;
        7)
            echo -e "${CYAN}👋 Goodbye!${NC}"
            echo ""
            exit 0
            ;;
        *)
            echo -e "${RED}❌ Invalid choice${NC}"
            echo ""
            read -p "Press Enter to continue..."
            ;;
    esac
done
