# Type Conversion Specification

## Overview

Data crosses the WASM boundary twice per invocation: once on the way in (Native → WIT) and once on the way out (WIT → Native). There are **two separate conversion modules** — one on each side of the boundary:

| Module | Location | Direction |
|--------|----------|-----------|
| Host conversions | `cpex-wasm-host/src/conversions.rs` | Native cpex-core ↔ WIT |
| Guest conversions | `cpex-wasm-plugin/src/conversions.rs` | WIT ↔ Native cpex-payload |

## Host-Side Conversions (cpex-wasm-host)

### Native → WIT (Inbound)

| Function | From | To |
|----------|------|-----|
| `native_payload_to_wit` | `cpex_core::cmf::message::MessagePayload` | WIT `MessagePayload` |
| `native_extensions_to_wit` | `cpex_core::extensions::Extensions` | WIT `Extensions` |
| `native_context_to_wit` | `cpex_core::context::PluginContext` | WIT `PluginContext` |

Key transformations:
- `HashMap<String, Value>` → JSON string (tool args, annotations, context state)
- `HashMap<String, String>` → `list<tuple<string, string>>` (HTTP headers)
- `HashSet<String>` → `list<string>` (roles, permissions, labels, tags)
- `Vec<Message>` → JSON string (prompt result messages)
- Rust enums → WIT enums (1:1 variant mapping)

### WIT → Native (Outbound — plugin results)

| Function | From | To |
|----------|------|-----|
| `wit_result_to_native` | WIT `PluginResult` | `cpex_core::hooks::PluginResult<MessagePayload>` |
| `wit_payload_to_native` | WIT `MessagePayload` | `cpex_core::cmf::message::MessagePayload` |
| `wit_violation_to_native` | WIT `PluginViolation` | `cpex_core::error::PluginViolation` |

Key transformations:
- JSON string → `HashMap<String, Value>` (deserialization)
- `list<tuple<string, string>>` → `HashMap<String, String>`
- WIT enums → Rust enums

## Guest-Side Conversions (cpex-wasm-plugin)

### WIT → Native (Inbound — receiving from host)

| Function | From | To |
|----------|------|-----|
| `wit_payload_to_native` | WIT `MessagePayload` | `cpex_payload::cmf::message::MessagePayload` |
| `wit_extensions_to_native` | WIT `Extensions` | `cpex_payload::extensions::Extensions` |
| `wit_context_to_native` | WIT `PluginContext` | `cpex_payload::context::PluginContext` |

Key transformations:
- JSON string → `HashMap<String, Value>` (tool args, annotations, context)
- `list<tuple<string, string>>` → `HashMap<String, String>` (headers)
- `list<string>` → `HashSet<String>` (roles, permissions, teams, tags)
- Extension sub-structs wrapped in `Arc` for shared ownership
- `MonotonicSet` rebuilt from label list (security labels)

### Native → WIT (Outbound — returning to host)

| Function | From | To |
|----------|------|-----|
| `native_result_to_wit` | `cpex_payload::hooks::PluginResult<MessagePayload>` | WIT `PluginResult` |
| `native_payload_to_wit` | `cpex_payload::cmf::message::MessagePayload` | WIT `MessagePayload` |
| `native_violation_to_wit` | `cpex_payload::error::PluginViolation` | WIT `PluginViolation` |
| `native_owned_extensions_to_wit` | `cpex_payload::extensions::OwnedExtensions` | WIT `Extensions` |

Key transformations:
- `HashMap<String, Value>` → JSON string (serialization)
- `HashMap<String, String>` → `list<tuple<string, string>>`
- `HashSet<String>` → `list<string>`
- `Vec<Message>` → JSON string (prompt result messages)

## Type Correspondence

### cpex-core vs cpex-payload

Both the host and guest have their own native type systems that mirror each other:

| Concept | cpex-core (host) | cpex-payload (guest) |
|---------|-----------------|---------------------|
| Message | `cpex_core::cmf::message::Message` | `cpex_payload::cmf::message::Message` |
| ContentPart | `cpex_core::cmf::content::ContentPart` | `cpex_payload::cmf::content::ContentPart` |
| Extensions | `cpex_core::extensions::container::Extensions` | `cpex_payload::extensions::container::Extensions` |
| PluginContext | `cpex_core::context::PluginContext` | `cpex_payload::context::PluginContext` |
| PluginResult | `cpex_core::hooks::trait_def::PluginResult<T>` | `cpex_payload::hooks::PluginResult<T>` |
| PluginViolation | `cpex_core::error::PluginViolation` | `cpex_payload::error::PluginViolation` |

The WIT types act as the intermediate representation between these two type systems.

## Error Handling

- JSON deserialization failures (`serde_json::from_str`) default to empty collections (`HashMap::default()`, `Vec::default()`)
- JSON serialization failures default to `"{}"` or `"[]"`
- This ensures the WASM boundary never panics on malformed data — it degrades gracefully
