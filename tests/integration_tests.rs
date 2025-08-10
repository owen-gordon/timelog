use tempfile::TempDir;
use std::env;
use std::fs;
use std::path::Path;
use chrono::{NaiveDate, Utc};
use serde_json;
use serial_test::serial;

use timelog::*;

/// Helper to set up a clean test environment with temporary directories
fn setup_test_env() -> TempDir {
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
fn cleanup_test_env() {
    unsafe {
        env::remove_var("TIMELOG_RECORD_PATH");
        env::remove_var("TIMELOG_STATE_PATH");
        env::remove_var("TIMELOG_PLUGIN_PATH");
    }
}

#[test]
#[serial]
fn test_state_operations() {
    let _temp_dir = setup_test_env();
    
    // Test saving and loading state
    let original_state = State {
        timestamp: Utc::now(),
        task: "test task".to_string(),
        active: true,
        project: Some("test project".to_string()),
    };
    
    // Save state
    assert!(save_state(&original_state).is_ok());
    
    // Load state
    let loaded_state = load_state().expect("Failed to load state");
    assert_eq!(loaded_state.task, original_state.task);
    assert_eq!(loaded_state.active, original_state.active);
    assert_eq!(loaded_state.project, original_state.project);
    
    // Delete state
    assert!(delete_state().is_ok());
    
    // Verify state is deleted
    assert!(load_state().is_err());
    
    cleanup_test_env();
}

#[test]
#[serial]
fn test_record_operations() {
    let _temp_dir = setup_test_env();
    
    let record1 = Record {
        task: "task1".to_string(),
        duration_ms: 3600000, // 1 hour
        date: NaiveDate::from_ymd_opt(2024, 1, 15).unwrap(),
        project: Some("project1".to_string()),
    };
    
    let record2 = Record {
        task: "task2".to_string(),
        duration_ms: 1800000, // 30 minutes
        date: NaiveDate::from_ymd_opt(2024, 1, 16).unwrap(),
        project: None,
    };
    
    // Save records
    assert!(save_record(&record1).is_ok());
    assert!(save_record(&record2).is_ok());
    
    // Load records
    let loaded_records = load_records().expect("Failed to load records");
    assert_eq!(loaded_records.len(), 2);
    
    // Verify first record
    assert_eq!(loaded_records[0].task, record1.task);
    assert_eq!(loaded_records[0].duration_ms, record1.duration_ms);
    assert_eq!(loaded_records[0].date, record1.date);
    assert_eq!(loaded_records[0].project, record1.project);
    
    // Verify second record
    assert_eq!(loaded_records[1].task, record2.task);
    assert_eq!(loaded_records[1].duration_ms, record2.duration_ms);
    assert_eq!(loaded_records[1].date, record2.date);
    assert_eq!(loaded_records[1].project, record2.project);
    
    cleanup_test_env();
}

#[test]
#[serial]
fn test_backwards_compatibility() {
    let _temp_dir = setup_test_env();
    
    // Write old format CSV (without project column)
    let old_csv_content = "task,duration_ms,date\ntask1,3600000,2024-01-15\ntask2,1800000,2024-01-16";
    fs::write(record_path(), old_csv_content).expect("Failed to write old format CSV");
    
    // Load records should work with old format
    let loaded_records = load_records().expect("Failed to load old format records");
    assert_eq!(loaded_records.len(), 2);
    
    // Verify records have None for project
    assert_eq!(loaded_records[0].project, None);
    assert_eq!(loaded_records[1].project, None);
    
    // Verify other fields
    assert_eq!(loaded_records[0].task, "task1");
    assert_eq!(loaded_records[0].duration_ms, 3600000);
    
    cleanup_test_env();
}

#[test]
#[serial]
fn test_plugin_discovery() {
    let _temp_dir = setup_test_env();
    
    let plugin_dir = plugin_dir();
    
    // Create some test plugins
    let plugin1_path = plugin_dir.join("timelog-test1");
    let plugin2_path = plugin_dir.join("timelog-test2");
    let config_path = plugin_dir.join("timelog-test1.json");
    let non_plugin_path = plugin_dir.join("some-other-file");
    
    // Create executable plugin files
    fs::write(&plugin1_path, "#!/bin/bash\necho 'test plugin'").expect("Failed to create plugin1");
    fs::write(&plugin2_path, "#!/bin/bash\necho 'test plugin'").expect("Failed to create plugin2");
    fs::write(&config_path, "{}").expect("Failed to create config");
    fs::write(&non_plugin_path, "not a plugin").expect("Failed to create non-plugin");
    
    // Make plugins executable
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms1 = fs::metadata(&plugin1_path).unwrap().permissions();
        perms1.set_mode(0o755);
        fs::set_permissions(&plugin1_path, perms1).expect("Failed to set permissions");
        
        let mut perms2 = fs::metadata(&plugin2_path).unwrap().permissions();
        perms2.set_mode(0o755);
        fs::set_permissions(&plugin2_path, perms2).expect("Failed to set permissions");
    }
    
    // Discover plugins
    let plugins = discover_plugins();
    
    // Should find exactly 2 plugins (excluding config files and non-executable files)
    assert_eq!(plugins.len(), 2);
    assert!(plugins.contains(&"test1".to_string()));
    assert!(plugins.contains(&"test2".to_string()));
    
    cleanup_test_env();
}

