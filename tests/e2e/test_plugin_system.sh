#!/bin/bash

# End-to-end test for plugin system
# Tests plugin discovery, execution, and configuration

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

TEST_NAME="Plugin System E2E Test"
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

echo -e "\n${YELLOW}Setting up plugin directory${NC}"
mkdir -p "$TIMELOG_PLUGIN_PATH"

echo -e "\n${YELLOW}Test 1: List plugins (should be empty)${NC}"
check_output "upload --list-plugins" "No plugins found"

echo -e "\n${YELLOW}Test 2: Create a test plugin${NC}"
cat > "$TIMELOG_PLUGIN_PATH/timelog-test" << 'EOF'
#!/bin/bash

# Simple test plugin for timelog
# Reads JSON from stdin and outputs JSON to stdout

read -r input

# Parse basic info from input
echo "Plugin received input: $input" >&2

# Extract period from input (basic parsing)
period=$(echo "$input" | grep -o '"period":"[^"]*"' | cut -d'"' -f4)
record_count=$(echo "$input" | grep -o '"records":\[' | wc -l)

# Output plugin result
if [ "$1" = "--dry-run" ]; then
    cat << JSON
{
    "success": true,
    "message": "Dry run: Would upload $record_count records for period $period",
    "uploaded_count": 0,
    "errors": []
}
JSON
else
    cat << JSON
{
    "success": true,
    "message": "Successfully uploaded $record_count records for period $period",
    "uploaded_count": $record_count,
    "errors": []
}
JSON
fi
EOF

chmod +x "$TIMELOG_PLUGIN_PATH/timelog-test"

echo -e "\n${YELLOW}Test 3: Create plugin configuration${NC}"
cat > "$TIMELOG_PLUGIN_PATH/timelog-test.json" << 'EOF'
{
    "api_endpoint": "https://example.com/api",
    "timeout": 30,
    "retry_count": 3
}
EOF

echo -e "\n${YELLOW}Test 4: List plugins (should show test plugin)${NC}"
check_output "upload --list-plugins" "test"

echo -e "\n${YELLOW}Test 5: Create some test data${NC}"
run_timelog "start \"test task 1\""
sleep 0.1
run_timelog "stop"

run_timelog "start \"test task 2\""
sleep 0.1
run_timelog "stop"

echo -e "\n${YELLOW}Test 6: Upload with dry-run${NC}"
check_output "upload --plugin test today --dry-run" "Dry run"
check_output "upload --plugin test today --dry-run" "Would upload"

echo -e "\n${YELLOW}Test 7: Upload without dry-run${NC}"
check_output "upload --plugin test today" "Successfully uploaded"

echo -e "\n${YELLOW}Test 8: Test plugin with no data${NC}"
# Try uploading yesterday's data (should be empty)
output=$(eval "$TIMELOG_BIN upload --plugin test yesterday" 2>&1)
if echo "$output" | grep -q "no records in selected period"; then
    echo -e "${GREEN}✓ Correctly handled empty data set${NC}"
else
    echo -e "${RED}✗ Unexpected output for empty data set${NC}"
    echo "Output: $output"
    exit 1
fi

echo -e "\n${YELLOW}Test 9: Create a failing plugin${NC}"
cat > "$TIMELOG_PLUGIN_PATH/timelog-fail" << 'EOF'
#!/bin/bash
echo '{"success": false, "message": "Test failure", "errors": ["Simulated error"]}' 
exit 1
EOF
chmod +x "$TIMELOG_PLUGIN_PATH/timelog-fail"

echo -e "\n${YELLOW}Test 10: Test failing plugin${NC}"
output=$(eval "$TIMELOG_BIN upload --plugin fail today" 2>&1 || true)
if echo "$output" | grep -q "Plugin failed"; then
    echo -e "${GREEN}✓ Correctly handled plugin failure${NC}"
else
    echo -e "${RED}✗ Did not properly handle plugin failure${NC}"
    echo "Output: $output"
    exit 1
fi

echo -e "\n${YELLOW}Test 11: Create a non-executable file (should be ignored)${NC}"
cat > "$TIMELOG_PLUGIN_PATH/timelog-notexec" << 'EOF'
#!/bin/bash
echo "This should not be executed"
EOF
# Don't make it executable

echo -e "\n${YELLOW}Test 12: List plugins (should not include non-executable)${NC}"
output=$(eval "$TIMELOG_BIN upload --list-plugins" 2>&1)
if echo "$output" | grep -q "notexec"; then
    echo -e "${RED}✗ Non-executable file was listed as plugin${NC}"
    exit 1
else
    echo -e "${GREEN}✓ Non-executable file correctly ignored${NC}"
fi

echo -e "\n${YELLOW}Test 13: Create a non-plugin file (should be ignored)${NC}"
cat > "$TIMELOG_PLUGIN_PATH/regular-file" << 'EOF'
Just a regular file
EOF
chmod +x "$TIMELOG_PLUGIN_PATH/regular-file"

echo -e "\n${YELLOW}Test 14: List plugins (should not include non-plugin files)${NC}"
output=$(eval "$TIMELOG_BIN upload --list-plugins" 2>&1)
if echo "$output" | grep -q "regular"; then
    echo -e "${RED}✗ Non-plugin file was listed as plugin${NC}"
    exit 1
else
    echo -e "${GREEN}✓ Non-plugin file correctly ignored${NC}"
fi

echo -e "\n${YELLOW}Test 15: Test auto-selection with single plugin${NC}"
# Remove the failing plugin to test auto-selection
rm "$TIMELOG_PLUGIN_PATH/timelog-fail"
check_output "upload today" "Successfully uploaded"

echo -e "\n${GREEN}All plugin system tests passed! ✓${NC}"
echo -e "${GREEN}$TEST_NAME completed successfully${NC}"
