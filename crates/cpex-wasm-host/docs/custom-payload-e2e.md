# Custom Payload End-to-End Flow

## Overview

The `wasm_custom_payload_demo` demonstrates dispatching a non-CMF payload (`ToolInvokePayload`) through the WASM plugin sandbox. Unlike the standard CMF path where the payload is a structured `MessagePayload`, custom payloads cross the WASM boundary as a JSON envelope (`Payload::Custom`).

## End-to-End Flow

### 1. Host defines a custom payload

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ToolInvokePayload {
    tool_name: String,
    user: String,
    arguments: String,
}
cpex_core::impl_plugin_payload_json!(ToolInvokePayload, "ToolInvokePayload");
```

The `impl_plugin_payload_json!` macro implements the `PluginPayload` trait with:
- `payload_type_name()` → returns `"ToolInvokePayload"`
- `to_json()` → serializes the struct to a JSON string via `serde_json`

### 2. Host invokes the hook

```rust
mgr.invoke_named::<ToolPreInvokeHook>("tool_pre_invoke", payload, ext, None).await;
```

The `PluginManager` resolves which plugins are registered for `tool_pre_invoke`, and for WASM plugins, calls `WasmBridgeHandler::invoke`.

### 3. WasmBridgeHandler converts native → WIT

In `cpex-wasm-host/src/conversions.rs`, `native_any_payload_to_wit()` runs:

```
payload.as_any().downcast_ref::<MessagePayload>()  →  fails (not a MessagePayload)
    ↓
payload.to_json()  →  Ok(r#"{"tool_name":"get_compensation","user":"alice","arguments":"{...}"}"#)
    ↓
Payload::Custom(CustomPayload {
    payload_type: "ToolInvokePayload",
    payload_json: "<the JSON above>",
})
```

Extensions and PluginContext are also converted to their WIT representations.

### 4. WASM sandbox invocation

`SandboxManager::invoke()` calls the WASM component's exported `handle-hook` function, passing:
- `payload: Payload::Custom { payload_type, payload_json }`
- `extensions: Extensions { ... }`
- `ctx: PluginContext { local_state, global_state }`

### 5. Inside the WASM plugin (`cpex-wasm-plugin/src/lib.rs`)

The plugin's `handle_hook` implementation:

```rust
match native_payload {
    NativePayload::Message(msg) => { /* CMF path */ }
    NativePayload::Custom(custom) => {
        // custom.payload_type == "ToolInvokePayload"
        // custom.payload_json == the serialized JSON
        // Currently: passes through unchanged (no-op stub)
        PluginResult {
            continue_processing: true,
            modified_payload: None,
            ...
        }
    }
}
```

The plugin receives the `NativeCustomPayload` and can:
- Match on `payload_type` to identify which hook fired
- Deserialize `payload_json` into the concrete struct: `custom.deserialize::<ToolInvokePayload>()?`
- Apply business logic (validate, modify, deny)
- Return a modified payload via `native_result_custom_to_wit()`

### 6. Result flows back to host

The WIT `PluginResult` is returned to the host, which calls `wit_result_to_native()` to convert it back to a `PipelineResult` that the `PluginManager` executor understands.

### Flow Diagram

```
Host (example)                    cpex-wasm-host                    WASM sandbox (cpex-wasm-plugin)
─────────────                    ──────────────                    ──────────────────────────────
                                                                   
ToolInvokePayload                                                  
       │                                                           
       ▼                                                           
invoke_named()                                                     
       │                                                           
       ▼                                                           
WasmBridgeHandler::invoke()                                        
       │                                                           
       ▼                                                           
native_any_payload_to_wit()                                        
  downcast fails → to_json()                                       
       │                                                           
       ▼                                                           
Payload::Custom {                                                  
  "ToolInvokePayload",                                             
  "{...json...}"                                                   
}                                                                  
       │                                                           
       ▼                                                           
SandboxManager::invoke() ──────────────────────────────────────▶  handle_hook(payload, ext, ctx)
                                                                          │
                                                                          ▼
                                                                   wit_payload_to_native()
                                                                          │
                                                                          ▼
                                                                   NativePayload::Custom(...)
                                                                          │
                                                                          ▼
                                                                   match → Custom arm (stub)
                                                                          │
                                                                          ▼
                                                                   PluginResult { continue: true }
                                                                          │
       ◀──────────────────────────────────────────────────────────────────┘
       │
       ▼
wit_result_to_native()
       │
       ▼
PipelineResult::allowed
```

---

## Gaps

### 1. `cpex-core` missing `payload_type_name()` and `to_json()` on PluginPayload trait

The WASM host (`native_any_payload_to_wit`) calls these methods on `&dyn PluginPayload`. They were removed from `cpex-core/src/hooks/payload.rs`. Without them, the host crate will not compile.

**Fix:** Add the two default methods and `impl_plugin_payload_json!` macro back to `cpex-core`'s `PluginPayload` trait. These are the minimum additions needed for custom payload WASM dispatch.

### 2. WASM plugin custom arm is a no-op

The `NativePayload::Custom` match arm in `cpex-wasm-plugin/src/lib.rs` passes through without processing. No business logic is applied to custom payloads — the plugin always returns `continue_processing: true` with no modifications.

**Fix:** Implement actual plugin logic for custom payload types, e.g.:
```rust
NativePayload::Custom(custom) => {
    if custom.payload_type == "ToolInvokePayload" {
        let tool: ToolInvokePayload = custom.deserialize().unwrap();
        // Apply validation/transformation logic...
    }
}
```

### 3. `native_result_custom_to_wit` is unused (`#[allow(dead_code)]`)

The helper for converting custom payload results back to WIT exists in `cpex-wasm-plugin/src/conversions.rs` but nothing calls it. When the custom arm implements real logic that modifies the payload, it needs to use this function to return the modified payload.

### 4. Capability tokens not propagated to WASM sandbox

The `header_injector` plugin uses `extensions.cow_copy()` and checks `labels_write_token` / `http_write_token`. These write-guard tokens are populated by the native cpex-core executor but are NOT set when Extensions are converted to WIT types. Any plugin logic that relies on write tokens will silently skip its mutations inside WASM.

**Fix:** The host needs to populate capability tokens based on the plugin's declared `capabilities` in config, or the WASM plugin needs an alternative write path that doesn't depend on tokens.

### 5. Config hook name mismatch

The existing `config.yaml` registers the WASM plugin for `cmf.tool_pre_invoke`. The custom payload demo uses `tool_pre_invoke` (without the `cmf.` prefix). The example works around this by string-replacing the config at runtime, which is fragile.

**Fix:** Either add a separate config file for the custom payload demo, or register the plugin under both hook names.