#[test]
#[serial]
fn test_path_functions() {
    // Store original values
    let orig_record = env::var("TIMELOG_RECORD_PATH").ok();
    let orig_state = env::var("TIMELOG_STATE_PATH").ok();
    let orig_plugin = env::var("TIMELOG_PLUGIN_PATH").ok();
    
    // Test default paths
    unsafe {
        env::remove_var("TIMELOG_RECORD_PATH");
        env::remove_var("TIMELOG_STATE_PATH");
        env::remove_var("TIMELOG_PLUGIN_PATH");
    }
    
    let home = env::var("HOME").expect("HOME not set");
    
    assert_eq!(record_path(), Path::new(&home).join(".timelog-record"));
    assert_eq!(state_path(), Path::new(&home).join(".timelog-state"));
    assert_eq!(plugin_dir(), Path::new(&home).join(".timelog").join("plugins"));
    
    // Test custom paths
    unsafe {
        env::set_var("TIMELOG_RECORD_PATH", "/custom/record/path");
        env::set_var("TIMELOG_STATE_PATH", "/custom/state/path");
        env::set_var("TIMELOG_PLUGIN_PATH", "/custom/plugin/path");
    }
    
    assert_eq!(record_path(), Path::new("/custom/record/path"));
    assert_eq!(state_path(), Path::new("/custom/state/path"));
    assert_eq!(plugin_dir(), Path::new("/custom/plugin/path"));
    
    // Restore original values
    unsafe {
        if let Some(val) = orig_record {
            env::set_var("TIMELOG_RECORD_PATH", val);
        } else {
            env::remove_var("TIMELOG_RECORD_PATH");
        }
        if let Some(val) = orig_state {
            env::set_var("TIMELOG_STATE_PATH", val);
        } else {
            env::remove_var("TIMELOG_STATE_PATH");
        }
        if let Some(val) = orig_plugin {
            env::set_var("TIMELOG_PLUGIN_PATH", val);
        } else {
            env::remove_var("TIMELOG_PLUGIN_PATH");
        }
    }
}

#[test]
fn test_period_filtering() {
    let records = vec![
        Record {
            task: "task1".to_string(),
            duration_ms: 3600000,
            date: NaiveDate::from_ymd_opt(2024, 1, 15).unwrap(), // Monday
            project: None,
        },
        Record {
            task: "task2".to_string(),
            duration_ms: 1800000,
            date: NaiveDate::from_ymd_opt(2024, 1, 16).unwrap(), // Tuesday
            project: None,
        },
        Record {
            task: "task3".to_string(),
            duration_ms: 2700000,
            date: NaiveDate::from_ymd_opt(2024, 1, 17).unwrap(), // Wednesday
            project: None,
        },
    ];
    
    let today = NaiveDate::from_ymd_opt(2024, 1, 17).unwrap(); // Wednesday
    
    // Test this week (should include Monday, Tuesday, Wednesday)
    let (start, end) = period_range(Period::ThisWeek, today);
    let filtered: Vec<&Record> = records.iter()
        .filter(|r| r.date >= start && r.date <= end)
        .collect();
    assert_eq!(filtered.len(), 3);
    
    // Test today (should include only Wednesday)
    let (start, end) = period_range(Period::Today, today);
    let filtered: Vec<&Record> = records.iter()
        .filter(|r| r.date >= start && r.date <= end)
        .collect();
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].task, "task3");
}

