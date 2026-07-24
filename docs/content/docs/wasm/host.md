---
title: "Host Runtime"
weight: 10
---

# cpex-wasm-host

The host runtime loads WebAssembly Component Model plugins into sandboxed wasmtime environments, enforces resource limits and capability-based access control, and bridges to cpex-core's `PluginManager` for seamless integration with native plugins.

## Modules

| Module | Purpose |
|--------|---------|
| `factory` | `WasmPluginFactory` — implements cpex-core's `PluginFactory` trait for WASM plugins |
| `sandbox_manager` | `SandboxManager` — wasmtime engine, store, component loading, and invocation |
| `policy_loader` | `SandboxPolicy` schema and WASI context construction from YAML config |
| `payload_registry` | Type-erased serialization registry for custom payloads crossing the WASM boundary |
| `conversions` | Native ↔ WIT type conversions for payloads, extensions, and context |

## Usage

```rust
use cpex_wasm_host::factory::WasmPluginFactory;
use cpex_wasm_host::payload_registry::PayloadSerializerRegistry;

// For built-in payloads (CMF, Identity, Delegation):
let factory = WasmPluginFactory::with_builtin_payloads(wasm_dir);

// For custom payloads:
let mut registry = PayloadSerializerRegistry::new();
registry.register::<MyPayload>();
let factory = WasmPluginFactory::new(wasm_dir, Arc::new(registry));

// Register with PluginManager — WASM plugins participate transparently
mgr.register_factory("wasm://my-plugin.wasm", Box::new(factory));
```

## Sandbox Policy

Each WASM plugin's sandbox is configured via a `SandboxPolicy` in YAML. All fields default to empty/deny — an absent or empty policy means full lockdown.

```yaml
sandbox_policy:
  allowed_filesystem:
    - dir: "/data/shared"
      permission: read-only
    - dir: "/tmp/plugin-scratch"
      permission: full-access
    - dir: "/var/artifacts"
      permission: drop-box
  allowed_network:
    - "api.example.com"
    - "internal.service.local"
  allowed_env:
    - "API_KEY"
    - "ENVIRONMENT"
  resources:
    max_memory_bytes: 67108864      # 64 MiB
    max_fuel: 1000000000            # ~1 billion instructions
    max_execution_time_ms: 5000     # 5 seconds per invocation
    max_instances: 10
    max_tables: 10
```

### Filesystem Rules

Each rule specifies either a `dir:` (directory path) or `file:` (file path, whose parent directory is preopened). The `permission` field controls what the plugin can do within that path:

| Permission | DirPerms | FilePerms | Description |
|-----------|----------|-----------|-------------|
| `read-only` | READ | READ | List entries, open and read files; no modifications |
| `full-access` | READ+MUTATE | READ+WRITE | Full access within the preopened path |
| `drop-box` | MUTATE | WRITE | Write-only: create files, no listing, no reading back |
| `fixed-mutable` | READ | READ+WRITE | Read/write existing files; no create/delete |
| `list-only` | READ | (none) | Enumerate filenames only; cannot open file contents |
| `private-scratch` | MUTATE | READ+WRITE | Full file I/O; cannot list directory |

### Network Policy

The `allowed_network` list specifies host names (not URLs). Outbound HTTP requests to any host not in the list are blocked. Subdomain matching is supported — `"example.com"` allows `api.example.com`.

### Resource Limits

| Field | Description | Default |
|-------|-------------|---------|
| `max_memory_bytes` | Maximum linear memory the plugin can allocate | Unlimited |
| `max_fuel` | Maximum instructions (fuel units) across all invocations | Unlimited |
| `max_execution_time_ms` | Maximum wall-clock time for a single invocation | Unlimited |
| `max_instances` | Maximum WASM module instances | Unlimited |
| `max_tables` | Maximum WASM tables | Unlimited |

## SandboxManager

The `SandboxManager` handles the wasmtime lifecycle for a single plugin:

1. **Engine creation** — shared `Engine` with component model and fuel metering enabled
2. **Component loading** — compiles the `.wasm` file into a wasmtime `Component`
3. **Store construction** — builds a `Store` with WASI context (filesystem, env, network) from the sandbox policy
4. **Resource enforcement** — configures `StoreLimits` for memory and table caps, adds fuel
5. **Invocation** — calls the plugin's `handle-hook` export, passing WIT-typed arguments
6. **Network interception** — a `WasiHttpHooks` implementation enforces the network allow-list on every outbound request

## WIT Interface

The host binds to the `plugin` WIT world:

```wit
world plugin {
    import host-logging
    export handle-hook: func(
        hook-name: string,
        payload: hook-payload,
        extensions: extensions,
        ctx: plugin-context,
    ) -> hook-result
}
```

The host-side `host-logging` import routes plugin log messages to the host's `tracing` subscriber with the plugin name attached as a span field.
