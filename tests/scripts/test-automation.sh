#!/bin/bash
# Automated Test Script for Spaceway MLS E2EE Flow
# This script runs Alice and Bob in separate processes and automates the complete workflow

set -e  # Exit on error

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
ALICE_PORT=9001
BOB_PORT=9002
ALICE_ACCOUNT="alice.key"
BOB_ACCOUNT="bob.key"
SPACE_NAME="automated-test"

# Output files
ALICE_LOG="alice_output.log"
BOB_LOG="bob_output.log"
TEST_RESULTS="test_results.txt"
ALICE_FIFO="/tmp/alice_input.fifo"
BOB_FIFO="/tmp/bob_input.fifo"

# Cleanup function
cleanup() {
    echo -e "${YELLOW}Cleaning up...${NC}"
    pkill -f "spaceway.*alice.key" 2>/dev/null || true
    pkill -f "spaceway.*bob.key" 2>/dev/null || true
    rm -f $ALICE_FIFO $BOB_FIFO
    echo -e "${GREEN}Cleanup complete${NC}"
}

# Set up cleanup on exit
trap cleanup EXIT

# Clean old data
echo -e "${BLUE}=== Cleaning old test data ===${NC}"
rm -rf *-data/ *.key *.history $ALICE_LOG $BOB_LOG $TEST_RESULTS 2>/dev/null || true

# Build the project
echo -e "${BLUE}=== Building Spaceway ===${NC}"
cargo +nightly build --release 2>&1 | tail -5

echo -e "${GREEN}Build complete!${NC}"
echo ""

# Create named pipes for input
mkfifo $ALICE_FIFO || true
mkfifo $BOB_FIFO || true

# Start Alice in background
echo -e "${BLUE}=== Starting Alice (port $ALICE_PORT) ===${NC}"
./target/release/spaceway --account $ALICE_ACCOUNT --port $ALICE_PORT < $ALICE_FIFO > $ALICE_LOG 2>&1 &
ALICE_PID=$!
exec 3>$ALICE_FIFO  # Keep FIFO open

sleep 3  # Wait for Alice to start

# Start Bob in background
echo -e "${BLUE}=== Starting Bob (port $BOB_PORT) ===${NC}"
./target/release/spaceway --account $BOB_ACCOUNT --port $BOB_PORT < $BOB_FIFO > $BOB_LOG 2>&1 &
BOB_PID=$!
exec 4>$BOB_FIFO  # Keep FIFO open

sleep 3  # Wait for Bob to start

echo -e "${GREEN}Both clients started!${NC}"
echo "  Alice PID: $ALICE_PID"
echo "  Bob PID: $BOB_PID"
echo ""

# Helper function to send command and wait
send_command() {
    local client=$1
    local command=$2
    local wait_time=${3:-2}
    
    if [ "$client" == "alice" ]; then
        echo "$command" >&3
        echo -e "${YELLOW}[Alice]${NC} > $command"
    else
        echo "$command" >&4
        echo -e "${YELLOW}[Bob]${NC} > $command"
    fi
    
    sleep $wait_time
}

# Extract value from log using grep
extract_value() {
    local log_file=$1
    local pattern=$2
    local field=$3
    
    grep "$pattern" "$log_file" | tail -1 | awk "{print \$$field}"
}

# Start automated test sequence
echo -e "${BLUE}=== Starting Automated Test Sequence ===${NC}"
echo "" | tee $TEST_RESULTS

# Step 1: Alice and Bob publish KeyPackages
echo -e "${GREEN}Step 1: Publishing KeyPackages${NC}" | tee -a $TEST_RESULTS
send_command alice "keypackage publish" 5
send_command bob "keypackage publish" 5

# Step 2: Alice creates space
echo -e "${GREEN}Step 2: Alice creates space${NC}" | tee -a $TEST_RESULTS
send_command alice "space create $SPACE_NAME" 3

# Extract space ID from Alice's log
SPACE_ID=$(grep "Created space:" $ALICE_LOG | tail -1 | grep -o '[0-9a-f]\{16\}' | head -1)
if [ -z "$SPACE_ID" ]; then
    echo -e "${RED}Failed to extract Space ID${NC}" | tee -a $TEST_RESULTS
    exit 1
fi
echo "  Space ID: $SPACE_ID" | tee -a $TEST_RESULTS

