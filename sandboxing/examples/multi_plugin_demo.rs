//! Example demonstrating multiple plugins running in parallel with the SandboxManager

use anyhow::Result;
use sandboxing::sandbox_manager::{PluginConfig, SandboxManager};
use std::path::PathBuf;
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== Multi-Plugin Sandbox Manager Demo ===\n");
    
    // Create the sandbox manager
    let manager = SandboxManager::new()?;
    println!("✓ Sandbox manager created\n");
    
    // Load multiple plugin instances
    let plugin_configs = vec![
        PluginConfig {
            id: "plugin-1".to_string(),
            name: "Plugin Instance 1".to_string(),
            wasm_path: PathBuf::from("plugin/target/wasm32-wasip2/release/plugin.wasm"),
            policy_path: PathBuf::from("config/policy.yaml"),
            auto_restart: true,
            max_restart_attempts: 3,
        },
        PluginConfig {
            id: "plugin-2".to_string(),
            name: "Plugin Instance 2".to_string(),
            wasm_path: PathBuf::from("plugin/target/wasm32-wasip2/release/plugin.wasm"),
            policy_path: PathBuf::from("config/policy.yaml"),
            auto_restart: true,
            max_restart_attempts: 5,
        },
        PluginConfig {
            id: "plugin-3".to_string(),
            name: "Plugin Instance 3".to_string(),
            wasm_path: PathBuf::from("plugin/target/wasm32-wasip2/release/plugin.wasm"),
            policy_path: PathBuf::from("config/policy.yaml"),
            auto_restart: false,
            max_restart_attempts: 1,
        },
    ];
    
    // Load all plugins
    println!("Loading {} plugins...", plugin_configs.len());
    for config in plugin_configs {
        let plugin_id = manager.load_plugin(config.clone()).await?;
        println!("  ✓ Loaded: {} (ID: {})", config.name, plugin_id);
    }
    println!();
    
    // List all active plugins
    let plugin_list = manager.list_plugins().await;
    println!("Active plugins: {} total", plugin_list.len());
    for id in &plugin_list {
        println!("  - {}", id);
    }
    println!();
    
    // Check status of all plugins
    println!("=== Initial Status Check ===");
    let statuses = manager.get_all_statuses().await;
    for (id, status) in &statuses {
        println!("  {}: {:?}", id, status);
    }
    println!();
    
    // Perform health checks
    println!("=== Health Check ===");
    let health_results = manager.health_check_all().await;
    for (id, result) in &health_results {
        match result {
            Ok(healthy) => {
                println!("  {}: {}", id, if *healthy { "✓ Healthy" } else { "✗ Unhealthy" });
            }
            Err(e) => {
                println!("  {}: ✗ Error - {}", id, e);
            }
        }
    }
    println!();
    
    // Execute operations on each plugin in parallel
    println!("=== Executing Operations in Parallel ===");
    let mut handles = vec![];
    
    for plugin_id in &plugin_list {
        let manager_clone = manager.clone();
        let id = plugin_id.clone();
        
        let handle = tokio::spawn(async move {
            if let Ok(plugin_instance) = manager_clone.get_plugin(&id).await {
                let plugin = plugin_instance.get_plugin();
                let store = plugin_instance.get_store();
                
                // Execute a simple operation
                let mut store_guard = store.lock().await;
                let plugin_guard = plugin.lock().await;
                
                let result = plugin_guard
                    .example_plugin_policy()
                    .call_check_key(&mut *store_guard, r#"{"action": "allow"}"#, "action")
                    .await;
                
                match result {
                    Ok(res) => {
                        println!("  {}: ✓ Operation completed - {}", id, res);
                        plugin_instance.update_stats(true, 100).await;
                    }
                    Err(e) => {
                        println!("  {}: ✗ Operation failed - {}", id, e);
                        plugin_instance.update_stats(false, 0).await;
                    }
                }
            }
        });
        
        handles.push(handle);
    }
    
    // Wait for all operations to complete
    for handle in handles {
        let _ = handle.await;
    }
    println!();
    
    // Demonstrate lifecycle operations
    println!("=== Lifecycle Operations ===");
    
    // Pause plugin-1
    println!("Pausing plugin-1...");
    manager.pause_plugin("plugin-1").await?;
    println!("  ✓ plugin-1 paused");
    
    sleep(Duration::from_millis(500)).await;
    
    // Resume plugin-1
    println!("Resuming plugin-1...");
    manager.resume_plugin("plugin-1").await?;
    println!("  ✓ plugin-1 resumed");
    
    // Restart plugin-2
    println!("Restarting plugin-2...");
    manager.restart_plugin("plugin-2").await?;
    println!("  ✓ plugin-2 restarted");
    
    println!();
    
    // Check statuses after lifecycle operations
    println!("=== Status After Lifecycle Operations ===");
    let statuses = manager.get_all_statuses().await;
    for (id, status) in &statuses {
        println!("  {}: {:?}", id, status);
    }
    println!();
    
    // Get comprehensive statistics
    println!("=== Plugin Statistics ===");
    let all_stats = manager.get_all_stats().await;
    for (id, stats) in &all_stats {
        println!("\n{}:", id);
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
    println!();
    
    // Demonstrate selective unloading
    println!("=== Selective Plugin Unloading ===");
    println!("Unloading plugin-3...");
    manager.unload_plugin("plugin-3").await?;
    println!("  ✓ plugin-3 unloaded");
    
    let remaining = manager.list_plugins().await;
    println!("Remaining plugins: {:?}", remaining);
    println!();
    
    // Stop all remaining plugins
    println!("=== Stopping All Plugins ===");
    manager.stop_all().await?;
    println!("  ✓ All plugins stopped");
    
    // Final status check
    println!("\n=== Final Status ===");
    let final_statuses = manager.get_all_statuses().await;
    for (id, status) in &final_statuses {
        println!("  {}: {:?}", id, status);
    }
    
    println!("\n=== Demo Complete ===");
    
    Ok(())
}