#[test]
fn test_project_filtering() {
    let records = vec![
        Record {
            task: "task1".to_string(),
            duration_ms: 3600000,
            date: NaiveDate::from_ymd_opt(2024, 1, 15).unwrap(),
            project: Some("project1".to_string()),
        },
        Record {
            task: "task2".to_string(),
            duration_ms: 1800000,
            date: NaiveDate::from_ymd_opt(2024, 1, 16).unwrap(),
            project: Some("project2".to_string()),
        },
        Record {
            task: "task3".to_string(),
            duration_ms: 2700000,
            date: NaiveDate::from_ymd_opt(2024, 1, 17).unwrap(),
            project: None,
        },
    ];
    
    // Filter by project1
    let filtered: Vec<&Record> = records.iter()
        .filter(|r| r.project.as_ref().map_or(false, |p| p == "project1"))
        .collect();
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].task, "task1");
    
    // Filter by project2
    let filtered: Vec<&Record> = records.iter()
        .filter(|r| r.project.as_ref().map_or(false, |p| p == "project2"))
        .collect();
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].task, "task2");
    
    // Filter for no project (None)
    let filtered: Vec<&Record> = records.iter()
        .filter(|r| r.project.is_none())
        .collect();
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].task, "task3");
}

#[test]
#[serial]
fn test_plugin_input_serialization() {
    let records = vec![
        Record {
            task: "task1".to_string(),
            duration_ms: 3600000,
            date: NaiveDate::from_ymd_opt(2024, 1, 15).unwrap(),
            project: Some("project1".to_string()),
        }
    ];
    
    let config = serde_json::json!({
        "api_key": "test_key",
        "endpoint": "https://example.com"
    });
    
    let input = PluginInput {
        records,
        period: "today".to_string(),
        config,
    };
    
    // Test serialization
    let serialized = serde_json::to_string(&input).expect("Failed to serialize");
    assert!(serialized.contains("task1"));
    assert!(serialized.contains("project1"));
    assert!(serialized.contains("today"));
    assert!(serialized.contains("api_key"));
    
    // Test that it can be parsed back (by a plugin)
    let parsed: serde_json::Value = serde_json::from_str(&serialized).expect("Failed to parse");
    assert_eq!(parsed["period"], "today");
    assert_eq!(parsed["records"][0]["task"], "task1");
}

#[test]
fn test_error_handling() {
    // Test loading state when file doesn't exist
    unsafe {
        env::set_var("TIMELOG_STATE_PATH", "/nonexistent/path/state.json");
    }
    assert!(load_state().is_err());
    
    // Test loading records when file doesn't exist
    unsafe {
        env::set_var("TIMELOG_RECORD_PATH", "/nonexistent/path/records.csv");
    }
    assert!(load_records().is_err());
    
    // Clean up
    unsafe {
        env::remove_var("TIMELOG_STATE_PATH");
        env::remove_var("TIMELOG_RECORD_PATH");
    }
}

#[test]
fn test_duration_edge_cases() {
    // Test zero duration
    assert_eq!(fmt_duration(0), "00h00m");
    
    // Test exact minutes
    assert_eq!(fmt_duration(60000), "00h01m");
    assert_eq!(fmt_duration(3600000), "01h00m");
    
    // Test mixed durations
    assert_eq!(fmt_duration(3661000), "01h01m01s");
    
    // Test large durations
    assert_eq!(fmt_duration(86400000), "24h00m"); // 24 hours
    
    // Test millisecond formatting
    assert_eq!(fmt_hms_ms(1500), "00:00:01.500");
    assert_eq!(fmt_hms_ms(3661500), "01:01:01.500");
}

#[test]
fn test_date_formatting() {
    let date = NaiveDate::from_ymd_opt(2024, 1, 15).unwrap();
    let record = Record {
        task: "test task".to_string(),
        duration_ms: 3600000,
        date,
        project: None,
    };
    
    // Test formatting for different periods
    let formatted_today = fmt_record_for_period(&record, Period::Today, date);
    assert!(formatted_today.contains("Today"));
    
    let formatted_week = fmt_record_for_period(&record, Period::ThisWeek, date);
    assert!(formatted_week.contains("Mon")); // Jan 15, 2024 is a Monday
    
    let formatted_ytd = fmt_record_for_period(&record, Period::YTD, date);
    assert!(formatted_ytd.contains("2024-01-15"));
}
