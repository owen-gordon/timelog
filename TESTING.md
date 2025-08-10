# Testing Guide for Timelog

This document describes the comprehensive testing setup for the timelog project.

## Test Structure

The timelog project has a multi-layered testing approach:

### 1. Unit Tests (`src/lib.rs`)
- **Location**: Embedded in `src/lib.rs` 
- **Purpose**: Test individual functions and utilities
- **Coverage**: 
  - Date period calculations (`period_range`)
  - Duration formatting (`fmt_duration`, `fmt_hms_ms`)
  - Utility functions (`clamp_nonneg`, `weekday_short`)
- **Run with**: `cargo test --lib`

### 2. Integration Tests (`tests/`)
- **Location**: `tests/` directory
- **Purpose**: Test module interactions and file I/O operations

#### `integration_tests.rs`
- State management (save/load/delete)
- Record operations (CSV handling)
- Backwards compatibility with old file formats
- Plugin discovery and path resolution
- Error handling

#### `cli_tests.rs`
- End-to-end CLI command testing using `assert_cmd`
- Complete workflow testing (start → pause → resume → stop → report)
- Error condition testing
- Help and version output validation

#### `plugin_tests.rs`
- Plugin discovery mechanics
- Plugin execution (success/failure/dry-run)
- Plugin input/output format validation
- Configuration file handling

- **Run with**: `cargo test --test <test_name>`

### 3. End-to-End Tests (`tests/e2e/`)
- **Location**: `tests/e2e/` directory  
- **Purpose**: Real-world usage simulation with bash scripts
- **Environment**: Uses temporary directories and actual binary

#### Test Scripts

##### `test_basic_workflow.sh`
Complete user journey:
1. Start a task
2. Check status
3. Pause/resume workflow
4. Stop task
5. Generate report
6. Verify file cleanup

##### `test_project_workflow.sh`
Project-based workflows:
1. Tasks with different projects
2. Project filtering in reports
3. CSV format validation

##### `test_plugin_system.sh`
Plugin system validation:
1. Plugin discovery
2. Plugin execution (success/failure)
3. Configuration handling
4. Dry-run mode
5. Error handling

##### `test_period_reports.sh`
Period-based reporting:
1. All period types (today, yesterday, this-week, etc.)
2. Date boundary testing
3. Report format validation
4. Edge case handling

- **Run with**: `tests/e2e/run_all_tests.sh`

## Running Tests

### Quick Commands

```bash
# Run all tests
make test

# Run specific test types
make test-unit
make test-integration  
make test-e2e

# Run comprehensive test suite
make test-all
./run_tests.sh

# Quick tests (skip E2E)
make test-quick
./run_tests.sh --skip-e2e
```

### Individual Test Commands

```bash
# Unit tests
cargo test --lib

# Specific integration test
cargo test --test integration_tests
cargo test --test cli_tests
cargo test --test plugin_tests

# All integration tests
cargo test --test integration_tests --test cli_tests --test plugin_tests

# E2E tests
tests/e2e/run_all_tests.sh

# Specific E2E test
tests/e2e/test_basic_workflow.sh
```

## Test Environment

### Environment Variables
Tests use temporary directories via these environment variables:
- `TIMELOG_RECORD_PATH`: Path to CSV records file
- `TIMELOG_STATE_PATH`: Path to state JSON file  
- `TIMELOG_PLUGIN_PATH`: Path to plugins directory

### Serial Test Execution
Many tests use `#[serial]` attribute to prevent race conditions when accessing environment variables or shared resources.

### Temporary Directories
- Integration and E2E tests create isolated temporary directories
- Automatic cleanup after test completion
- No interference between test runs

## Test Features

### Error Condition Testing
- Invalid commands and arguments
- Missing files and permissions
- Plugin failures and invalid output
- State conflicts (e.g., starting when already running)

### Backwards Compatibility
- Tests old CSV format without project column
- Ensures seamless migration to new formats

### Real Binary Testing
- E2E tests use actual compiled binary
- Validates complete CLI interface
- Tests in realistic environment

### Plugin System Testing
- Mock plugin creation with proper executable permissions
- JSON input/output validation
- Configuration file handling
- Dry-run mode testing

## Continuous Integration

### GitHub Actions (`.github/workflows/test.yml`)
- Runs on Ubuntu with multiple Rust versions
- Includes formatting and linting checks
- Security audit with `cargo audit`
- Release build validation

### Pre-commit Hooks
```bash
# Setup development environment with pre-commit hooks
make setup-dev
```

## Test Coverage

### Coverage Report
```bash
# Generate HTML coverage report
make coverage
```

### Current Coverage Areas
- ✅ Core functionality (start/stop/pause/resume)
- ✅ Reporting with all period types
- ✅ Project filtering and management
- ✅ Plugin discovery and execution
- ✅ Error handling and edge cases
- ✅ File I/O operations
- ✅ CLI argument parsing
- ✅ Backwards compatibility

## Writing New Tests

### Unit Tests
Add to `src/lib.rs` in the `#[cfg(test)]` module:
```rust
#[test]
fn test_new_functionality() {
    // Test implementation
}
```

### Integration Tests
Create new file in `tests/` directory:
```rust
use timelog::*;
use serial_test::serial;

#[test]
#[serial]
fn test_integration_scenario() {
    // Test implementation with proper setup/cleanup
}
```

### E2E Tests
Create new bash script in `tests/e2e/`:
```bash
#!/bin/bash
set -e

# Test implementation with proper environment setup
```

## Debugging Tests

### Verbose Output
```bash
# Verbose test execution
./run_tests.sh
cargo test -- --nocapture

# Show test output for specific test
cargo test test_name -- --nocapture
```

### Test Isolation
- Each test uses isolated temporary directories
- Environment variables are properly cleaned up
- Use `#[serial]` for tests that can't run in parallel

### Common Issues
1. **Environment variable conflicts**: Use `#[serial]` attribute
2. **Temporary directory cleanup**: Implement proper `Drop` or cleanup functions
3. **Binary not found**: Ensure `cargo build` runs before E2E tests
4. **Permission issues**: Check executable permissions on test scripts and mock plugins

## Performance Considerations

### Test Execution Time
- Unit tests: < 1 second
- Integration tests: 1-3 seconds
- E2E tests: 5-10 seconds
- Full suite: < 15 seconds

### Optimization Strategies
- Parallel test execution where possible
- Minimal sleep times in timing-dependent tests
- Efficient temporary directory management
- Cached builds in CI environment

## Future Test Enhancements

### Potential Additions
- Property-based testing with `quickcheck`
- Performance benchmarks
- Memory usage validation
- Cross-platform testing (Windows, macOS)
- Plugin ecosystem testing
- Database integration tests (if added)
- WebAssembly compatibility tests (if added)

### Test Metrics
- Line coverage tracking
- Branch coverage analysis
- Performance regression detection
- Flaky test identification
