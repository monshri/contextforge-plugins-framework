# Sandbox Manager

A comprehensive plugin sandbox manager for managing multiple WebAssembly plugins with lifecycle control, resource monitoring, and security policies.

## Overview

The Sandbox Manager provides a robust framework for:
- **Multi-plugin management**: Load, run, and manage multiple plugin instances simultaneously
- **Lifecycle control**: Start, stop, pause, resume, and restart plugins
- **Resource monitoring**: Track fuel consumption, memory usage, and execution statistics
- **Health checks**: Monitor plugin health and automatically handle failures
- **Security policies**: Enforce resource limits, network access controls, and filesystem restrictions

## Architecture

### Core Components

#### 1. `PluginInstance`
Manages a single plugin running in a sandboxed environment.

**Key Features:**
- Isolated WASM runtime with configurable resource limits
- Status tracking (Initializing, Running, Paused, Stopped, Failed)
- Statistics collection (calls, failures, fuel consumption, uptime)
- Lifecycle operations (stop, pause, resume, restart)
- Health monitoring

**Example:**
```rust
let config = PluginConfig {
    id: "my-plugin".to_string(),
    name: "My Plugin".to_string(),
    wasm_path: PathBuf::from("plugin.wasm"),
    policy_path: PathBuf::from("policy.yaml"),
    auto_restart: true,
    max_restart_attempts: 3,
};

let instance = PluginInstance::new(config).await?;
```

#### 2. `SandboxManager`
Orchestrates multiple plugin instances with centralized management.

**Key Features:**
- Load and unload plugins dynamically
- Query plugin status and statistics
- Perform bulk operations (stop all, health check all)
- Thread-safe concurrent access
- Automatic resource cleanup

**Example:**
```rust
let manager = SandboxManager::new()?;

// Load a plugin
let plugin_id = manager.load_plugin(config).await?;

// Get plugin instance
let plugin = manager.get_plugin(&plugin_id).await?;

// Perform operations
manager.pause_plugin(&plugin_id).await?;
manager.resume_plugin(&plugin_id).await?;

// Cleanup
manager.unload_plugin(&plugin_id).await?;
```

#### 3. `PluginConfig`
Configuration for plugin instances.

**Fields:**
- `id`: Unique identifier for the plugin
- `name`: Human-readable name
- `wasm_path`: Path to the WASM component
- `policy_path`: Path to the security policy file
- `auto_restart`: Enable automatic restart on failure
- `max_restart_attempts`: Maximum number of restart attempts

#### 4. `PluginStatus`
Represents the current state of a plugin.

**States:**
- `Initializing`: Plugin is being loaded and initialized
- `Running`: Plugin is active and healthy
- `Paused`: Plugin is temporarily suspended
- `Stopped`: Plugin has been stopped
- `Failed(String)`: Plugin encountered an error

#### 5. `PluginStats`
Statistics and metrics for a plugin instance.

**Metrics:**
- `start_time`: When the plugin was started
- `last_activity`: Last time the plugin was used
- `total_calls`: Total number of function calls
- `failed_calls`: Number of failed calls
- `fuel_consumed`: Total fuel (CPU cycles) consumed
- `memory_used`: Memory usage in bytes

## Usage Examples

### Basic Usage

```rust
use sandboxing::sandbox_manager::{PluginConfig, SandboxManager};
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<()> {
    // Create manager
    let manager = SandboxManager::new()?;
    
    // Configure plugin
    let config = PluginConfig {
        id: "example-plugin".to_string(),
        name: "Example Plugin".to_string(),
        wasm_path: PathBuf::from("plugin.wasm"),
        policy_path: PathBuf::from("policy.yaml"),
        auto_restart: true,
        max_restart_attempts: 3,
    };
    
    // Load plugin
    let plugin_id = manager.load_plugin(config).await?;
    
    // Get plugin instance
    let plugin_instance = manager.get_plugin(&plugin_id).await?;
    
    // Execute plugin functions
    let plugin = plugin_instance.get_plugin();
    let store = plugin_instance.get_store();
    
    let mut store_guard = store.lock().await;
    let plugin_guard = plugin.lock().await;
    
    let result = plugin_guard
        .example_plugin_policy()
        .call_check_key(&mut *store_guard, r#"{"action": "allow"}"#, "action")
        .await?;
    
    println!("Result: {}", result);
    
    // Update statistics
    plugin_instance.update_stats(true, 100).await;
    
    // Cleanup
    manager.unload_plugin(&plugin_id).await?;
    
    Ok(())
}
```

