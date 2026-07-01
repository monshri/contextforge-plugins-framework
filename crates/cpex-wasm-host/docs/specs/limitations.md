# Current Implementation Limitations

## Functional Gaps

### 1. `modified_extensions` is silently dropped on the host side

**Location:** `cpex-wasm-host/src/conversions.rs:287`

```rust
modified_extensions: None, // WIT modified_extensions would require OwnedExtensions conversion
```

The guest side (`cpex-wasm-plugin`) correctly converts and returns modified extensions via `native_owned_extensions_to_wit()`, but the host discards them. A plugin that modifies security labels, adds headers, or mutates meta extensions will have those changes silently ignored by the executor pipeline.

### 2. Plugin context mutations are not propagated back

The `PluginContext` is passed by value into the WASM guest, but the host never reads back any context mutations. If a plugin modifies `local_state` or `global_state`, those changes die inside the sandbox. The executor passes `&mut PluginContext` to `invoke()`, but the WASM bridge never writes updated state back into it.

## Scalability

### 3. Mutex serialization per plugin (no instance pool)

**Location:** `cpex-wasm-host/src/factory.rs:87`

```rust
let sandbox = Arc::new(Mutex::new(sandbox));
```

Since wasmtime stores are not `Send`-safe across concurrent calls, all invocations to the same plugin are serialized behind a `tokio::sync::Mutex`. High-throughput hooks hitting one plugin become a bottleneck. There is no pool of pre-instantiated WASM instances to handle concurrent requests.

### 4. One engine + one ticker thread per plugin

Each `WasmPluginFactory::create()` call creates a brand new `Engine`, `Store`, `Linker`, and epoch ticker thread. Loading 20 plugins means 20 wasmtime engines and 20 background threads. Engines could be shared across plugins (especially those using the same configuration), and a single ticker thread could serve all stores on the same engine.

## Resource Leaks

### 5. Memory leak from `Box::leak` on hook names

**Location:** `cpex-wasm-host/src/factory.rs:103`

```rust
let leaked: &'static str = Box::leak(hook_name.clone().into_boxed_str());
```

Every hook name string is leaked to satisfy the `&'static str` lifetime requirement. For a fixed set of plugins this is acceptable, but it will not scale for dynamic plugin loading/unloading scenarios — the strings are never reclaimed.

### 6. Epoch ticker thread is never stopped

**Location:** `cpex-wasm-host/src/sandbox_manager.rs:130`

```rust
std::thread::spawn(move || loop {
    std::thread::sleep(std::time::Duration::from_millis(1));
    engine_clone.increment_epoch();
});
```

The background thread runs an infinite loop with no shutdown signal. No `JoinHandle` is stored, and there is no mechanism to stop it when the `SandboxManager` is dropped. Each plugin gets its own immortal thread.

## Developer Experience

### 7. Hardcoded plugin in cpex-wasm-plugin

**Location:** `cpex-wasm-plugin/src/lib.rs:65`

```rust
let result = cpex_payload::plugins::identity_checker::identity_check(
    &native_payload, &native_extensions, &native_ctx,
);
```

The guest crate is hardcoded to call `identity_checker::identity_check`. To use a different plugin, you must edit the source and recompile the WASM module. There is no dynamic dispatch, feature-flag selection, or config-driven plugin routing inside the WASM module.

### 8. No plugin hot-reloading

The `SandboxManager` loads a WASM file once at startup. There is no mechanism to watch for file changes, reload a module, or swap a running plugin without restarting the entire host. The `instance` field is `Option` but `load_wasmplugin` simply replaces it with no orchestrated drain of in-flight requests.

## Performance

### 9. `block_in_place` in a sync factory method

**Location:** `cpex-wasm-host/src/factory.rs:71-80`

```rust
let sandbox = tokio::task::block_in_place(|| {
    tokio::runtime::Handle::current().block_on(async { ... })
});
```

The `PluginFactory::create()` trait is synchronous, so WASM loading (which is async) is forced through `block_in_place` + `block_on`. This blocks the current tokio worker thread and can cause stalls if many plugins are loaded concurrently during startup.

### 10. No WASM module pre-compilation or caching

`Component::from_file()` compiles the WASM module from scratch on every load. For large modules, this is expensive. wasmtime supports serializing compiled modules to disk (`Component::serialize` / `unsafe Component::deserialize`) but this is not used. Repeated startups pay the full compilation cost each time.

## Observability

### 11. No structured logging or metrics

All tracing is `eprintln!` debug output. There is no:
- Structured logging (tracing spans, JSON log events)
- Metrics collection (invocation latency, fuel consumed per call, memory highwater mark)
- Integration with the host's telemetry/tracing system
- Distinction between debug output and production-relevant signals

## Correctness

### 12. Silent data loss on JSON deserialization failure

All JSON deserialization uses `unwrap_or_default()`:

```rust
serde_json::from_str(&tc.arguments).unwrap_or_default();
```

If a plugin returns malformed JSON in fields like `tool-call.arguments`, `plugin-violation.details`, or `plugin-context.local-state`, the host receives an empty HashMap with no error signal. Data is silently lost with no way to detect the issue.

## Summary

| Severity | Issue | Impact |
|----------|-------|--------|
| Functional gap | `modified_extensions` silently dropped | Plugins cannot modify security/meta/HTTP state |
| Functional gap | Plugin context mutations not propagated | Plugins cannot persist state across invocations |
| Scalability | Mutex serialization per plugin | Single-plugin throughput bottleneck |
| Scalability | One engine + ticker thread per plugin | Thread/memory overhead scales linearly with plugin count |
| Resource leak | `Box::leak` for hook names | Unbounded memory growth with dynamic plugins |
| Resource leak | Immortal ticker threads | Threads never cleaned up |
| DX | Hardcoded plugin in guest | Requires recompilation to swap plugin logic |
| DX | No hot-reloading | Requires full restart to update plugins |
| Performance | `block_in_place` during load | Blocks tokio worker threads |
| Performance | No pre-compilation cache | Repeated startup cost |
| Observability | Only `eprintln!` tracing | No production-grade telemetry |
| Correctness | Silent JSON deserialization fallback | Data loss without error signal |