# Step 3: Alice gets her full space ID
send_command alice "spaces" 2
FULL_SPACE_ID=$(grep -A 5 "Spaces (" $ALICE_LOG | grep "$SPACE_ID" | grep -o '[0-9a-f]\{64\}' || echo "")
if [ -z "$FULL_SPACE_ID" ]; then
    # Fallback: construct from what we know (space IDs are 32 bytes)
    # We'll need to extract it differently
    send_command alice "context" 2
    FULL_SPACE_ID=$(grep "Space:" $ALICE_LOG | tail -1 | awk '{print $2}')
fi
echo "  Full Space ID: $FULL_SPACE_ID" | tee -a $TEST_RESULTS

# Step 4: Alice creates invite
echo -e "${GREEN}Step 3: Alice creates invite${NC}" | tee -a $TEST_RESULTS
send_command alice "invite create" 3

# Extract invite code
INVITE_CODE=$(grep "Created invite code:" $ALICE_LOG | tail -1 | awk '{print $NF}')
if [ -z "$INVITE_CODE" ]; then
    echo -e "${RED}Failed to extract invite code${NC}" | tee -a $TEST_RESULTS
    exit 1
fi
echo "  Invite Code: $INVITE_CODE" | tee -a $TEST_RESULTS

# Step 5: Get Alice's peer ID for Bob to connect
ALICE_PEER_ID=$(grep "Local peer ID:" $ALICE_LOG | tail -1 | awk '{print $NF}')
echo "  Alice Peer ID: $ALICE_PEER_ID" | tee -a $TEST_RESULTS

# Step 6: Bob connects to Alice
echo -e "${GREEN}Step 4: Bob connects to Alice${NC}" | tee -a $TEST_RESULTS
ALICE_MULTIADDR="/ip4/127.0.0.1/tcp/$ALICE_PORT/p2p/$ALICE_PEER_ID"
send_command bob "connect $ALICE_MULTIADDR" 3

# Step 7: Bob joins space with invite
echo -e "${GREEN}Step 5: Bob joins space with invite${NC}" | tee -a $TEST_RESULTS
send_command bob "join $FULL_SPACE_ID $INVITE_CODE" 5

# Step 8: Bob checks his user ID
echo -e "${GREEN}Step 6: Getting Bob's User ID${NC}" | tee -a $TEST_RESULTS
send_command bob "whoami" 2

# Extract Bob's full user ID
BOB_USER_ID=$(grep "^User ID:" $BOB_LOG | tail -1 | awk '{print $NF}')
if [ -z "$BOB_USER_ID" ]; then
    echo -e "${RED}Failed to extract Bob's User ID${NC}" | tee -a $TEST_RESULTS
    exit 1
fi
echo "  Bob User ID: $BOB_USER_ID" | tee -a $TEST_RESULTS

# Step 9: Alice checks members
echo -e "${GREEN}Step 7: Alice checks members${NC}" | tee -a $TEST_RESULTS
send_command alice "members" 3

# Step 10: Alice adds Bob to MLS group
echo -e "${GREEN}Step 8: Alice adds Bob to MLS encryption group${NC}" | tee -a $TEST_RESULTS
send_command alice "member add $BOB_USER_ID" 5

# Step 11: Check if Bob received Welcome message
echo -e "${GREEN}Step 9: Checking if Bob received MLS Welcome message${NC}" | tee -a $TEST_RESULTS
sleep 3

if grep -q "Received MLS Welcome message" $BOB_LOG; then
    echo -e "${GREEN}✓ Bob received MLS Welcome message!${NC}" | tee -a $TEST_RESULTS
else
    echo -e "${RED}✗ Bob did NOT receive MLS Welcome message${NC}" | tee -a $TEST_RESULTS
fi

if grep -q "Joined MLS group" $BOB_LOG; then
    echo -e "${GREEN}✓ Bob joined MLS group!${NC}" | tee -a $TEST_RESULTS
else
    echo -e "${RED}✗ Bob did NOT join MLS group${NC}" | tee -a $TEST_RESULTS
fi

# Step 12: Bob lists spaces
echo -e "${GREEN}Step 10: Bob lists spaces${NC}" | tee -a $TEST_RESULTS
send_command bob "space list" 2

if grep -q "$SPACE_NAME" $BOB_LOG; then
    echo -e "${GREEN}✓ Bob can see the space '$SPACE_NAME'!${NC}" | tee -a $TEST_RESULTS
else
    echo -e "${RED}✗ Bob cannot see the space${NC}" | tee -a $TEST_RESULTS
fi

# Step 13: Alice creates a channel
echo -e "${GREEN}Step 11: Alice creates channel${NC}" | tee -a $TEST_RESULTS
send_command alice "channel create general" 3

# Step 14: Alice creates a thread
echo -e "${GREEN}Step 12: Alice creates thread${NC}" | tee -a $TEST_RESULTS
send_command alice "thread create 'Test Thread'" 3

