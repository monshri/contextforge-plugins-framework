# WIT Type System

## Overview

WIT (WebAssembly Interface Types) is the IDL used to define the contract between the WASM host and plugin components. It supports a limited set of portable types — anything richer (like Rust's `HashMap`, `Arc`, or trait objects) must be manually converted before crossing the boundary.

## Supported Types

### Primitives

| Type | Description |
|------|-------------|
| `bool` | Boolean |
| `u8`, `u16`, `u32`, `u64` | Unsigned integers |
| `s8`, `s16`, `s32`, `s64` | Signed integers |
| `f32`, `f64` | Floating point |
| `char` | Unicode scalar value |
| `string` | UTF-8 string |

### Containers

| Type | Description |
|------|-------------|
| `list<T>` | Ordered sequence of elements |
| `option<T>` | Nullable value (Some or None) |
| `result<T, E>` | Success or error |
| `tuple<T1, T2, ...>` | Fixed-size heterogeneous tuple |

### User-Defined Types

| Type | Description | Rust Analogue |
|------|-------------|---------------|
| `record` | Named fields | `struct` |
| `enum` | Unit variants only (no data) | `enum { A, B, C }` |
| `variant` | Tagged union with data per arm | `enum { A(T), B(U) }` |
| `flags` | Bitfield set of named booleans | `bitflags!` |

## Not Supported

WIT has no equivalent for:

| Rust Type | WIT Workaround |
|-----------|----------------|
| `HashMap<K, V>` | `list<tuple<K, V>>` |
| `HashSet<T>` | `list<T>` |
| `Arc<T>` / `Box<T>` / references | Copy the value into a `record` |
| `serde_json::Value` (arbitrary JSON) | `string` (serialize/deserialize manually) |
| Trait objects / `dyn Trait` | Not expressible — use `variant` or `string` + runtime dispatch |
| Generics / type parameters | Not supported — monomorphize manually |
| Nested dynamic structures | Flatten or serialize to `string` |

## Why Conversions Are Necessary

WASM components run in an isolated memory sandbox. They cannot dereference host pointers, share heap allocations, or access Rust-specific types like `Arc` or `HashMap`. The WIT interface is the only communication channel.

This means every value crossing the host ↔ plugin boundary must be:

1. **Going in (host → WASM):** Converted from rich Rust types to flat WIT types
   - `HashMap<String, String>` → `list<tuple<string, string>>`
   - `HashMap<String, Value>` → `string` (JSON-serialized)
   - `Arc<SecurityExtension>` → unwrapped, fields copied into a WIT `record`
   - `HashSet<String>` → `list<string>`

2. **Coming out (WASM → host):** Converted back to Rust types
   - `list<tuple<string, string>>` → `HashMap<String, String>`
   - `string` → deserialized back to `serde_json::Value` or concrete struct
   - WIT `record` fields → reconstructed into native struct, wrapped in `Arc`

## Example: How a ToolCall Crosses the Boundary

**Native Rust (cpex-core):**
```rust
struct ToolCall {
    tool_call_id: String,
    name: String,
    arguments: HashMap<String, serde_json::Value>,  // ← not expressible in WIT
    namespace: Option<String>,
}
```

**WIT definition:**
```wit
record tool-call {
    tool-call-id: string,
    name: string,
    arguments: string,           // ← JSON-serialized HashMap
    namespace: option<string>,
}
```

**Host → WASM conversion:**
```rust
fn native_tool_call_to_wit(tc: &ToolCall) -> WitToolCall {
    WitToolCall {
        tool_call_id: tc.tool_call_id.clone(),
        name: tc.name.clone(),
        arguments: serde_json::to_string(&tc.arguments).unwrap_or("{}".into()),
        namespace: tc.namespace.clone(),
    }
}
```

**WASM → Host conversion:**
```rust
fn wit_tool_call_to_native(tc: WitToolCall) -> ToolCall {
    ToolCall {
        tool_call_id: tc.tool_call_id,
        name: tc.name,
        arguments: serde_json::from_str(&tc.arguments).unwrap_or_default(),
        namespace: tc.namespace,
    }
}
```

## Design Decisions in CPEX

1. **Maps as JSON strings** — Fields like `arguments`, `annotations`, and `history` are serialized to JSON strings rather than `list<tuple<...>>` because they contain nested `serde_json::Value` (which has no WIT equivalent).

2. **Maps as tuples** — Simple string-to-string maps (HTTP headers, properties, claims) use `list<tuple<string, string>>` for type safety without JSON overhead.

3. **Custom payloads as JSON envelope** — Non-CMF payload types are wrapped in `custom-payload { payload-type: string, payload-json: string }` since WIT can't express open-ended polymorphism.

## Why cpex-payload Exists (Instead of Compiling cpex-core to WASM)

WASM plugins need the type definitions (Message, ContentPart, Extensions, PluginResult) but not the runtime engine. `cpex-core` cannot compile to `wasm32-wasip2` because of:

| Blocker | Why it fails in WASM |
|---------|---------------------|
| **Tokio** | Relies on epoll/kqueue/IOCP, OS threads, and timers — none exist in a WASM sandbox |
| **async-trait** | Generates `Pin<Box<dyn Future + Send>>` with `Send` bounds assuming a multi-threaded runtime |
| **Arc + Send + Sync bounds** | Used throughout PluginManager, HookRegistry, Extensions — pulls in atomics/threading primitives not available in WASM |
| **PluginManager / 5-phase Executor** | The orchestration engine (routing, scheduling, context tables) doesn't belong inside a plugin — a plugin only implements `handle_hook` |
| **Heavy dependencies** | `tracing`, `serde_yaml`, `dashmap`, etc. either don't compile to WASM or bloat the binary |

`cpex-payload` is the **data model layer** extracted for WASM compilation:
- Type definitions: Message, ContentPart, enums, Extensions, PluginResult
- Plugin logic helpers: the functions plugins actually call (e.g., `header_inject`)
- No runtime machinery: no executor, no manager, no async runtime, no threading

This gives WASM plugins everything they need to process payloads without pulling in the engine that orchestrates them.
