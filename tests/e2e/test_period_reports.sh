#!/bin/bash

# End-to-end test for different period reporting
# Tests all period types: today, yesterday, this-week, last-week, etc.

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

TEST_NAME="Period Reports E2E Test"
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

check_period_works() {
    local period="$1"
    local expected_title="$2"
    
    echo "Testing period: $period"
    
    # First check with no data - should warn about no records
    output=$(eval "$TIMELOG_BIN report \"$period\"" 2>&1 || true)
    if echo "$output" | grep -q "no records"; then
        echo -e "${GREEN}✓ Period '$period' correctly handles no data${NC}"
    else
        # If there are records from previous tests, just check the title
        if echo "$output" | grep -q "$expected_title"; then
            echo -e "${GREEN}✓ Period '$period' shows correct title${NC}"
        else
            echo -e "${RED}✗ Period '$period' output unexpected${NC}"
            echo "Output: $output"
            exit 1
        fi
    fi
}

cleanup() {
    rm -rf "$TEMP_DIR"
}
trap cleanup EXIT

echo -e "\n${YELLOW}Test 1: Test all period types with no data${NC}"

# Test all supported period types
check_period_works "today" "Today"
check_period_works "yesterday" "Yesterday"
check_period_works "this-week" "This Week"
check_period_works "last-week" "Last Week"
check_period_works "this-month" "This Month"
check_period_works "last-month" "Last Month"
check_period_works "ytd" "Year To Date"
check_period_works "last-year" "Last Year"

echo -e "\n${YELLOW}Test 2: Create some test data for today${NC}"
run_timelog "start \"morning task\""
sleep 0.1
run_timelog "stop"

run_timelog "start \"afternoon task\""
sleep 0.1
run_timelog "stop"

echo -e "\n${YELLOW}Test 3: Test today report with data${NC}"
check_output "report today" "Today report"
check_output "report today" "morning task"
check_output "report today" "afternoon task"
check_output "report today" "TOTAL"

echo -e "\n${YELLOW}Test 4: Verify report structure${NC}"
output=$(eval "$TIMELOG_BIN report today" 2>&1)

# Check for proper table headers
if echo "$output" | grep -q "TASK"; then
    echo -e "${GREEN}✓ Report has TASK header${NC}"
else
    echo -e "${RED}✗ Report missing TASK header${NC}"
    exit 1
fi

if echo "$output" | grep -q "DURATION"; then
    echo -e "${GREEN}✓ Report has DURATION header${NC}"
else
    echo -e "${RED}✗ Report missing DURATION header${NC}"
    exit 1
fi

if echo "$output" | grep -q "PROJECT"; then
    echo -e "${GREEN}✓ Report has PROJECT header${NC}"
else
    echo -e "${RED}✗ Report missing PROJECT header${NC}"
    exit 1
fi

if echo "$output" | grep -q "DATE"; then
    echo -e "${GREEN}✓ Report has DATE header${NC}"
else
    echo -e "${RED}✗ Report missing DATE header${NC}"
    exit 1
fi

# Check for separator lines
if echo "$output" | grep -q -- "---"; then
    echo -e "${GREEN}✓ Report has separator lines${NC}"
else
    echo -e "${RED}✗ Report missing separator lines${NC}"
    exit 1
fi

echo -e "\n${YELLOW}Test 5: Test period boundaries${NC}"

# Test yesterday (should be empty since we only have today's data)
output=$(eval "$TIMELOG_BIN report yesterday" 2>&1 || true)
if echo "$output" | grep -q "no records in selected period"; then
    echo -e "${GREEN}✓ Yesterday report correctly shows no records${NC}"
else
    echo -e "${RED}✗ Yesterday report should be empty${NC}"
    echo "Output: $output"
    exit 1
fi

echo -e "\n${YELLOW}Test 6: Test this-week (should include today's data)${NC}"
check_output "report this-week" "This Week"
check_output "report this-week" "morning task"

echo -e "\n${YELLOW}Test 7: Test this-month (should include today's data)${NC}"
check_output "report this-month" "This Month"
check_output "report this-month" "morning task"

echo -e "\n${YELLOW}Test 8: Test ytd (should include today's data)${NC}"
check_output "report ytd" "Year To Date"
check_output "report ytd" "morning task"

echo -e "\n${YELLOW}Test 9: Test invalid period handling${NC}"
if eval "$TIMELOG_BIN report invalid-period" >/dev/null 2>&1; then
    echo -e "${RED}✗ Invalid period should have been rejected${NC}"
    exit 1
else
    echo -e "${GREEN}✓ Invalid period correctly rejected${NC}"
fi

echo -e "\n${YELLOW}Test 10: Test case sensitivity${NC}"
if eval "$TIMELOG_BIN report TODAY" >/dev/null 2>&1; then
    echo -e "${RED}✗ Should be case sensitive${NC}"
    exit 1
else
    echo -e "${GREEN}✓ Uppercase period correctly rejected${NC}"
fi

echo -e "\n${YELLOW}Test 11: Verify CSV file format${NC}"
if [ -f "$TIMELOG_RECORD_PATH" ]; then
    echo "Record file contents:"
    cat "$TIMELOG_RECORD_PATH"
    
    # Check that records are properly formatted
    if grep -q "morning task" "$TIMELOG_RECORD_PATH" && grep -q "afternoon task" "$TIMELOG_RECORD_PATH"; then
        echo -e "${GREEN}✓ Record file contains expected tasks${NC}"
    else
        echo -e "${RED}✗ Record file missing expected tasks${NC}"
        exit 1
    fi
else
    echo -e "${RED}✗ Record file not found${NC}"
    exit 1
fi

echo -e "\n${GREEN}All period report tests passed! ✓${NC}"
echo -e "${GREEN}$TEST_NAME completed successfully${NC}"
