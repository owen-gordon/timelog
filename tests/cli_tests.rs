use assert_cmd::Command;
use predicates::prelude::*;
use serial_test::serial;
use std::env;
use std::fs;
use std::thread;
use std::time::Duration;
use tempfile::TempDir;

/// Helper to set up a clean test environment with temporary directories
fn setup_cli_test_env() -> TempDir {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let temp_path = temp_dir.path().to_str().unwrap();

    // Set environment variables to use temporary directories
    unsafe {
        env::set_var("TIMELOG_RECORD_PATH", format!("{}/records.csv", temp_path));
        env::set_var("TIMELOG_STATE_PATH", format!("{}/state.json", temp_path));
        env::set_var("TIMELOG_PLUGIN_PATH", format!("{}/plugins", temp_path));
    }

    // Create plugins directory
    fs::create_dir_all(format!("{}/plugins", temp_path)).expect("Failed to create plugins dir");

    temp_dir
}

/// Clean up environment variables after test
fn cleanup_cli_test_env() {
    unsafe {
        env::remove_var("TIMELOG_RECORD_PATH");
        env::remove_var("TIMELOG_STATE_PATH");
        env::remove_var("TIMELOG_PLUGIN_PATH");
    }
}

#[test]
#[serial]
fn test_start_task() {
    let _temp_dir = setup_cli_test_env();

    let mut cmd = Command::cargo_bin("timelog").unwrap();
    cmd.args(&["start", "test task"])
        .assert()
        .success()
        .stdout(predicate::str::contains("started test task"));

    cleanup_cli_test_env();
}

#[test]
#[serial]
fn test_start_task_with_project() {
    let _temp_dir = setup_cli_test_env();

    let mut cmd = Command::cargo_bin("timelog").unwrap();
    cmd.args(&["start", "test task", "--project", "test project"])
        .assert()
        .success()
        .stdout(predicate::str::contains("started test task"))
        .stdout(predicate::str::contains("in project test project"));

    cleanup_cli_test_env();
}

