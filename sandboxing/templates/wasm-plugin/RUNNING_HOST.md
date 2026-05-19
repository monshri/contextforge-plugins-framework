# Running the WASM Plugin Host

This document explains how to invoke the `plugin.wasm` file from Rust.

## Quick Start

```bash
# Build the plugin and run the host example
make run-host

# Or run them separately:
make component          # Build plugin.wasm
cargo run --example host --release
```

## What Was Implemented

### 1. Host Example (`examples/host.rs`)

A complete Rust host application that demonstrates:

- **Loading** the WASM component using Wasmtime
- **Initializing** the plugin with configuration
- **Invoking hooks** with different payloads
- **Handling results** (Allow, Deny, Modify)
- **Graceful shutdown**

### 2. Key Components

#### Plugin Lifecycle
```rust
// Initialize with config
plugin.cpex_plugin_lifecycle()
    .call_init(&mut store, config_json).await?;

// Shutdown
plugin.cpex_plugin_lifecycle()
    .call_shutdown(&mut store).await?;
```

#### Hook Invocation
```rust
let request = HookRequest {
    hook_name: "tool_pre_invoke".to_string(),
    payload_json: r#"{"tool_name":"test","user":"alice","arguments":""}"#.to_string(),
    extensions_json: "{}".to_string(),
    context_json: "{}".to_string(),
};

let result = plugin.cpex_plugin_hooks()
    .call_handle(&mut store, &request).await?;
```

#### Result Handling
```rust
match result {
    HookResult::Allow => {
        // Request allowed to proceed
    }
    HookResult::Deny(violation) => {
        // Request denied with violation details
    }
    HookResult::Modify(modified_payload) => {
        // Request modified, use new payload
    }
}
```

## Test Scenarios

The host example runs 4 test scenarios:

1. **Valid tool invocation** - Tests basic hook execution
2. **Post-invoke hook** - Tests lifecycle hooks
3. **Unknown hook** - Tests passthrough behavior
4. **Different user** - Tests with different payload data

## Output Example

```
=== WASM Plugin Host Demo ===

Loading plugin.wasm...
✓ Loaded plugin.wasm

Instantiating plugin...
✓ Plugin instantiated

--- Initializing plugin ---
✓ Plugin initialized

=== Scenario 1: Valid tool invocation ===
  Hook 'tool_pre_invoke': ✓ ALLOW
  → Request allowed to proceed

=== Scenario 2: Post-invoke hook ===
  Hook 'tool_post_invoke': ✓ ALLOW
  → Request allowed to proceed

...

--- Shutting down plugin ---
✓ Plugin shutdown complete

=== Demo complete ===
```

## Architecture

```
┌─────────────────────────────────────────┐
│         Rust Host Application           │
│  (examples/host.rs)                     │
│                                         │
│  ┌─────────────────────────────────┐   │
│  │   Wasmtime Engine               │   │
│  │   - Component Model Support     │   │
│  │   - WASI Support                │   │
│  │   - Async Runtime               │   │
│  └─────────────────────────────────┘   │
│              ↓                          │
│  ┌─────────────────────────────────┐   │
│  │   WIT Bindings (Generated)      │   │
│  │   - Plugin interface            │   │
│  │   - Lifecycle methods           │   │
│  │   - Hook handlers               │   │
│  └─────────────────────────────────┘   │
│              ↓                          │
│  ┌─────────────────────────────────┐   │
│  │   plugin.wasm                   │   │
│  │   (Sandboxed WASM Component)    │   │
│  └─────────────────────────────────┘   │
└─────────────────────────────────────────┘
```

## Dependencies

The host requires these dependencies (already in `Cargo.toml`):

```toml
[dev-dependencies]
wasmtime = { version = "28", features = ["component-model", "async"] }
wasmtime-wasi = "28"
tokio = { version = "1", features = ["full"] }
anyhow = "1"
```

## Customization

### Adding New Test Scenarios

Edit `examples/host.rs` and add new test cases:

```rust
println!("=== My Custom Scenario ===");
let custom_request = HookRequest {
    hook_name: "my_hook".to_string(),
    payload_json: r#"{"custom": "data"}"#.to_string(),
    extensions_json: "{}".to_string(),
    context_json: "{}".to_string(),
};

let result = plugin.cpex_plugin_hooks()
    .call_handle(&mut store, &custom_request).await?;
print_hook_result("my_hook", &result);
```

### Passing Configuration

Modify the config JSON in the init call:

```rust
let config_json = r#"{
    "your_field": "value",
    "max_retries": 3
}"#;

plugin.cpex_plugin_lifecycle()
    .call_init(&mut store, config_json).await?;
```

## Integration with CPEX Framework

To integrate this WASM plugin loader into the CPEX framework:

1. Create a `WasmPluginFactory` that implements `PluginFactory`
2. Load the WASM component in the factory's `create()` method
3. Wrap the WASM plugin in a `HookHandler` implementation
4. Register with `PluginManager` like any other plugin

This allows WASM plugins to work alongside native Rust plugins in the same pipeline!

## Troubleshooting

### Build Issues

If the build fails with memory issues:
```bash
# Try without release optimization first
cargo build --example host
cargo run --example host
```

### Runtime Issues

If the plugin fails to load:
- Ensure `plugin.wasm` exists in the current directory
- Check that it was built with `make component`
- Verify the WIT interface matches between host and plugin

## Next Steps

1. **Modify the plugin** - Edit `src/lib.rs` to add custom logic
2. **Rebuild** - Run `make component` to rebuild the WASM
3. **Test** - Run `make run-host` to test your changes
4. **Deploy** - Copy `plugin.wasm` to your deployment location