### Managing Multiple Plugins

```rust
// Load multiple plugins
let configs = vec![
    PluginConfig { id: "plugin-1".to_string(), /* ... */ },
    PluginConfig { id: "plugin-2".to_string(), /* ... */ },
    PluginConfig { id: "plugin-3".to_string(), /* ... */ },
];

for config in configs {
    manager.load_plugin(config).await?;
}

// List all plugins
let plugins = manager.list_plugins().await;
println!("Active plugins: {:?}", plugins);

// Check status of all plugins
let statuses = manager.get_all_statuses().await;
for (id, status) in statuses {
    println!("{}: {:?}", id, status);
}

// Health check all plugins
let health_results = manager.health_check_all().await;
for (id, result) in health_results {
    match result {
        Ok(healthy) => println!("{}: {}", id, if healthy { "Healthy" } else { "Unhealthy" }),
        Err(e) => println!("{}: Error - {}", id, e),
    }
}

// Get statistics for all plugins
let all_stats = manager.get_all_stats().await;
for (id, stats) in all_stats {
    println!("{}: {} calls, {} failed", id, stats.total_calls, stats.failed_calls);
}

// Stop all plugins
manager.stop_all().await?;
```

### Lifecycle Management

```rust
let plugin_id = "my-plugin";

// Pause a plugin
manager.pause_plugin(plugin_id).await?;
println!("Plugin paused");

// Resume a plugin
manager.resume_plugin(plugin_id).await?;
println!("Plugin resumed");

// Restart a plugin
manager.restart_plugin(plugin_id).await?;
println!("Plugin restarted");

// Stop and unload
manager.unload_plugin(plugin_id).await?;
println!("Plugin unloaded");
```

### Parallel Execution

```rust
use tokio::task;

let plugin_ids = manager.list_plugins().await;
let mut handles = vec![];

for plugin_id in plugin_ids {
    let manager_clone = manager.clone();
    let id = plugin_id.clone();
    
    let handle = task::spawn(async move {
        if let Ok(plugin_instance) = manager_clone.get_plugin(&id).await {
            // Execute plugin operations
            let plugin = plugin_instance.get_plugin();
            let store = plugin_instance.get_store();
            
            // ... perform operations ...
            
            plugin_instance.update_stats(true, 100).await;
        }
    });
    
    handles.push(handle);
}

// Wait for all operations to complete
for handle in handles {
    handle.await?;
}
```

## Security Features

### Resource Limits

Configured via policy.yaml:

```yaml
plugin:
  sandbox:
    policy:
      resource_limits:
        max_memory_bytes: 104857600  # 100 MB
        max_fuel: 1000000            # CPU cycles
        cpu_timeout_ms: 5000         # 5 seconds
        wall_clock_timeout_ms: 10000 # 10 seconds
```

### Network Access Control

```yaml
allowed_hosts:
  - "httpbin.org"
  - "api.example.com"
  - "127.0.0.1"
```

### Filesystem Restrictions

```yaml
preopened_dirs:
  - guest_path: "/data"
    host_path: "./data"
    readonly: false
```

### Environment Variable Filtering

```yaml
allowed_env_vars:
  - "PATH"
  - "USER"
  - "HOME"
```

## Monitoring and Observability

### Statistics Collection

