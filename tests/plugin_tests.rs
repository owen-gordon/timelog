use tempfile::TempDir;
use std::env;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use serde_json;
use serial_test::serial;

use timelog::*;

/// Helper to set up a clean test environment with temporary directories
fn setup_plugin_test_env() -> TempDir {
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
fn cleanup_plugin_test_env() {
    unsafe {
        env::remove_var("TIMELOG_RECORD_PATH");
        env::remove_var("TIMELOG_STATE_PATH");
        env::remove_var("TIMELOG_PLUGIN_PATH");
    }
}

#[test]
#[serial]
fn test_plugin_discovery_empty_directory() {
    let _temp_dir = setup_plugin_test_env();
    
    let plugins = discover_plugins();
    assert_eq!(plugins.len(), 0);
    
    cleanup_plugin_test_env();
}

#[test]
#[serial]
fn test_plugin_discovery_with_plugins() {
    let _temp_dir = setup_plugin_test_env();
    
    let plugin_dir = plugin_dir();
    
    // Create valid plugins
    let plugin1_path = plugin_dir.join("timelog-test1");
    let plugin2_path = plugin_dir.join("timelog-another");
    
    fs::write(&plugin1_path, "#!/bin/bash\necho 'test'").expect("Failed to create plugin1");
    fs::write(&plugin2_path, "#!/bin/bash\necho 'test'").expect("Failed to create plugin2");
    
    // Make them executable
    let mut perms1 = fs::metadata(&plugin1_path).unwrap().permissions();
    perms1.set_mode(0o755);
    fs::set_permissions(&plugin1_path, perms1).expect("Failed to set permissions");
    
    let mut perms2 = fs::metadata(&plugin2_path).unwrap().permissions();
    perms2.set_mode(0o755);
    fs::set_permissions(&plugin2_path, perms2).expect("Failed to set permissions");
    
    let plugins = discover_plugins();
    assert_eq!(plugins.len(), 2);
    assert!(plugins.contains(&"test1".to_string()));
    assert!(plugins.contains(&"another".to_string()));
    
    cleanup_plugin_test_env();
}

#[test]
#[serial]
fn test_plugin_discovery_ignores_non_executable() {
    let _temp_dir = setup_plugin_test_env();
    
    let plugin_dir = plugin_dir();
    
    // Create a non-executable file with timelog- prefix
    let non_exec_path = plugin_dir.join("timelog-notexec");
    fs::write(&non_exec_path, "#!/bin/bash\necho 'test'").expect("Failed to create non-exec file");
    // Don't make it executable
    
    // Create executable plugin
    let plugin_path = plugin_dir.join("timelog-good");
    fs::write(&plugin_path, "#!/bin/bash\necho 'test'").expect("Failed to create plugin");
    let mut perms = fs::metadata(&plugin_path).unwrap().permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&plugin_path, perms).expect("Failed to set permissions");
    
    let plugins = discover_plugins();
    assert_eq!(plugins.len(), 1);
    assert!(plugins.contains(&"good".to_string()));
    assert!(!plugins.contains(&"notexec".to_string()));
    
    cleanup_plugin_test_env();
}

#[test]
#[serial]
fn test_plugin_discovery_ignores_config_files() {
    let _temp_dir = setup_plugin_test_env();
    
    let plugin_dir = plugin_dir();
    
    // Create plugin and its config file
    let plugin_path = plugin_dir.join("timelog-withconfig");
    let config_path = plugin_dir.join("timelog-withconfig.json");
    
    fs::write(&plugin_path, "#!/bin/bash\necho 'test'").expect("Failed to create plugin");
    fs::write(&config_path, "{}").expect("Failed to create config");
    
    let mut perms = fs::metadata(&plugin_path).unwrap().permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&plugin_path, perms).expect("Failed to set permissions");
    
    let plugins = discover_plugins();
    assert_eq!(plugins.len(), 1);
    assert!(plugins.contains(&"withconfig".to_string()));
    
    cleanup_plugin_test_env();
}

#[test]
#[serial]
fn test_plugin_discovery_ignores_non_timelog_files() {
    let _temp_dir = setup_plugin_test_env();
    
    let plugin_dir = plugin_dir();
    
    // Create files that don't start with "timelog-"
    let other_file = plugin_dir.join("other-script");
    let readme_file = plugin_dir.join("README.md");
    
    fs::write(&other_file, "#!/bin/bash\necho 'test'").expect("Failed to create other file");
    fs::write(&readme_file, "# Plugins").expect("Failed to create readme");
    
    let mut perms = fs::metadata(&other_file).unwrap().permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&other_file, perms).expect("Failed to set permissions");
    
    // Create valid plugin
    let plugin_path = plugin_dir.join("timelog-valid");
    fs::write(&plugin_path, "#!/bin/bash\necho 'test'").expect("Failed to create plugin");
    let mut perms = fs::metadata(&plugin_path).unwrap().permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&plugin_path, perms).expect("Failed to set permissions");
    
    let plugins = discover_plugins();
    assert_eq!(plugins.len(), 1);
    assert!(plugins.contains(&"valid".to_string()));
    
    cleanup_plugin_test_env();
}

