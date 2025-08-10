#!/bin/bash

# Simple, reliable test runner for timelog

set -e

# Colors
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m'

echo "=== TIMELOG TEST SUITE ==="

# Parse arguments
SKIP_E2E=false
if [ "$1" = "--skip-e2e" ]; then
    SKIP_E2E=true
fi

# Build
echo -e "\n${YELLOW}Building project...${NC}"
cargo build
echo -e "${GREEN}✓ Build successful${NC}"

# Unit tests
echo -e "\n${YELLOW}Running unit tests...${NC}"
cargo test --lib
echo -e "${GREEN}✓ Unit tests passed${NC}"

# Integration tests
echo -e "\n${YELLOW}Running integration tests...${NC}"
cargo test --test integration_tests
cargo test --test cli_tests  
cargo test --test plugin_tests
echo -e "${GREEN}✓ Integration tests passed${NC}"

# E2E tests (if not skipped)
if [ "$SKIP_E2E" = "false" ]; then
    echo -e "\n${YELLOW}Running E2E tests...${NC}"
    tests/e2e/test_basic_workflow.sh
    tests/e2e/test_project_workflow.sh
    tests/e2e/test_period_reports.sh
    tests/e2e/test_plugin_system.sh
    echo -e "${GREEN}✓ E2E tests passed${NC}"
fi

echo -e "\n${GREEN}All tests completed successfully! ✓${NC}"
