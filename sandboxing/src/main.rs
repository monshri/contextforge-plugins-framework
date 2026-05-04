use anyhow::Result;
use sandboxing::sandbox_manager::{PluginConfig, SandboxManager};
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== Sandbox Manager Demo ===\n");
    
    // Create the sandbox manager
    let manager = SandboxManager::new()?;
    println!("✓ Sandbox manager created\n");
    
    // Configure and load the first plugin
    let plugin_config = PluginConfig {
        id: "example-plugin-1".to_string(),
        name: "Example Plugin 1".to_string(),
        wasm_path: PathBuf::from("plugin/target/wasm32-wasip2/release/plugin.wasm"),
        policy_path: PathBuf::from("config/policy.yaml"),
        auto_restart: true,
        max_restart_attempts: 3,
    };
    
    println!("Loading plugin: {}", plugin_config.name);
    let plugin_id = manager.load_plugin(plugin_config).await?;
    println!("✓ Plugin loaded with ID: {}\n", plugin_id);
    
    // Get the plugin instance
    let plugin_instance = manager.get_plugin(&plugin_id).await?;
    
    // Check initial status
    let status = plugin_instance.get_status().await;
    println!("Plugin status: {:?}\n", status);
    
    // Get plugin and store for making calls
    let plugin = plugin_instance.get_plugin();
    let store = plugin_instance.get_store();
    
    // === Test 1: check_key ===
    println!("--- Test 1: Calling check_key ---");
    {
        let mut store_guard = store.lock().await;
        let plugin_guard = plugin.lock().await;
        
        let fuel_before = store_guard.get_fuel().unwrap_or(0);
        
        let result = plugin_guard
            .example_plugin_policy()
            .call_check_key(&mut *store_guard, r#"{"action": "allow"}"#, "action")
            .await;
        
        match result {
            Ok(res) => {
                println!("✓ Result: {}", res);
                let fuel_after = store_guard.get_fuel().unwrap_or(0);
                let fuel_consumed = fuel_before.saturating_sub(fuel_after);
                plugin_instance.update_stats(true, fuel_consumed).await;
            }
            Err(e) => {
                println!("✗ Error: {}", e);
                plugin_instance.update_stats(false, 0).await;
            }
        }
    }
    print_plugin_stats(&plugin_instance).await;
    
    // === Test 2: create_file ===
    println!("\n--- Test 2: Calling create_file ---");
    {
        let mut store_guard = store.lock().await;
        let plugin_guard = plugin.lock().await;
        
        let fuel_before = store_guard.get_fuel().unwrap_or(0);
        
        let result = plugin_guard
            .example_plugin_policy()
            .call_create_file(&mut *store_guard, "data/test_output.txt", "Hello from sandboxed plugin!")
            .await;
        
        match result {
            Ok(res) => {
                println!("✓ Result: {}", res);
                let fuel_after = store_guard.get_fuel().unwrap_or(0);
                let fuel_consumed = fuel_before.saturating_sub(fuel_after);
                plugin_instance.update_stats(true, fuel_consumed).await;
            }
            Err(e) => {
                println!("✗ Error: {}", e);
                plugin_instance.update_stats(false, 0).await;
            }
        }
    }
    print_plugin_stats(&plugin_instance).await;
    
    // === Test 3: HTTP request to allowed host ===
    println!("\n--- Test 3: HTTP request to allowed host ---");
    {
        let mut store_guard = store.lock().await;
        let plugin_guard = plugin.lock().await;
        
        let fuel_before = store_guard.get_fuel().unwrap_or(0);
        
        let result = plugin_guard
            .example_plugin_policy()
            .call_make_http_request(&mut *store_guard, "https://httpbin.org/get")
            .await;
        
        match result {
            Ok(res) => {
                println!("✓ Response from httpbin.org:\n{}", res);
                let fuel_after = store_guard.get_fuel().unwrap_or(0);
                let fuel_consumed = fuel_before.saturating_sub(fuel_after);
                plugin_instance.update_stats(true, fuel_consumed).await;
            }
            Err(e) => {
                println!("✗ Error: {}", e);
                plugin_instance.update_stats(false, 0).await;
            }
        }
    }
    print_plugin_stats(&plugin_instance).await;
    
    // === Test 4: HTTP request to denied host ===
    println!("\n--- Test 4: HTTP request to denied host ---");
    {
        let mut store_guard = store.lock().await;
        let plugin_guard = plugin.lock().await;
        
        let fuel_before = store_guard.get_fuel().unwrap_or(0);
        
        let result = plugin_guard
            .example_plugin_policy()
            .call_make_http_request(&mut *store_guard, "https://evil.example.test/")
            .await;
        
        match result {
            Ok(res) => {
                println!("✓ Response: {}", res);
                let fuel_after = store_guard.get_fuel().unwrap_or(0);
                let fuel_consumed = fuel_before.saturating_sub(fuel_after);
                plugin_instance.update_stats(true, fuel_consumed).await;
            }
            Err(e) => {
                println!("✗ Error (expected): {}", e);
                plugin_instance.update_stats(false, 0).await;
            }
        }
    }
    print_plugin_stats(&plugin_instance).await;
    
    // === Test 5: Environment variable access ===
    println!("\n--- Test 5: Environment variable access ---");
    
    // Test allowed env var (PATH)
    {
        let mut store_guard = store.lock().await;
        let plugin_guard = plugin.lock().await;
        
        let result = plugin_guard
            .example_plugin_policy()
            .call_get_env_var(&mut *store_guard, "PATH")
            .await;
        
        match result {
            Ok(res) => {
                println!("✓ Allowed env var (PATH): {}", res);
                plugin_instance.update_stats(true, 0).await;
            }
            Err(e) => {
                println!("✗ Error: {}", e);
                plugin_instance.update_stats(false, 0).await;
            }
        }
    }
    
    // Test denied env var (SECRET_KEY)
    {
        let mut store_guard = store.lock().await;
        let plugin_guard = plugin.lock().await;
        
        let result = plugin_guard
            .example_plugin_policy()
            .call_get_env_var(&mut *store_guard, "SECRET_KEY")
            .await;
        
        match result {
            Ok(res) => {
                println!("✓ Denied env var (SECRET_KEY): {}", res);
                plugin_instance.update_stats(true, 0).await;
            }
            Err(e) => {
                println!("✗ Error (expected): {}", e);
                plugin_instance.update_stats(false, 0).await;
            }
        }
    }
    print_plugin_stats(&plugin_instance).await;
    
    // === Lifecycle Management Demo ===
    println!("\n=== Lifecycle Management Demo ===\n");
    
    // List all plugins
    let plugin_list = manager.list_plugins().await;
    println!("Active plugins: {:?}", plugin_list);
    
    // Get all statuses
    let all_statuses = manager.get_all_statuses().await;
    println!("Plugin statuses: {:?}", all_statuses);
    
    // Health check
    println!("\n--- Health Check ---");
    let health_results = manager.health_check_all().await;
    for (id, result) in health_results {
        match result {
            Ok(healthy) => println!("Plugin '{}': {}", id, if healthy { "✓ Healthy" } else { "✗ Unhealthy" }),
            Err(e) => println!("Plugin '{}': ✗ Error - {}", id, e),
        }
    }
    
    // Pause plugin
    println!("\n--- Pausing Plugin ---");
    manager.pause_plugin(&plugin_id).await?;
    println!("✓ Plugin paused");
    println!("Status: {:?}", plugin_instance.get_status().await);
    
    // Resume plugin
    println!("\n--- Resuming Plugin ---");
    manager.resume_plugin(&plugin_id).await?;
    println!("✓ Plugin resumed");
    println!("Status: {:?}", plugin_instance.get_status().await);
    
    // Get all statistics
    println!("\n--- Final Statistics ---");
    let all_stats = manager.get_all_stats().await;
    for (id, stats) in all_stats {
        println!("\nPlugin '{}' statistics:", id);
        println!("  Total calls: {}", stats.total_calls);
        println!("  Failed calls: {}", stats.failed_calls);
        println!("  Success rate: {:.1}%", 
            if stats.total_calls > 0 {
                ((stats.total_calls - stats.failed_calls) as f64 / stats.total_calls as f64) * 100.0
            } else {
                0.0
            }
        );
        println!("  Fuel consumed: {}", stats.fuel_consumed);
        println!("  Uptime: {:.2}s", stats.start_time.elapsed().as_secs_f64());
        println!("  Last activity: {:.2}s ago", stats.last_activity.elapsed().as_secs_f64());
    }
    
    // Stop all plugins
    println!("\n--- Stopping All Plugins ---");
    manager.stop_all().await?;
    println!("✓ All plugins stopped");
    
    // Unload plugin
    println!("\n--- Unloading Plugin ---");
    manager.unload_plugin(&plugin_id).await?;
    println!("✓ Plugin unloaded");
    
    println!("\n=== Demo Complete ===");
    
    Ok(())
}

/// Print plugin statistics
async fn print_plugin_stats(plugin_instance: &sandboxing::sandbox_manager::PluginInstance) {
    let stats = plugin_instance.get_stats().await;
    println!("  Stats: calls={}, failed={}, fuel_consumed={}", 
        stats.total_calls, stats.failed_calls, stats.fuel_consumed);
}


