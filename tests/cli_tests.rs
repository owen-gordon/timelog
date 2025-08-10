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
        env::set_var("TIMELOG_RECORD_PATH", format!("{temp_path}/records.csv"));
        env::set_var("TIMELOG_STATE_PATH", format!("{temp_path}/state.json"));
        env::set_var("TIMELOG_PLUGIN_PATH", format!("{temp_path}/plugins"));
    }

    // Create plugins directory
    fs::create_dir_all(format!("{temp_path}/plugins")).expect("Failed to create plugins dir");

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
    cmd.args(["start", "test task"])
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
    cmd.args(["start", "test task", "--project", "test project"])
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
    cmd.args(["start", "first task"]).assert().success();

    // Try to start second task - should fail
    let mut cmd = Command::cargo_bin("timelog").unwrap();
    cmd.args(["start", "second task"])
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
    cmd.args(["start", "pausable task"]).assert().success();

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
    cmd.args(["start", "stoppable task"]).assert().success();

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
    cmd.args(["report", "today"])
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
    cmd.args(["start", "workflow task"]).assert().success();

    // Wait a moment to ensure measurable duration
    thread::sleep(Duration::from_millis(100));

    let mut cmd = Command::cargo_bin("timelog").unwrap();
    cmd.arg("stop").assert().success();

    // Generate report
    let mut cmd = Command::cargo_bin("timelog").unwrap();
    cmd.args(["report", "today"])
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
    cmd.args(["start", "project1 task", "--project", "project1"])
        .assert()
        .success();

    thread::sleep(Duration::from_millis(50));

    let mut cmd = Command::cargo_bin("timelog").unwrap();
    cmd.arg("stop").assert().success();

    let mut cmd = Command::cargo_bin("timelog").unwrap();
    cmd.args(["start", "project2 task", "--project", "project2"])
        .assert()
        .success();

    thread::sleep(Duration::from_millis(50));

    let mut cmd = Command::cargo_bin("timelog").unwrap();
    cmd.arg("stop").assert().success();

    // Test project filtering
    let mut cmd = Command::cargo_bin("timelog").unwrap();
    cmd.args(["report", "today", "--project", "project1"])
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
    cmd.args(["start", "pausable task"]).assert().success();

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
    cmd.args(["start", "active task"]).assert().success();

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
    cmd.args(["start", "paused task"]).assert().success();

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
    cmd.args(["upload", "--list-plugins"])
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
        cmd.args(["report", period])
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
    cmd.args(["start", "--help"])
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

#[test]
#[serial]
fn test_amend_task_name() {
    let _temp_dir = setup_cli_test_env();

    // Create a task first
    Command::cargo_bin("timelog")
        .unwrap()
        .args(["start", "original task", "--project", "testproj"])
        .assert()
        .success();

    thread::sleep(Duration::from_millis(100));

    Command::cargo_bin("timelog")
        .unwrap()
        .args(["stop"])
        .assert()
        .success();

    // Amend the task name
    let mut cmd = Command::cargo_bin("timelog").unwrap();
    cmd.args([
        "amend",
        "--date",
        "2025-08-10", // Use today's date
        "--task",
        "original",
        "--new-task",
        "amended task",
    ])
    .assert()
    .success()
    .stdout(predicate::str::contains("Successfully amended"));

    cleanup_cli_test_env();
}

#[test]
#[serial]
fn test_amend_duration() {
    let _temp_dir = setup_cli_test_env();

    // Create a task first
    Command::cargo_bin("timelog")
        .unwrap()
        .args(["start", "test task"])
        .assert()
        .success();

    thread::sleep(Duration::from_millis(100));

    Command::cargo_bin("timelog")
        .unwrap()
        .args(["stop"])
        .assert()
        .success();

    // Amend the duration
    let mut cmd = Command::cargo_bin("timelog").unwrap();
    cmd.args([
        "amend",
        "--date",
        "2025-08-10",
        "--task",
        "test",
        "--new-duration",
        "30",
    ])
    .assert()
    .success()
    .stdout(predicate::str::contains("00:30:00.000"));

    cleanup_cli_test_env();
}

#[test]
#[serial]
fn test_amend_project() {
    let _temp_dir = setup_cli_test_env();

    // Create a task first
    Command::cargo_bin("timelog")
        .unwrap()
        .args(["start", "test task", "--project", "oldproj"])
        .assert()
        .success();

    thread::sleep(Duration::from_millis(100));

    Command::cargo_bin("timelog")
        .unwrap()
        .args(["stop"])
        .assert()
        .success();

    // Amend the project
    let mut cmd = Command::cargo_bin("timelog").unwrap();
    cmd.args([
        "amend",
        "--date",
        "2025-08-10",
        "--task",
        "test",
        "--new-project",
        "newproj",
    ])
    .assert()
    .success()
    .stdout(predicate::str::contains("oldproj → newproj"));

    cleanup_cli_test_env();
}

