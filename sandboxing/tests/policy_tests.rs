mod common;

use common::setup_plugin;

#[test]
fn test_file_creation_allowed_in_data_dir() {
    let (mut store, plugin) = setup_plugin().expect("Failed to setup plugin");
    
    let result = plugin.example_plugin_policy()
        .call_create_file(&mut store, "test_output.txt", "Test content")
        .expect("Failed to call create_file");
    
    assert!(result.contains("Success"), "Expected success but got: {}", result);
}

#[test]
fn test_file_creation_denied_outside_data_dir() {
    let (mut store, plugin) = setup_plugin().expect("Failed to setup plugin");
    
    let result = plugin.example_plugin_policy()
        .call_create_file(&mut store, "../secret.txt", "Blocked content")
        .expect("Failed to call create_file");
    
    assert!(result.contains("Error"), "Expected error but got: {}", result);
}

#[test]
fn test_check_key_deny() {
    let (mut store, plugin) = setup_plugin().expect("Failed to setup plugin");
    
    let json = r#"{"status": "please deny this"}"#;
    let result = plugin.example_plugin_policy()
        .call_check_key(&mut store, json, "status")
        .expect("Failed to call check_key");
    
    assert_eq!(result, "deny");
}

#[test]
fn test_check_key_allow() {
    let (mut store, plugin) = setup_plugin().expect("Failed to setup plugin");
    
    let json = r#"{"status": "please allow this"}"#;
    let result = plugin.example_plugin_policy()
        .call_check_key(&mut store, json, "status")
        .expect("Failed to call check_key");
    
    assert_eq!(result, "allow");
}

#[test]
fn test_file_creation_blocked_by_read_only_policy() {
    let (mut store, plugin) = setup_plugin().expect("Failed to setup plugin");
    
    // With read-only permissions, file creation should fail
    let result = plugin.example_plugin_policy()
        .call_create_file(&mut store, "test_output.txt", "Test content")
        .expect("Failed to call create_file");
    
    assert!(result.contains("Error") || result.contains("Operation not permitted"), 
            "Expected permission error but got: {}", result);
}

