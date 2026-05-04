//! Sandbox Manager for managing multiple plugin instances with lifecycle control.
//!
//! This module provides:
//! - PluginInstance: Manages a single plugin in a sandbox
//! - SandboxManager: Manages multiple plugin instances
//! - Lifecycle operations: start, stop, restart, health checks
//! - Resource monitoring and limits enforcement

use anyhow::{Context, Result, anyhow};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{RwLock, Mutex};
use wasmtime::component::{Component, Linker, ResourceTable};
use wasmtime::{Config, Engine, Store};
use wasmtime_wasi_http::WasiHttpCtx;

use crate::policy_loader::{build_wasi_ctx, load_policy, PolicyConfig};
use crate::{MyState, Plugin};

/// Status of a plugin instance
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PluginStatus {
    /// Plugin is initializing
    Initializing,
    /// Plugin is running and healthy
    Running,
    /// Plugin is paused
    Paused,
    /// Plugin has stopped
    Stopped,
    /// Plugin encountered an error
    Failed(String),
}

/// Statistics for a plugin instance
#[derive(Debug, Clone)]
pub struct PluginStats {
    pub start_time: Instant,
    pub last_activity: Instant,
    pub total_calls: u64,
    pub failed_calls: u64,
    pub fuel_consumed: u64,
    pub memory_used: u64,
}

impl Default for PluginStats {
    fn default() -> Self {
        let now = Instant::now();
        Self {
            start_time: now,
            last_activity: now,
            total_calls: 0,
            failed_calls: 0,
            fuel_consumed: 0,
            memory_used: 0,
        }
    }
}

/// Configuration for a plugin instance
#[derive(Debug, Clone)]
pub struct PluginConfig {
    pub id: String,
    pub name: String,
    pub wasm_path: PathBuf,
    pub policy_path: PathBuf,
    pub auto_restart: bool,
    pub max_restart_attempts: u32,
}

/// A single plugin instance running in a sandbox
pub struct PluginInstance {
    pub config: PluginConfig,
    pub status: Arc<RwLock<PluginStatus>>,
    pub stats: Arc<RwLock<PluginStats>>,
    engine: Engine,
    linker: Arc<Linker<MyState>>,
    store: Arc<Mutex<Store<MyState>>>,
    plugin: Arc<Mutex<Plugin>>,
    policy: PolicyConfig,
    restart_count: Arc<Mutex<u32>>,
}

impl PluginInstance {
    /// Create a new plugin instance
    pub async fn new(config: PluginConfig) -> Result<Self> {
        let status = Arc::new(RwLock::new(PluginStatus::Initializing));
        
        // Load policy
        let policy = load_policy(&config.policy_path.to_string_lossy())
            .context("Failed to load policy")?;
        let resource_limits = policy.plugin.sandbox.policy.resource_limits.clone();
        
        // Configure engine
        let mut engine_config = Config::new();
        engine_config.wasm_component_model(true);
        engine_config.async_support(true);
        
        // Enable fuel metering for CPU limits
        if resource_limits.max_fuel.is_some() || resource_limits.cpu_timeout_ms.is_some() {
            engine_config.consume_fuel(true);
        }
        
        // Enable epoch interruption for timeouts
        if resource_limits.wall_clock_timeout_ms.is_some() {
            engine_config.epoch_interruption(true);
        }
        
        let engine = Engine::new(&engine_config)?;
        
        // Create linker
        let mut linker: Linker<MyState> = Linker::new(&engine);
        wasmtime_wasi::add_to_linker_async(&mut linker)
            .context("Failed to add WASI to linker")?;
        wasmtime_wasi_http::add_only_http_to_linker_async(&mut linker)
            .context("Failed to add WASI HTTP to linker")?;
        
        // Build WASI context
        let wasi = build_wasi_ctx(&policy)?;
        let allowed_hosts = policy.plugin.sandbox.policy.allowed_hosts.clone();
        
        let state = MyState {
            wasi,
            http: WasiHttpCtx::new(),
            table: ResourceTable::new(),
            allowed_hosts,
            resource_limits: resource_limits.clone(),
            start_time: Instant::now(),
        };
        
        let mut store = Store::new(&engine, state);
        
        // Set fuel limit if configured
        if let Some(fuel) = resource_limits.max_fuel {
            store.set_fuel(fuel).context("Failed to set fuel limit")?;
        }
        
        // Set epoch deadline for timeout interruption
        if resource_limits.wall_clock_timeout_ms.is_some() {
            store.set_epoch_deadline(1);
        }
        
        // Start epoch ticker for wall clock timeout
        if let Some(timeout_ms) = resource_limits.wall_clock_timeout_ms {
            let engine_clone = engine.clone();
            tokio::spawn(async move {
                tokio::time::sleep(Duration::from_millis(timeout_ms)).await;
                engine_clone.increment_epoch();
            });
        }
        
        // Load component
        let component = Component::from_file(&engine, &config.wasm_path)
            .context("Failed to load WASM component")?;
        
        let plugin = Plugin::instantiate_async(&mut store, &component, &linker).await?;
        
        *status.write().await = PluginStatus::Running;
        
        Ok(Self {
            config,
            status,
            stats: Arc::new(RwLock::new(PluginStats::default())),
            engine,
            linker: Arc::new(linker),
            store: Arc::new(Mutex::new(store)),
            plugin: Arc::new(Mutex::new(plugin)),
            policy,
            restart_count: Arc::new(Mutex::new(0)),
        })
    }
    