#[test]
#[serial]
fn test_plugin_input_structure() {
    let _temp_dir = setup_plugin_test_env();
    
    // Create test records
    let records = vec![
        Record {
            task: "task1".to_string(),
            duration_ms: 3600000,
            date: chrono::NaiveDate::from_ymd_opt(2024, 1, 15).unwrap(),
            project: Some("project1".to_string()),
        },
        Record {
            task: "task2".to_string(),
            duration_ms: 1800000,
            date: chrono::NaiveDate::from_ymd_opt(2024, 1, 15).unwrap(),
            project: None,
        },
    ];
    
    let config = serde_json::json!({
        "api_key": "test_key",
        "endpoint": "https://api.example.com"
    });
    
    let input = PluginInput {
        records,
        period: "today".to_string(),
        config,
    };
    
    // Test serialization
    let serialized = serde_json::to_string(&input).expect("Failed to serialize");
    
    // Verify structure
    let parsed: serde_json::Value = serde_json::from_str(&serialized).expect("Failed to parse");
    
    assert_eq!(parsed["period"], "today");
    assert_eq!(parsed["records"].as_array().unwrap().len(), 2);
    assert_eq!(parsed["records"][0]["task"], "task1");
    assert_eq!(parsed["records"][0]["project"], "project1");
    assert_eq!(parsed["records"][1]["task"], "task2");
    assert!(parsed["records"][1]["project"].is_null());
    assert_eq!(parsed["config"]["api_key"], "test_key");
    
    cleanup_plugin_test_env();
}

#[test]
#[serial]
fn test_plugin_execution_success() {
    let _temp_dir = setup_plugin_test_env();
    
    let plugin_dir = plugin_dir();
    let plugin_path = plugin_dir.join("timelog-test");
    
    // Create a test plugin that returns success
    let plugin_script = r#"#!/bin/bash
read input
echo '{
    "success": true,
    "message": "Test plugin executed successfully",
    "uploaded_count": 2,
    "errors": []
}'
"#;
    
    fs::write(&plugin_path, plugin_script).expect("Failed to create plugin");
    let mut perms = fs::metadata(&plugin_path).unwrap().permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&plugin_path, perms).expect("Failed to set permissions");
    
    // Create test input
    let records = vec![
        Record {
            task: "test".to_string(),
            duration_ms: 1000,
            date: chrono::NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            project: None,
        }
    ];
    
    let input = PluginInput {
        records,
        period: "today".to_string(),
        config: serde_json::Value::Object(serde_json::Map::new()),
    };
    
    // Execute plugin
    let result = execute_plugin("test", &input, false);
    
    assert!(result.is_ok());
    let output = result.unwrap();
    assert!(output.success);
    assert_eq!(output.message, "Test plugin executed successfully");
    assert_eq!(output.uploaded_count, Some(2));
    assert!(output.errors.is_empty());
    
    cleanup_plugin_test_env();
}

#[test]
#[serial]
fn test_plugin_execution_failure() {
    let _temp_dir = setup_plugin_test_env();
    
    let plugin_dir = plugin_dir();
    let plugin_path = plugin_dir.join("timelog-fail");
    
    // Create a test plugin that returns failure
    let plugin_script = r#"#!/bin/bash
read input
echo '{
    "success": false,
    "message": "Plugin failed intentionally",
    "uploaded_count": null,
    "errors": ["Error 1", "Error 2"]
}'
exit 1
"#;
    
    fs::write(&plugin_path, plugin_script).expect("Failed to create plugin");
    let mut perms = fs::metadata(&plugin_path).unwrap().permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&plugin_path, perms).expect("Failed to set permissions");
    
    let records = vec![];
    let input = PluginInput {
        records,
        period: "today".to_string(),
        config: serde_json::Value::Object(serde_json::Map::new()),
    };
    
    // Execute plugin
    let result = execute_plugin("fail", &input, false);
    
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(error.contains("Plugin failed"));
    
    cleanup_plugin_test_env();
}