#[test]
#[serial]
fn test_amend_remove_project() {
    let _temp_dir = setup_cli_test_env();

    // Create a task first
    Command::cargo_bin("timelog")
        .unwrap()
        .args(["start", "test task", "--project", "someproj"])
        .assert()
        .success();

    thread::sleep(Duration::from_millis(100));

    Command::cargo_bin("timelog")
        .unwrap()
        .args(["stop"])
        .assert()
        .success();

    // Remove the project
    let mut cmd = Command::cargo_bin("timelog").unwrap();
    cmd.args([
        "amend",
        "--date",
        "2025-08-10",
        "--task",
        "test",
        "--new-project",
        "",
    ])
    .assert()
    .success()
    .stdout(predicate::str::contains("someproj → (none)"));

    cleanup_cli_test_env();
}

#[test]
#[serial]
fn test_amend_dry_run() {
    let _temp_dir = setup_cli_test_env();

    // Create a task first
    Command::cargo_bin("timelog")
        .unwrap()
        .args(["start", "test task"])
        .assert()
        .success();

    thread::sleep(Duration::from_millis(100));

    Command::cargo_bin("timelog")
        .unwrap()
        .args(["stop"])
        .assert()
        .success();

    // Dry run amendment
    let mut cmd = Command::cargo_bin("timelog").unwrap();
    cmd.args([
        "amend",
        "--date",
        "2025-08-10",
        "--task",
        "test",
        "--new-task",
        "changed task",
        "--dry-run",
    ])
    .assert()
    .success()
    .stdout(predicate::str::contains(
        "Dry run mode - no changes were made",
    ));

    cleanup_cli_test_env();
}

#[test]
#[serial]
fn test_amend_no_matching_record() {
    let _temp_dir = setup_cli_test_env();

    // Try to amend non-existent record
    let mut cmd = Command::cargo_bin("timelog").unwrap();
    cmd.args([
        "amend",
        "--date",
        "2025-08-10",
        "--task",
        "nonexistent",
        "--new-task",
        "changed",
    ])
    .assert()
    .failure()
    .stderr(predicate::str::contains("no records found"));

    cleanup_cli_test_env();
}

#[test]
#[serial]
fn test_amend_invalid_duration() {
    let _temp_dir = setup_cli_test_env();

    // Create a task first
    Command::cargo_bin("timelog")
        .unwrap()
        .args(["start", "test task"])
        .assert()
        .success();

    thread::sleep(Duration::from_millis(100));

    Command::cargo_bin("timelog")
        .unwrap()
        .args(["stop"])
        .assert()
        .success();

    // Try to set zero duration
    let mut cmd = Command::cargo_bin("timelog").unwrap();
    cmd.args([
        "amend",
        "--date",
        "2025-08-10",
        "--task",
        "test",
        "--new-duration",
        "0",
    ])
    .assert()
    .failure()
    .stderr(predicate::str::contains("Duration must be positive"));

    cleanup_cli_test_env();
}

#[test]
#[serial]
fn test_amend_no_changes_specified() {
    let _temp_dir = setup_cli_test_env();

    // Create a task first
    Command::cargo_bin("timelog")
        .unwrap()
        .args(["start", "test task"])
        .assert()
        .success();

    thread::sleep(Duration::from_millis(100));

    Command::cargo_bin("timelog")
        .unwrap()
        .args(["stop"])
        .assert()
        .success();

    // Try to amend without specifying changes
    let mut cmd = Command::cargo_bin("timelog").unwrap();
    cmd.args(["amend", "--date", "2025-08-10", "--task", "test"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("No changes specified"));

    cleanup_cli_test_env();
}

#[test]
#[serial]
fn test_amend_multiple_changes() {
    let _temp_dir = setup_cli_test_env();

    // Create a task first
    Command::cargo_bin("timelog")
        .unwrap()
        .args(["start", "original task", "--project", "oldproj"])
        .assert()
        .success();

    thread::sleep(Duration::from_millis(100));

    Command::cargo_bin("timelog")
        .unwrap()
        .args(["stop"])
        .assert()
        .success();

    // Amend multiple fields
    let mut cmd = Command::cargo_bin("timelog").unwrap();
    cmd.args([
        "amend",
        "--date",
        "2025-08-10",
        "--task",
        "original",
        "--new-task",
        "updated task",
        "--new-duration",
        "60",
        "--new-project",
        "newproj",
    ])
    .assert()
    .success()
    .stdout(predicate::str::contains(
        "task: 'original task' → 'updated task'",
    ))
    .stdout(predicate::str::contains("01:00:00.000"))
    .stdout(predicate::str::contains("oldproj → newproj"));

    cleanup_cli_test_env();
}