    /// Get current status
    pub async fn get_status(&self) -> PluginStatus {
        self.status.read().await.clone()
    }
    
    /// Get current statistics
    pub async fn get_stats(&self) -> PluginStats {
        self.stats.read().await.clone()
    }
    
    /// Stop the plugin instance
    pub async fn stop(&self) -> Result<()> {
        let mut status = self.status.write().await;
        *status = PluginStatus::Stopped;
        Ok(())
    }
    
    /// Pause the plugin instance
    pub async fn pause(&self) -> Result<()> {
        let mut status = self.status.write().await;
        if *status == PluginStatus::Running {
            *status = PluginStatus::Paused;
            Ok(())
        } else {
            Err(anyhow!("Cannot pause plugin in {:?} state", *status))
        }
    }
    
    /// Resume the plugin instance
    pub async fn resume(&self) -> Result<()> {
        let mut status = self.status.write().await;
        if *status == PluginStatus::Paused {
            *status = PluginStatus::Running;
            Ok(())
        } else {
            Err(anyhow!("Cannot resume plugin in {:?} state", *status))
        }
    }
    
    /// Restart the plugin instance
    pub async fn restart(&self) -> Result<()> {
        let mut restart_count = self.restart_count.lock().await;
        
        if *restart_count >= self.config.max_restart_attempts {
            return Err(anyhow!(
                "Max restart attempts ({}) reached",
                self.config.max_restart_attempts
            ));
        }
        
        *restart_count += 1;
        
        // Stop current instance
        self.stop().await?;
        
        // Reset status to initializing
        *self.status.write().await = PluginStatus::Initializing;
        
        // Note: Full restart would require recreating the entire instance
        // This is a simplified version that just updates status
        *self.status.write().await = PluginStatus::Running;
        
        Ok(())
    }
    
    /// Check if plugin is healthy
    pub async fn health_check(&self) -> Result<bool> {
        let status = self.status.read().await;
        match *status {
            PluginStatus::Running => Ok(true),
            PluginStatus::Failed(ref err) => {
                Err(anyhow!("Plugin health check failed: {}", err))
            }
            _ => Ok(false),
        }
    }
    
    /// Update statistics after a call
    pub async fn update_stats(&self, success: bool, fuel_consumed: u64) {
        let mut stats = self.stats.write().await;
        stats.total_calls += 1;
        if !success {
            stats.failed_calls += 1;
        }
        stats.fuel_consumed += fuel_consumed;
        stats.last_activity = Instant::now();
    }
    
    /// Get the plugin for executing calls
    pub fn get_plugin(&self) -> Arc<Mutex<Plugin>> {
        self.plugin.clone()
    }
    
    /// Get the store for executing calls
    pub fn get_store(&self) -> Arc<Mutex<Store<MyState>>> {
        self.store.clone()
    }
}