#[test]
#[serial]
fn test_cannot_start_when_already_running() {
    let _temp_dir = setup_cli_test_env();

    // Start first task
    let mut cmd = Command::cargo_bin("timelog").unwrap();
    cmd.args(&["start", "first task"]).assert().success();

    // Try to start second task - should fail
    let mut cmd = Command::cargo_bin("timelog").unwrap();
    cmd.args(&["start", "second task"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("task is already in progress"));

    cleanup_cli_test_env();
}

#[test]
#[serial]
fn test_status_when_no_task() {
    let _temp_dir = setup_cli_test_env();

    let mut cmd = Command::cargo_bin("timelog").unwrap();
    cmd.arg("status")
        .assert()
        .failure()
        .stderr(predicate::str::contains("no task to provide status"));

    cleanup_cli_test_env();
}

#[test]
#[serial]
fn test_pause_resume_workflow() {
    let _temp_dir = setup_cli_test_env();

    // Start a task
    let mut cmd = Command::cargo_bin("timelog").unwrap();
    cmd.args(&["start", "pausable task"]).assert().success();

    // Check status (should be active)
    let mut cmd = Command::cargo_bin("timelog").unwrap();
    cmd.arg("status")
        .assert()
        .success()
        .stdout(predicate::str::contains("active"))
        .stdout(predicate::str::contains("pausable task"));

    // Wait a moment to ensure some time passes
    thread::sleep(Duration::from_millis(100));

    // Pause the task
    let mut cmd = Command::cargo_bin("timelog").unwrap();
    cmd.arg("pause")
        .assert()
        .success()
        .stdout(predicate::str::contains("paused"))
        .stdout(predicate::str::contains("pausable task"));

    // Check status (should be paused)
    let mut cmd = Command::cargo_bin("timelog").unwrap();
    cmd.arg("status")
        .assert()
        .success()
        .stdout(predicate::str::contains("paused"))
        .stdout(predicate::str::contains("pausable task"));

    // Resume the task
    let mut cmd = Command::cargo_bin("timelog").unwrap();
    cmd.arg("resume")
        .assert()
        .success()
        .stdout(predicate::str::contains("resumed"))
        .stdout(predicate::str::contains("pausable task"));

    // Check status (should be active again)
    let mut cmd = Command::cargo_bin("timelog").unwrap();
    cmd.arg("status")
        .assert()
        .success()
        .stdout(predicate::str::contains("active"))
        .stdout(predicate::str::contains("pausable task"));

    cleanup_cli_test_env();
}

#[test]
#[serial]
fn test_stop_workflow() {
    let _temp_dir = setup_cli_test_env();

    // Start a task
    let mut cmd = Command::cargo_bin("timelog").unwrap();
    cmd.args(&["start", "stoppable task"]).assert().success();

    // Wait a moment
    thread::sleep(Duration::from_millis(100));

    // Stop the task
    let mut cmd = Command::cargo_bin("timelog").unwrap();
    cmd.arg("stop")
        .assert()
        .success()
        .stdout(predicate::str::contains("recorded"))
        .stdout(predicate::str::contains("stoppable task"));

    // Status should fail now (no active task)
    let mut cmd = Command::cargo_bin("timelog").unwrap();
    cmd.arg("status")
        .assert()
        .failure()
        .stderr(predicate::str::contains("no task to provide status"));

    cleanup_cli_test_env();
}

#[test]
#[serial]
fn test_report_no_records() {
    let _temp_dir = setup_cli_test_env();

    let mut cmd = Command::cargo_bin("timelog").unwrap();
    cmd.args(&["report", "today"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("no records found"));

    cleanup_cli_test_env();
}

#[test]
#[serial]
fn test_complete_workflow_with_report() {
    let _temp_dir = setup_cli_test_env();

    // Start and immediately stop a task to create a record
    let mut cmd = Command::cargo_bin("timelog").unwrap();
    cmd.args(&["start", "workflow task"]).assert().success();

    // Wait a moment to ensure measurable duration
    thread::sleep(Duration::from_millis(100));

    let mut cmd = Command::cargo_bin("timelog").unwrap();
    cmd.arg("stop").assert().success();

    // Generate report
    let mut cmd = Command::cargo_bin("timelog").unwrap();
    cmd.args(&["report", "today"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Today report"))
        .stdout(predicate::str::contains("workflow task"))
        .stdout(predicate::str::contains("TASK"))
        .stdout(predicate::str::contains("DURATION"));

    cleanup_cli_test_env();
}

#[test]
#[serial]
fn test_project_filtering_in_report() {
    let _temp_dir = setup_cli_test_env();

    // Create records with different projects
    let mut cmd = Command::cargo_bin("timelog").unwrap();
    cmd.args(&["start", "project1 task", "--project", "project1"])
        .assert()
        .success();

    thread::sleep(Duration::from_millis(50));

    let mut cmd = Command::cargo_bin("timelog").unwrap();
    cmd.arg("stop").assert().success();

    let mut cmd = Command::cargo_bin("timelog").unwrap();
    cmd.args(&["start", "project2 task", "--project", "project2"])
        .assert()
        .success();

    thread::sleep(Duration::from_millis(50));

    let mut cmd = Command::cargo_bin("timelog").unwrap();
    cmd.arg("stop").assert().success();

    // Test project filtering
    let mut cmd = Command::cargo_bin("timelog").unwrap();
    cmd.args(&["report", "today", "--project", "project1"])
        .assert()
        .success()
        .stdout(predicate::str::contains("project1 task"))
        .stdout(predicate::str::contains("for project project1"))
        .stdout(predicate::str::contains("project2 task").not());

    cleanup_cli_test_env();
}

#[test]
#[serial]
fn test_pause_without_active_task() {
    let _temp_dir = setup_cli_test_env();

    let mut cmd = Command::cargo_bin("timelog").unwrap();
    cmd.arg("pause")
        .assert()
        .failure()
        .stderr(predicate::str::contains("no active task to pause"));

    cleanup_cli_test_env();
}

#[test]
#[serial]
fn test_resume_without_paused_task() {
    let _temp_dir = setup_cli_test_env();

    let mut cmd = Command::cargo_bin("timelog").unwrap();
    cmd.arg("resume")
        .assert()
        .failure()
        .stderr(predicate::str::contains("no paused task to resume"));

    cleanup_cli_test_env();
}

#[test]
#[serial]
fn test_pause_already_paused_task() {
    let _temp_dir = setup_cli_test_env();

    // Start and pause a task
    let mut cmd = Command::cargo_bin("timelog").unwrap();
    cmd.args(&["start", "pausable task"]).assert().success();

    let mut cmd = Command::cargo_bin("timelog").unwrap();
    cmd.arg("pause").assert().success();

    // Try to pause again
    let mut cmd = Command::cargo_bin("timelog").unwrap();
    cmd.arg("pause")
        .assert()
        .failure()
        .stderr(predicate::str::contains("task is already paused"));

    cleanup_cli_test_env();
}

#[test]
#[serial]
fn test_resume_already_active_task() {
    let _temp_dir = setup_cli_test_env();

    // Start a task
    let mut cmd = Command::cargo_bin("timelog").unwrap();
    cmd.args(&["start", "active task"]).assert().success();

    // Try to resume (should fail since it's already active)
    let mut cmd = Command::cargo_bin("timelog").unwrap();
    cmd.arg("resume")
        .assert()
        .failure()
        .stderr(predicate::str::contains("task is already running"));

    cleanup_cli_test_env();
}

#[test]
#[serial]
fn test_stop_without_task() {
    let _temp_dir = setup_cli_test_env();

    let mut cmd = Command::cargo_bin("timelog").unwrap();
    cmd.arg("stop")
        .assert()
        .failure()
        .stderr(predicate::str::contains("no task to stop"));

    cleanup_cli_test_env();
}

#[test]
#[serial]
fn test_stop_paused_task() {
    let _temp_dir = setup_cli_test_env();

    // Start, pause, then stop a task
    let mut cmd = Command::cargo_bin("timelog").unwrap();
    cmd.args(&["start", "paused task"]).assert().success();

    thread::sleep(Duration::from_millis(50));

    let mut cmd = Command::cargo_bin("timelog").unwrap();
    cmd.arg("pause").assert().success();

    let mut cmd = Command::cargo_bin("timelog").unwrap();
    cmd.arg("stop")
        .assert()
        .success()
        .stdout(predicate::str::contains("recorded"))
        .stdout(predicate::str::contains("paused task"));

    cleanup_cli_test_env();
}

#[test]
#[serial]
fn test_upload_list_plugins() {
    let _temp_dir = setup_cli_test_env();

    let mut cmd = Command::cargo_bin("timelog").unwrap();
    cmd.args(&["upload", "--list-plugins"])
        .assert()
        .success()
        .stdout(predicate::str::contains("No plugins found"));

    cleanup_cli_test_env();
}

#[test]
#[serial]
fn test_different_period_types() {
    let _temp_dir = setup_cli_test_env();

    // The command should accept different period types without crashing
    let periods = &[
        "today",
        "yesterday",
        "this-week",
        "last-week",
        "this-month",
        "last-month",
        "ytd",
        "last-year",
    ];

    for period in periods {
        let mut cmd = Command::cargo_bin("timelog").unwrap();
        cmd.args(&["report", period])
            .assert()
            .failure() // Will fail due to no records, but should parse the period correctly
            .stderr(predicate::str::contains("no records found"));
    }

    cleanup_cli_test_env();
}

#[test]
#[serial]
fn test_help_commands() {
    // Test main help
    let mut cmd = Command::cargo_bin("timelog").unwrap();
    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Usage:"))
        .stdout(predicate::str::contains("Commands:"));

    // Test subcommand help
    let mut cmd = Command::cargo_bin("timelog").unwrap();
    cmd.args(&["start", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Usage:"))
        .stdout(predicate::str::contains("TASK"));
}

#[test]
#[serial]
fn test_version() {
    let mut cmd = Command::cargo_bin("timelog").unwrap();
    cmd.arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("timelog"));
}
