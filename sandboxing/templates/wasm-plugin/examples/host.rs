// WASM Plugin Host Example
//
// Demonstrates how to:
//   1. Load a WASM component plugin
//   2. Initialize it with configuration
//   3. Invoke hooks with payloads
//   4. Handle results (Allow, Deny, Modify)
//   5. Shutdown gracefully
//
// Run with: cargo run --example host --release

use anyhow::Result;
use wasmtime::component::*;
use wasmtime::{Config, Engine, Store};
use wasmtime_wasi::{WasiCtx, WasiCtxBuilder, WasiView};

// Generate bindings from the WIT file
wasmtime::component::bindgen!({
    path: "wit/world.wit",
    world: "plugin",
    async: true,
});

// Import the generated types
use exports::cpex::plugin::hooks::{HookRequest, HookResult};

// Host state that implements WasiView for WASI support
struct HostState {
    wasi: WasiCtx,
    table: ResourceTable,
}

impl WasiView for HostState {
    fn ctx(&mut self) -> &mut WasiCtx {
        &mut self.wasi
    }
    fn table(&mut self) -> &mut ResourceTable {
        &mut self.table
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== WASM Plugin Host Demo ===\n");

    // 1. Configure Wasmtime engine with component model support
    let mut config = Config::new();
    config.wasm_component_model(true);
    config.async_support(true);
    let engine = Engine::new(&config)?;

    // 2. Load the WASM component
    println!("Loading plugin.wasm...");
    let component = Component::from_file(&engine, "plugin.wasm")?;
    println!("✓ Loaded plugin.wasm\n");

    // 3. Create linker and add WASI support
    let mut linker = Linker::new(&engine);
    wasmtime_wasi::add_to_linker_async(&mut linker)?;

    // 4. Create store with WASI context
    let wasi = WasiCtxBuilder::new()
        .inherit_stdio()
        .inherit_env()
        .build();
    let mut store = Store::new(
        &engine,
        HostState {
            wasi,
            table: ResourceTable::new(),
        },
    );

    // 5. Instantiate the plugin
    println!("Instantiating plugin...");
    let plugin = Plugin::instantiate_async(&mut store, &component, &linker).await?;
    println!("✓ Plugin instantiated\n");

    // 6. Initialize the plugin with config
    println!("--- Initializing plugin ---");
    let config_json = r#"{}"#; // Empty config for this demo
    plugin
        .cpex_plugin_lifecycle()
        .call_init(&mut store, config_json)
        .await?
        .map_err(|e| anyhow::anyhow!("Init failed: {}", e))?;
    println!("✓ Plugin initialized\n");

    // 7. Test Scenario 1: Valid tool invocation
    println!("=== Scenario 1: Valid tool invocation ===");
    let hook_request = HookRequest {
        hook_name: "tool_pre_invoke".to_string(),
        payload_json: r#"{
            "tool_name": "get_user_data",
            "user": "alice",
            "arguments": "user_id=123"
        }"#
        .to_string(),
        extensions_json: "{}".to_string(),
        context_json: "{}".to_string(),
    };

    let result = plugin
        .cpex_plugin_hooks()
        .call_handle(&mut store, &hook_request)
        .await?;

    print_hook_result("tool_pre_invoke", &result);

    // 8. Test Scenario 2: Post-invoke hook
    println!("=== Scenario 2: Post-invoke hook ===");
    let post_hook_request = HookRequest {
        hook_name: "tool_post_invoke".to_string(),
        payload_json: r#"{
            "tool_name": "get_user_data",
            "user": "alice",
            "arguments": "user_id=123"
        }"#
        .to_string(),
        extensions_json: "{}".to_string(),
        context_json: "{}".to_string(),
    };

    let post_result = plugin
        .cpex_plugin_hooks()
        .call_handle(&mut store, &post_hook_request)
        .await?;

    print_hook_result("tool_post_invoke", &post_result);

    // 9. Test Scenario 3: Unknown hook (should pass through)
    println!("=== Scenario 3: Unknown hook (passthrough) ===");
    let unknown_hook_request = HookRequest {
        hook_name: "unknown_hook".to_string(),
        payload_json: r#"{"data": "test"}"#.to_string(),
        extensions_json: "{}".to_string(),
        context_json: "{}".to_string(),
    };

    let unknown_result = plugin
        .cpex_plugin_hooks()
        .call_handle(&mut store, &unknown_hook_request)
        .await?;

    print_hook_result("unknown_hook", &unknown_result);

    // 10. Test Scenario 4: Different user
    println!("=== Scenario 4: Different user (bob) ===");
    let bob_request = HookRequest {
        hook_name: "tool_pre_invoke".to_string(),
        payload_json: r#"{
            "tool_name": "list_departments",
            "user": "bob",
            "arguments": ""
        }"#
        .to_string(),
        extensions_json: "{}".to_string(),
        context_json: "{}".to_string(),
    };

    let bob_result = plugin
        .cpex_plugin_hooks()
        .call_handle(&mut store, &bob_request)
        .await?;

    print_hook_result("tool_pre_invoke (bob)", &bob_result);

    // 11. Shutdown the plugin
    println!("--- Shutting down plugin ---");
    plugin
        .cpex_plugin_lifecycle()
        .call_shutdown(&mut store)
        .await?;
    println!("✓ Plugin shutdown complete\n");

    println!("=== Demo complete ===");
    Ok(())
}

/// Helper function to print hook results in a readable format
fn print_hook_result(hook_name: &str, result: &HookResult) {
    match result {
        HookResult::Allow => {
            println!("  Hook '{}': ✓ ALLOW", hook_name);
            println!("  → Request allowed to proceed\n");
        }
        HookResult::Deny(violation) => {
            println!("  Hook '{}': ✗ DENY", hook_name);
            println!("  → Violation: {}", violation);
            // Try to parse as JSON for better display
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(violation) {
                if let Some(code) = v.get("code") {
                    println!("  → Code: {}", code);
                }
                if let Some(reason) = v.get("reason") {
                    println!("  → Reason: {}", reason);
                }
            }
            println!();
        }
        HookResult::Modify(modified_payload) => {
            println!("  Hook '{}': ⚡ MODIFY", hook_name);
            println!("  → Modified payload:");
            // Pretty print JSON if possible
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(modified_payload) {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&v).unwrap_or_else(|_| modified_payload.clone())
                );
            } else {
                println!("  {}", modified_payload);
            }
            println!();
        }
    }
}

// Made with Bob