/// Manager for multiple plugin instances
#[derive(Clone)]
pub struct SandboxManager {
    plugins: Arc<RwLock<HashMap<String, Arc<PluginInstance>>>>,
    engine: Engine,
}

impl SandboxManager {
    /// Create a new sandbox manager
    pub fn new() -> Result<Self> {
        let mut config = Config::new();
        config.wasm_component_model(true);
        config.async_support(true);
        
        let engine = Engine::new(&config)?;
        
        Ok(Self {
            plugins: Arc::new(RwLock::new(HashMap::new())),
            engine,
        })
    }
    
    /// Load and start a new plugin
    pub async fn load_plugin(&self, config: PluginConfig) -> Result<String> {
        let plugin_id = config.id.clone();
        
        // Check if plugin already exists
        {
            let plugins = self.plugins.read().await;
            if plugins.contains_key(&plugin_id) {
                return Err(anyhow!("Plugin with ID '{}' already exists", plugin_id));
            }
        }
        
        // Create new plugin instance
        let instance = PluginInstance::new(config).await?;
        
        // Add to manager
        let mut plugins = self.plugins.write().await;
        plugins.insert(plugin_id.clone(), Arc::new(instance));
        
        Ok(plugin_id)
    }
    
    /// Unload and stop a plugin
    pub async fn unload_plugin(&self, plugin_id: &str) -> Result<()> {
        let mut plugins = self.plugins.write().await;
        
        if let Some(plugin) = plugins.remove(plugin_id) {
            plugin.stop().await?;
            Ok(())
        } else {
            Err(anyhow!("Plugin '{}' not found", plugin_id))
        }
    }
    
    /// Get a plugin instance
    pub async fn get_plugin(&self, plugin_id: &str) -> Result<Arc<PluginInstance>> {
        let plugins = self.plugins.read().await;
        plugins
            .get(plugin_id)
            .cloned()
            .ok_or_else(|| anyhow!("Plugin '{}' not found", plugin_id))
    }
    
    /// List all plugin IDs
    pub async fn list_plugins(&self) -> Vec<String> {
        let plugins = self.plugins.read().await;
        plugins.keys().cloned().collect()
    }
    
    /// Get status of all plugins
    pub async fn get_all_statuses(&self) -> HashMap<String, PluginStatus> {
        let plugins = self.plugins.read().await;
        let mut statuses = HashMap::new();
        
        for (id, plugin) in plugins.iter() {
            statuses.insert(id.clone(), plugin.get_status().await);
        }
        
        statuses
    }
    
    /// Stop all plugins
    pub async fn stop_all(&self) -> Result<()> {
        let plugins = self.plugins.read().await;
        
        for plugin in plugins.values() {
            plugin.stop().await?;
        }
        
        Ok(())
    }
    
    /// Restart a specific plugin
    pub async fn restart_plugin(&self, plugin_id: &str) -> Result<()> {
        let plugin = self.get_plugin(plugin_id).await?;
        plugin.restart().await
    }
    
    /// Perform health checks on all plugins
    pub async fn health_check_all(&self) -> HashMap<String, Result<bool>> {
        let plugins = self.plugins.read().await;
        let mut results = HashMap::new();
        
        for (id, plugin) in plugins.iter() {
            results.insert(id.clone(), plugin.health_check().await);
        }
        
        results
    }
    
    /// Get statistics for all plugins
    pub async fn get_all_stats(&self) -> HashMap<String, PluginStats> {
        let plugins = self.plugins.read().await;
        let mut stats = HashMap::new();
        
        for (id, plugin) in plugins.iter() {
            stats.insert(id.clone(), plugin.get_stats().await);
        }
        
        stats
    }
    
    /// Pause a plugin
    pub async fn pause_plugin(&self, plugin_id: &str) -> Result<()> {
        let plugin = self.get_plugin(plugin_id).await?;
        plugin.pause().await
    }
    
    /// Resume a plugin
    pub async fn resume_plugin(&self, plugin_id: &str) -> Result<()> {
        let plugin = self.get_plugin(plugin_id).await?;
        plugin.resume().await
    }
}

impl Default for SandboxManager {
    fn default() -> Self {
        Self::new().expect("Failed to create default SandboxManager")
    }
}