```rust
let stats = plugin_instance.get_stats().await;

println!("Total calls: {}", stats.total_calls);
println!("Failed calls: {}", stats.failed_calls);
println!("Success rate: {:.1}%", 
    ((stats.total_calls - stats.failed_calls) as f64 / stats.total_calls as f64) * 100.0
);
println!("Fuel consumed: {}", stats.fuel_consumed);
println!("Uptime: {:.2}s", stats.start_time.elapsed().as_secs_f64());
println!("Last activity: {:.2}s ago", stats.last_activity.elapsed().as_secs_f64());
```

### Health Monitoring

```rust
// Single plugin health check
let is_healthy = plugin_instance.health_check().await?;

// All plugins health check
let health_results = manager.health_check_all().await;
for (id, result) in health_results {
    match result {
        Ok(true) => println!("{}: Healthy", id),
        Ok(false) => println!("{}: Unhealthy", id),
        Err(e) => println!("{}: Error - {}", id, e),
    }
}
```

## Running Examples

### Basic Example
```bash
cd sandboxing
cargo run
```

### Multi-Plugin Demo
```bash
cd sandboxing
cargo run --example multi_plugin_demo
```

## API Reference

### SandboxManager

- `new() -> Result<Self>` - Create a new sandbox manager
- `load_plugin(config: PluginConfig) -> Result<String>` - Load and start a plugin
- `unload_plugin(plugin_id: &str) -> Result<()>` - Unload and stop a plugin
- `get_plugin(plugin_id: &str) -> Result<Arc<PluginInstance>>` - Get a plugin instance
- `list_plugins() -> Vec<String>` - List all plugin IDs
- `get_all_statuses() -> HashMap<String, PluginStatus>` - Get status of all plugins
- `stop_all() -> Result<()>` - Stop all plugins
- `restart_plugin(plugin_id: &str) -> Result<()>` - Restart a specific plugin
- `health_check_all() -> HashMap<String, Result<bool>>` - Health check all plugins
- `get_all_stats() -> HashMap<String, PluginStats>` - Get statistics for all plugins
- `pause_plugin(plugin_id: &str) -> Result<()>` - Pause a plugin
- `resume_plugin(plugin_id: &str) -> Result<()>` - Resume a plugin

### PluginInstance

- `new(config: PluginConfig) -> Result<Self>` - Create a new plugin instance
- `get_status() -> PluginStatus` - Get current status
- `get_stats() -> PluginStats` - Get current statistics
- `stop() -> Result<()>` - Stop the plugin
- `pause() -> Result<()>` - Pause the plugin
- `resume() -> Result<()>` - Resume the plugin
- `restart() -> Result<()>` - Restart the plugin
- `health_check() -> Result<bool>` - Check if plugin is healthy
- `update_stats(success: bool, fuel_consumed: u64)` - Update statistics
- `get_plugin() -> Arc<Mutex<Plugin>>` - Get the plugin for executing calls
- `get_store() -> Arc<Mutex<Store<MyState>>>` - Get the store for executing calls

## Best Practices

1. **Resource Limits**: Always configure appropriate resource limits for your use case
2. **Error Handling**: Handle plugin failures gracefully with auto-restart or fallback mechanisms
3. **Monitoring**: Regularly check plugin health and statistics
4. **Cleanup**: Always unload plugins when done to free resources
5. **Security**: Use strict allowlists for network access and filesystem operations
6. **Concurrency**: Use the manager's clone capability for parallel operations
7. **Statistics**: Track and analyze plugin statistics for performance optimization

## Troubleshooting

### Plugin Fails to Load
- Check that the WASM file path is correct
- Verify the policy file exists and is valid YAML
- Ensure resource limits are not too restrictive

### Plugin Crashes
- Check fuel limits (may need to increase)
- Review memory limits
- Check timeout settings
- Enable auto-restart for resilience

### Network Requests Fail
- Verify the host is in the allowed_hosts list
- Check network connectivity
- Review HTTP request format

### High Resource Usage
- Monitor fuel consumption statistics
- Adjust resource limits in policy
- Consider implementing rate limiting
- Use pause/resume for idle plugins

## License

See the main project LICENSE file.