#[test]
#[serial]
fn test_plugin_execution_dry_run() {
    let _temp_dir = setup_plugin_test_env();
    
    let plugin_dir = plugin_dir();
    let plugin_path = plugin_dir.join("timelog-dryrun");
    
    // Create a test plugin that handles dry-run flag
    let plugin_script = r#"#!/bin/bash
read input
if [ "$1" = "--dry-run" ]; then
    echo '{
        "success": true,
        "message": "Dry run: would upload data",
        "uploaded_count": 0,
        "errors": []
    }'
else
    echo '{
        "success": true,
        "message": "Actually uploaded data",
        "uploaded_count": 1,
        "errors": []
    }'
fi
"#;
    
    fs::write(&plugin_path, plugin_script).expect("Failed to create plugin");
    let mut perms = fs::metadata(&plugin_path).unwrap().permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&plugin_path, perms).expect("Failed to set permissions");
    
    let records = vec![];
    let input = PluginInput {
        records,
        period: "today".to_string(),
        config: serde_json::Value::Object(serde_json::Map::new()),
    };
    
    // Test dry run
    let result = execute_plugin("dryrun", &input, true);
    assert!(result.is_ok());
    let output = result.unwrap();
    assert!(output.success);
    assert!(output.message.contains("Dry run"));
    assert_eq!(output.uploaded_count, Some(0));
    
    // Test actual run
    let result = execute_plugin("dryrun", &input, false);
    assert!(result.is_ok());
    let output = result.unwrap();
    assert!(output.success);
    assert!(output.message.contains("Actually uploaded"));
    assert_eq!(output.uploaded_count, Some(1));
    
    cleanup_plugin_test_env();
}

#[test]
#[serial]
fn test_plugin_execution_nonexistent() {
    let _temp_dir = setup_plugin_test_env();
    
    let records = vec![];
    let input = PluginInput {
        records,
        period: "today".to_string(),
        config: serde_json::Value::Object(serde_json::Map::new()),
    };
    
    // Try to execute non-existent plugin
    let result = execute_plugin("nonexistent", &input, false);
    
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(error.contains("not found"));
    
    cleanup_plugin_test_env();
}

#[test]
#[serial]
fn test_plugin_execution_invalid_json() {
    let _temp_dir = setup_plugin_test_env();
    
    let plugin_dir = plugin_dir();
    let plugin_path = plugin_dir.join("timelog-badjson");
    
    // Create a test plugin that returns invalid JSON
    let plugin_script = r#"#!/bin/bash
read input
echo 'invalid json output'
"#;
    
    fs::write(&plugin_path, plugin_script).expect("Failed to create plugin");
    let mut perms = fs::metadata(&plugin_path).unwrap().permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&plugin_path, perms).expect("Failed to set permissions");
    
    let records = vec![];
    let input = PluginInput {
        records,
        period: "today".to_string(),
        config: serde_json::Value::Object(serde_json::Map::new()),
    };
    
    // Execute plugin
    let result = execute_plugin("badjson", &input, false);
    
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(error.contains("Failed to parse"));
    
    cleanup_plugin_test_env();
}

#[test]
#[serial]
fn test_plugin_with_config() {
    let _temp_dir = setup_plugin_test_env();
    
    let plugin_dir = plugin_dir();
    
    // Create plugin config
    let config_path = plugin_dir.join("timelog-configured.json");
    let config_content = serde_json::json!({
        "api_endpoint": "https://example.com/api",
        "timeout": 30,
        "api_key": "secret123"
    });
    fs::write(&config_path, serde_json::to_string_pretty(&config_content).unwrap())
        .expect("Failed to write config");
    
    // Create plugin that uses config
    let plugin_path = plugin_dir.join("timelog-configured");
    let plugin_script = r#"#!/bin/bash
read input
# Extract config from input (basic check that config is present)
if echo "$input" | grep -q '"api_endpoint":"https://example.com/api"'; then
    echo '{
        "success": true,
        "message": "Config loaded correctly",
        "uploaded_count": 1,
        "errors": []
    }'
else
    echo '{
        "success": false,
        "message": "Config not found or incorrect",
        "uploaded_count": null,
        "errors": ["Missing config"]
    }'
fi
"#;
    
    fs::write(&plugin_path, plugin_script).expect("Failed to create plugin");
    let mut perms = fs::metadata(&plugin_path).unwrap().permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&plugin_path, perms).expect("Failed to set permissions");
    
    let records = vec![];
    let input = PluginInput {
        records,
        period: "today".to_string(),
        config: config_content,
    };
    
    // Execute plugin
    let result = execute_plugin("configured", &input, false);
    
    assert!(result.is_ok());
    let output = result.unwrap();
    assert!(output.success);
    assert_eq!(output.message, "Config loaded correctly");
    
    cleanup_plugin_test_env();
}
