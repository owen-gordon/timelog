#!/bin/bash

# End-to-end test for basic timelog workflow
# This script tests the complete user journey: start -> status -> pause -> resume -> stop -> report

set -e  # Exit on any error

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Test configuration
TEST_NAME="Basic Workflow E2E Test"
TEMP_DIR=$(mktemp -d)
TIMELOG_BIN="${CARGO_TARGET_DIR:-target}/debug/timelog"

# Setup test environment
export TIMELOG_RECORD_PATH="$TEMP_DIR/records.csv"
export TIMELOG_STATE_PATH="$TEMP_DIR/state.json"
export TIMELOG_PLUGIN_PATH="$TEMP_DIR/plugins"

echo -e "${YELLOW}Starting $TEST_NAME${NC}"
echo "Test directory: $TEMP_DIR"
echo "Using binary: $TIMELOG_BIN"

# Ensure binary exists
if [ ! -f "$TIMELOG_BIN" ]; then
    echo -e "${RED}Error: timelog binary not found at $TIMELOG_BIN${NC}"
    echo "Please run 'cargo build' first"
    exit 1
fi

# Function to run timelog command and check exit code
run_timelog() {
    local expected_exit_code=${2:-0}
    echo "Running: timelog $1"
    
    if [ $expected_exit_code -eq 0 ]; then
        eval "$TIMELOG_BIN $1"
    else
        # Expect failure
        if eval "$TIMELOG_BIN $1" 2>/dev/null; then
            echo -e "${RED}Expected command to fail but it succeeded: timelog $1${NC}"
            exit 1
        else
            echo "Command failed as expected"
        fi
    fi
}

# Function to check if output contains expected string
check_output() {
    local command="$1"
    local expected="$2"
    local output
    
    echo "Running: timelog $command"
    output=$(eval "$TIMELOG_BIN $command" 2>&1)
    
    if echo "$output" | grep -q "$expected"; then
        echo -e "${GREEN}✓ Output contains '$expected'${NC}"
    else
        echo -e "${RED}✗ Output does not contain '$expected'${NC}"
        echo "Actual output: $output"
        exit 1
    fi
}

# Function to cleanup
cleanup() {
    echo "Cleaning up test directory: $TEMP_DIR"
    rm -rf "$TEMP_DIR"
}
trap cleanup EXIT

echo -e "\n${YELLOW}Test 1: Start a task${NC}"
run_timelog "start \"writing tests\""
echo -e "${GREEN}✓ Task started successfully${NC}"

echo -e "\n${YELLOW}Test 2: Check status (should be active)${NC}"
check_output "status" "active"
check_output "status" "writing tests"

echo -e "\n${YELLOW}Test 3: Try to start another task (should fail)${NC}"
run_timelog "start \"another task\"" 1

echo -e "\n${YELLOW}Test 4: Pause the task${NC}"
run_timelog "pause"
echo -e "${GREEN}✓ Task paused successfully${NC}"

echo -e "\n${YELLOW}Test 5: Check status (should be paused)${NC}"
check_output "status" "paused"
check_output "status" "writing tests"

echo -e "\n${YELLOW}Test 6: Try to pause again (should fail)${NC}"
run_timelog "pause" 1

echo -e "\n${YELLOW}Test 7: Resume the task${NC}"
run_timelog "resume"
echo -e "${GREEN}✓ Task resumed successfully${NC}"

echo -e "\n${YELLOW}Test 8: Check status (should be active again)${NC}"
check_output "status" "active"

echo -e "\n${YELLOW}Test 9: Try to resume again (should fail)${NC}"
run_timelog "resume" 1

echo -e "\n${YELLOW}Test 10: Stop the task${NC}"
run_timelog "stop"
echo -e "${GREEN}✓ Task stopped successfully${NC}"

echo -e "\n${YELLOW}Test 11: Check status (should fail - no active task)${NC}"
run_timelog "status" 1

echo -e "\n${YELLOW}Test 12: Generate report${NC}"
check_output "report today" "writing tests"
check_output "report today" "Today report"

echo -e "\n${YELLOW}Test 13: Verify record file exists and has content${NC}"
if [ -f "$TIMELOG_RECORD_PATH" ]; then
    if [ -s "$TIMELOG_RECORD_PATH" ]; then
        echo -e "${GREEN}✓ Record file exists and has content${NC}"
        echo "Record file contents:"
        cat "$TIMELOG_RECORD_PATH"
    else
        echo -e "${RED}✗ Record file exists but is empty${NC}"
        exit 1
    fi
else
    echo -e "${RED}✗ Record file does not exist${NC}"
    exit 1
fi

echo -e "\n${YELLOW}Test 14: Verify state file is cleaned up${NC}"
if [ ! -f "$TIMELOG_STATE_PATH" ]; then
    echo -e "${GREEN}✓ State file properly cleaned up after stop${NC}"
else
    echo -e "${RED}✗ State file still exists after stop${NC}"
    exit 1
fi

echo -e "\n${GREEN}All tests passed! ✓${NC}"
echo -e "${GREEN}$TEST_NAME completed successfully${NC}"