# Step 15: Alice sends a message
echo -e "${GREEN}Step 13: Alice sends a message${NC}" | tee -a $TEST_RESULTS
send_command alice "send Hello Bob! This is a test message." 3

# Step 16: Bob switches to the space
echo -e "${GREEN}Step 14: Bob switches to space${NC}" | tee -a $TEST_RESULTS
send_command bob "space $SPACE_ID" 2

# Step 17: Bob lists channels
echo -e "${GREEN}Step 15: Bob lists channels${NC}" | tee -a $TEST_RESULTS
send_command bob "channels" 2

# Step 18: Test trying to add Bob again (should fail with DuplicateSignatureKey)
echo -e "${GREEN}Step 16: Testing duplicate add (should fail)${NC}" | tee -a $TEST_RESULTS
send_command alice "member add $BOB_USER_ID" 3

if grep -q "already in the MLS encryption group" $ALICE_LOG; then
    echo -e "${GREEN}✓ Duplicate add correctly rejected!${NC}" | tee -a $TEST_RESULTS
else
    echo -e "${YELLOW}⚠ Expected duplicate add error message${NC}" | tee -a $TEST_RESULTS
fi

# Final summary
echo "" | tee -a $TEST_RESULTS
echo -e "${BLUE}=== Test Summary ===${NC}" | tee -a $TEST_RESULTS
echo "" | tee -a $TEST_RESULTS

# Check overall success
TOTAL_TESTS=0
PASSED_TESTS=0

check_test() {
    local description=$1
    local pattern=$2
    local log_file=$3
    
    TOTAL_TESTS=$((TOTAL_TESTS + 1))
    
    if grep -q "$pattern" "$log_file"; then
        echo -e "${GREEN}✓${NC} $description" | tee -a $TEST_RESULTS
        PASSED_TESTS=$((PASSED_TESTS + 1))
        return 0
    else
        echo -e "${RED}✗${NC} $description" | tee -a $TEST_RESULTS
        return 1
    fi
}

echo "Checking results..." | tee -a $TEST_RESULTS
echo "" | tee -a $TEST_RESULTS

check_test "Alice published KeyPackages" "Published 10 KeyPackages to DHT" $ALICE_LOG
check_test "Bob published KeyPackages" "Published 10 KeyPackages to DHT" $BOB_LOG
check_test "Alice created space" "Created space: $SPACE_NAME" $ALICE_LOG
check_test "Alice created invite" "Created invite code" $ALICE_LOG
check_test "Bob connected to Alice" "Connected to peer" $BOB_LOG
check_test "Bob joined space" "Successfully joined Space" $BOB_LOG
check_test "Alice added Bob to MLS" "added to MLS group" $ALICE_LOG
check_test "Bob received Welcome" "Received MLS Welcome message" $BOB_LOG
check_test "Bob joined MLS group" "Joined MLS group" $BOB_LOG
check_test "Alice created channel" "Created channel: general" $ALICE_LOG
check_test "Alice created thread" "Created thread: Test Thread" $ALICE_LOG
check_test "Alice sent message" "Message sent" $ALICE_LOG
check_test "Duplicate add rejected" "already in the MLS encryption group" $ALICE_LOG

echo "" | tee -a $TEST_RESULTS
echo -e "${BLUE}Results: $PASSED_TESTS/$TOTAL_TESTS tests passed${NC}" | tee -a $TEST_RESULTS

if [ $PASSED_TESTS -eq $TOTAL_TESTS ]; then
    echo -e "${GREEN}🎉 All tests passed!${NC}" | tee -a $TEST_RESULTS
else
    echo -e "${YELLOW}⚠ Some tests failed${NC}" | tee -a $TEST_RESULTS
fi

echo "" | tee -a $TEST_RESULTS
echo -e "${BLUE}=== Output Files ===${NC}" | tee -a $TEST_RESULTS
echo "  Alice log: $ALICE_LOG" | tee -a $TEST_RESULTS
echo "  Bob log: $BOB_LOG" | tee -a $TEST_RESULTS
echo "  Test results: $TEST_RESULTS" | tee -a $TEST_RESULTS
echo "" | tee -a $TEST_RESULTS

# Keep processes running for manual inspection if needed
echo -e "${YELLOW}Processes are still running. Press Ctrl+C to terminate.${NC}"
echo "You can inspect the logs:"
echo "  tail -f $ALICE_LOG"
echo "  tail -f $BOB_LOG"
echo ""

# Wait for user interrupt
wait $ALICE_PID $BOB_PID 2>/dev/null || true
