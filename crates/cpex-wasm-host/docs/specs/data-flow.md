# WASM Plugin Sandboxing — Data Flow Specification

## Architecture Overview

There are **4 layers** and **3 boundaries** the data crosses:

```
┌─────────────────────────────────────────────────────────────────┐
│  cpex-core (PluginManager / Executor)                           │
│  Native Rust types: MessagePayload, Extensions, PluginContext   │
└──────────────────────────────┬──────────────────────────────────┘
                               │ BOUNDARY 1: Type erasure (dyn PluginPayload → MessagePayload)
                               ▼
┌─────────────────────────────────────────────────────────────────┐
│  cpex-wasm-host (WasmBridgeHandler / factory.rs)                │
│  Converts Native ↔ WIT types via conversions.rs                 │
└──────────────────────────────┬──────────────────────────────────┘
                               │ BOUNDARY 2: WASM Component Model ABI
                               │ (wasmtime calls `handle-hook` across linear memory)
                               ▼
┌─────────────────────────────────────────────────────────────────┐
│  cpex-wasm-plugin (Guest impl / lib.rs)                         │
│  Receives WIT types, converts to cpex-payload native types      │
└──────────────────────────────┬──────────────────────────────────┘
                               │ BOUNDARY 3: Rust function call (pure in-process)
                               ▼
┌─────────────────────────────────────────────────────────────────┐
│  cpex-payload (plugin logic, e.g. identity_checker)             │
│  Pure Rust — no WASM/FFI awareness                              │
└─────────────────────────────────────────────────────────────────┘
```

## Step-by-Step Data Flow

### 1. Entry: PluginManager dispatches a hook

The `cpex-core` executor calls `AnyHookHandler::invoke()` with:
- `&dyn PluginPayload` (type-erased `MessagePayload`)
- `&Extensions` (security, HTTP headers, meta, request context)
- `&mut PluginContext` (local + global state as `HashMap<String, Value>`)

### 2. Host-side conversion (Boundary 1 → 2)

In `factory.rs`, the `WasmBridgeHandler`:

1. **Downcasts** `dyn PluginPayload` → `&MessagePayload`
2. **Converts Native → WIT** using `conversions.rs`:
   - `native_payload_to_wit()` — maps Rust enums/structs → WIT records/variants
   - `native_extensions_to_wit()` — `HashMap` → `list<tuple<string,string>>`, `HashSet` → `list<string>`
   - `native_context_to_wit()` — serializes `HashMap<String,Value>` → JSON strings
   - Complex fields like `ToolCall.arguments` (a `HashMap<String,Value>`) become **JSON strings** since WIT doesn't support nested maps
3. **Invokes the sandbox**: `sandbox_manager.invoke(wit_payload, wit_extensions, wit_ctx)`

### 3. WASM Component Model boundary (Boundary 2)

The `SandboxManager` calls:
```rust
plugin.call_handle_hook(&mut store, &payload, &extensions, &ctx)
```

This is wasmtime's Component Model ABI:
- WIT records are **decomposed into flat values** pushed onto the WASM stack or passed through linear memory for larger types
- The WIT definition (`world.wit`) is the **contract** — identical on both host and guest sides
- wasmtime handles all memory layout, lifting/lowering of types automatically
- No manual serialization across this boundary — it's the Component Model's canonical ABI

### 4. Guest-side reception (Boundary 2 → 3)

Inside the WASM module, `cpex-wasm-plugin/src/lib.rs`:
1. `wit_bindgen::generate!()` creates Rust types matching the WIT definition and a `Guest` trait
2. The `Plugin` struct implements `Guest::handle_hook(payload, extensions, ctx) -> PluginResult`
3. **Converts WIT → cpex-payload native types** using the plugin-side `conversions.rs`:
   - `wit_payload_to_native()` — WIT variants → cpex-payload enums
   - `wit_extensions_to_native()` — `list<tuple>` → `HashMap`, `list<string>` → `HashSet`, wraps in `Arc`
   - `wit_context_to_native()` — deserializes JSON strings → `HashMap<String, Value>`

### 5. Plugin logic execution (Boundary 3)

The actual plugin function (e.g., `identity_checker::identity_check`) receives pure cpex-payload types and returns `PluginResult<MessagePayload>`. It has no awareness of WASM.

### 6. Return path (all boundaries in reverse)

1. **Plugin → Guest**: `native_result_to_wit()` converts `PluginResult` back to WIT types (re-serializes any modified payload/extensions, maps violations to WIT records)
2. **Guest → Host**: wasmtime's Component Model ABI returns the WIT `plugin-result` record across the WASM boundary
3. **Host → Executor**: `wit_result_to_native()` converts back to `cpex-core::PluginResult`, then `erase_result()` boxes it for the type-erased executor pipeline

## Serialization Strategy

| Data | Across WASM boundary | Format |
|------|---------------------|--------|
| Enums (Role, Channel, ResourceType) | WIT enums | Canonical ABI (integers) |
| Simple records | WIT records | Canonical ABI (flat fields) |
| `HashMap<String, Value>` (e.g. tool args) | `string` | JSON-serialized |
| `HashMap<String, String>` (e.g. headers) | `list<tuple<string, string>>` | WIT native list of tuples |
| `HashSet<String>` (e.g. roles, labels) | `list<string>` | WIT native list |
| `PluginContext` state | `string` | JSON-serialized |
| Binary data (blob) | `option<list<u8>>` | WIT native byte list |

## Crate Dependency Graph

```
cpex-core  ←── cpex-wasm-host (depends on cpex-core for native types)
                     ↕ (WASM boundary, same WIT definition)
              cpex-wasm-plugin (depends on cpex-payload)  ──→  cpex-payload
```

Key design: **cpex-payload** is a minimal, WASM-compatible subset of cpex-core's types (no `net`, no heavy tokio features). Plugin authors write against cpex-payload and never touch WASM plumbing directly — the `cpex-wasm-plugin` crate handles all WIT binding/conversion.
