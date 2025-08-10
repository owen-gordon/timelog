#!/bin/bash

# Master test runner for all end-to-end tests
# Builds the project and runs all E2E test suites

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

echo -e "${BLUE}========================================${NC}"
echo -e "${BLUE}       Timelog E2E Test Suite          ${NC}"
echo -e "${BLUE}========================================${NC}"

echo -e "\n${YELLOW}Project root: $PROJECT_ROOT${NC}"
echo -e "${YELLOW}Test directory: $SCRIPT_DIR${NC}"

# Change to project root
cd "$PROJECT_ROOT"

echo -e "\n${YELLOW}Building timelog binary...${NC}"
if cargo build; then
    echo -e "${GREEN}✓ Build successful${NC}"
else
    echo -e "${RED}✗ Build failed${NC}"
    exit 1
fi

# Function to run a test script
run_test() {
    local test_script="$1"
    local test_name="$(basename "$test_script" .sh)"
    
    echo -e "\n${BLUE}========================================${NC}"
    echo -e "${BLUE}Running $test_name${NC}"
    echo -e "${BLUE}========================================${NC}"
    
    if bash "$test_script"; then
        echo -e "${GREEN}✓ $test_name PASSED${NC}"
        return 0
    else
        echo -e "${RED}✗ $test_name FAILED${NC}"
        return 1
    fi
}

# Track test results
PASSED=0
FAILED=0
FAILED_TESTS=()

# List of test scripts to run
TEST_SCRIPTS=(
    "$SCRIPT_DIR/test_basic_workflow.sh"
    "$SCRIPT_DIR/test_project_workflow.sh"
    "$SCRIPT_DIR/test_period_reports.sh"
    "$SCRIPT_DIR/test_plugin_system.sh"
)

echo -e "\n${YELLOW}Found ${#TEST_SCRIPTS[@]} test scripts to run${NC}"

# Run each test
for test_script in "${TEST_SCRIPTS[@]}"; do
    if [ -f "$test_script" ]; then
        if run_test "$test_script"; then
            PASSED=$((PASSED + 1))
        else
            FAILED=$((FAILED + 1))
            FAILED_TESTS+=("$(basename "$test_script" .sh)")
        fi
    else
        echo -e "${RED}Warning: Test script not found: $test_script${NC}"
        FAILED=$((FAILED + 1))
        FAILED_TESTS+=("$(basename "$test_script" .sh) (not found)")
    fi
done

# Print summary
echo -e "\n${BLUE}========================================${NC}"
echo -e "${BLUE}           Test Summary                 ${NC}"
echo -e "${BLUE}========================================${NC}"

echo -e "${GREEN}Passed: $PASSED${NC}"
echo -e "${RED}Failed: $FAILED${NC}"

if [ $FAILED -gt 0 ]; then
    echo -e "\n${RED}Failed tests:${NC}"
    for failed_test in "${FAILED_TESTS[@]}"; do
        echo -e "${RED}  - $failed_test${NC}"
    done
    echo -e "\n${RED}Some tests failed! ✗${NC}"
    exit 1
else
    echo -e "\n${GREEN}All tests passed! ✓${NC}"
    echo -e "${GREEN}E2E test suite completed successfully${NC}"
fi
