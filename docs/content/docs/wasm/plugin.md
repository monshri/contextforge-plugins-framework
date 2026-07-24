---
title: "Plugin SDK"
weight: 20
---

# cpex-wasm-plugin

The plugin SDK provides macros and conversion functions for authoring WASM plugins. Plugin authors implement `HookHandler<H>` on their type (identical to native plugins) and call a single macro — all WIT glue is generated automatically.

## Core API

### `register_wasm_plugin!`

The main macro generates a complete WIT `Guest` implementation:

```rust
use cpex_wasm_plugin::register_wasm_plugin;

struct MyPlugin;

// Implement HookHandler for each hook type (same as native plugins)
impl HookHandler<CmfHook> for MyPlugin {
    fn handle(&self, payload: &MessagePayload, ctx: &PluginContext) -> PluginResult {
        // your logic here
        PluginResult::allow()
    }
}

// Register — generates all WIT glue
register_wasm_plugin!(MyPlugin, [CmfHook, IdentityHook]);
```

**What the macro generates:**

1. Receives WIT types from the host (hook-name, payload, extensions, ctx)
2. Converts WIT → cpex-core native types
3. Routes to the matching `HookHandler<H>` based on the payload's concrete type
4. Converts `PluginResult` → WIT `HookResult` (with context writeback)

**Dispatch rules:**

- A CMF payload routes to the hook whose `Payload` is `MessagePayload`
- A custom payload routes to the hook whose `Payload` matches the WIT type discriminator (e.g. `"cpex.identity"` → `HookHandler<IdentityHook>`)
- Payloads that no listed hook handles return `allow()` (same as a native plugin not registered for that hook)
- A payload that names a listed type but fails to decode returns a deny violation (silently allowing on decode failure would skip the check)

### Structured Logging

Plugins should use `host_log` or the `cpex_log!` macro instead of `eprintln!` for production logging. Messages are routed to the host's tracing infrastructure with the plugin name attached.

```rust
use cpex_wasm_plugin::{host_log, LogLevel, cpex_log};

// Direct API
host_log(LogLevel::Info, "plugin initialized");

// Convenience macro with format args
cpex_log!(info, "processed {} items in {}ms", count, elapsed);
cpex_log!(warn, "payload field missing, using default");
cpex_log!(error, "authorization check failed for user {}", user_id);
```

**Log levels:** `Trace`, `Debug`, `Info`, `Warn`, `Error`

### Conversion Functions

For advanced use cases, the SDK exposes conversion functions directly:

| Function | Direction |
|----------|-----------|
| `wit_payload_to_native` | WIT `HookPayload` → cpex-core native payload |
| `wit_context_to_native` | WIT `PluginContext` → native context |
| `wit_extensions_to_native` | WIT `Extensions` → native extension map |
| `native_payload_to_wit` | Native payload → WIT `HookPayload` |
| `native_result_to_hook_result` | `PluginResult` → WIT `HookResult` |

## Building a Plugin

### Prerequisites

- Rust toolchain with `wasm32-wasip2` target
- `cargo-component` for Component Model compilation

```bash
rustup target add wasm32-wasip2
cargo install cargo-component
```

### Project Structure

```
my-plugin/
├── Cargo.toml
├── src/
│   └── lib.rs
└── wit/
    └── (symlink or copy of cpex plugin WIT)
```

### Cargo.toml

```toml
[package]
name = "my-plugin"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
cpex-core = { path = "../../cpex-core" }
cpex-wasm-plugin = { path = "../../cpex-wasm-plugin" }
wit-bindgen = "0.41"
```

### Build

```bash
cargo component build --release
# Output: target/wasm32-wasip2/release/my_plugin.wasm
```

## Example Plugins

The SDK ships with several reference plugins in `src/plugins/`:

| Plugin | Purpose |
|--------|---------|
| `noop` | Minimal plugin — allows everything (template for new plugins) |
| `pii_guard` | Scans payloads for PII patterns and redacts or blocks |
| `header_injector` | Injects headers into outbound requests based on context |
| `identity_checker` | Validates caller identity against an allow-list |
| `token_attenuator` | Attenuates delegation tokens before downstream calls |
| `audit_logger` | Logs hook invocations to the host's audit trail |
| `remote_authz` | Calls an external authorization service (demonstrates network access) |
| `tool_invoke_checker` | Validates tool invocation arguments against schema |
| `compute_bench` | CPU-intensive workload for benchmarking sandbox limits |
| `fs_test` | Exercises filesystem sandbox capabilities |
| `net_test` | Exercises network sandbox capabilities |
| `env_test` | Exercises environment variable sandbox capabilities |

## Lifecycle

1. The host loads the `.wasm` file and instantiates it in a sandboxed store
2. On each hook invocation, the host calls `handle-hook` with WIT-typed arguments
3. The `register_wasm_plugin!` macro dispatches to the correct `HookHandler` impl
4. The result (allow/deny/modify) is converted back to WIT types and returned to the host
5. The host converts the WIT result back to cpex-core types and continues the pipeline
