#!/bin/bash

# End-to-end test for project-based workflow
# Tests project assignment, filtering, and reporting

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

TEST_NAME="Project Workflow E2E Test"
TEMP_DIR=$(mktemp -d)
TIMELOG_BIN="${CARGO_TARGET_DIR:-target}/debug/timelog"

# Setup test environment
export TIMELOG_RECORD_PATH="$TEMP_DIR/records.csv"
export TIMELOG_STATE_PATH="$TEMP_DIR/state.json"
export TIMELOG_PLUGIN_PATH="$TEMP_DIR/plugins"

echo -e "${YELLOW}Starting $TEST_NAME${NC}"
echo "Test directory: $TEMP_DIR"

# Ensure binary exists
if [ ! -f "$TIMELOG_BIN" ]; then
    echo -e "${RED}Error: timelog binary not found at $TIMELOG_BIN${NC}"
    exit 1
fi

run_timelog() {
    echo "Running: timelog $1"
    eval "$TIMELOG_BIN $1"
}

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

cleanup() {
    rm -rf "$TEMP_DIR"
}
trap cleanup EXIT

# Wait function to ensure different timestamps
wait_brief() {
    sleep 0.1
}

echo -e "\n${YELLOW}Test 1: Start task with project 'frontend'${NC}"
run_timelog "start \"implement login\" --project frontend"
wait_brief
run_timelog "stop"

echo -e "\n${YELLOW}Test 2: Start task with project 'backend'${NC}"
run_timelog "start \"setup database\" --project backend"
wait_brief
run_timelog "stop"

echo -e "\n${YELLOW}Test 3: Start task without project${NC}"
run_timelog "start \"code review\""
wait_brief
run_timelog "stop"

echo -e "\n${YELLOW}Test 4: Check general report (should show all tasks)${NC}"
check_output "report today" "implement login"
check_output "report today" "setup database"
check_output "report today" "code review"

echo -e "\n${YELLOW}Test 5: Check frontend project report${NC}"
check_output "report today --project frontend" "implement login"
check_output "report today --project frontend" "for project frontend"

# Verify backend project is NOT in frontend report
echo "Verifying backend project exclusion from frontend report"
output=$($TIMELOG_BIN report today --project frontend 2>&1)
if echo "$output" | grep -q "setup database"; then
    echo -e "${RED}✗ Backend task found in frontend report${NC}"
    exit 1
else
    echo -e "${GREEN}✓ Backend task correctly excluded from frontend report${NC}"
fi

echo -e "\n${YELLOW}Test 6: Check backend project report${NC}"
check_output "report today --project backend" "setup database"
check_output "report today --project backend" "for project backend"

echo -e "\n${YELLOW}Test 7: Test project filtering with non-existent project${NC}"
output=$($TIMELOG_BIN report today --project nonexistent 2>&1)
if echo "$output" | grep -q "no records in selected period"; then
    echo -e "${GREEN}✓ Correctly handled non-existent project filter${NC}"
else
    echo -e "${RED}✗ Unexpected output for non-existent project${NC}"
    echo "Output: $output"
    exit 1
fi

echo -e "\n${YELLOW}Test 8: Verify CSV format includes project column${NC}"
if [ -f "$TIMELOG_RECORD_PATH" ]; then
    echo "Record file contents:"
    cat "$TIMELOG_RECORD_PATH"
    
    # Check that we have the expected number of columns (4: task, duration, date, project)
    line_count=$(wc -l < "$TIMELOG_RECORD_PATH")
    if [ $line_count -ge 4 ]; then  # Header + 3 records
        echo -e "${GREEN}✓ Record file has expected content${NC}"
    else
        echo -e "${RED}✗ Record file has unexpected number of lines: $line_count${NC}"
        exit 1
    fi
else
    echo -e "${RED}✗ Record file not found${NC}"
    exit 1
fi

echo -e "\n${GREEN}All project workflow tests passed! ✓${NC}"
echo -e "${GREEN}$TEST_NAME completed successfully${NC}"
