# Sandbox Policy Specification

## Overview

The `SandboxPolicy` defines what host resources a WASM plugin can access. It is deserialized from the plugin's `config.sandbox_policy` YAML key and enforced by the `SandboxManager` at runtime.

**Default posture: deny-all.** A missing or empty policy means the plugin runs in full lockdown with no access to the outside world.

## Policy Schema

```yaml
sandbox_policy:
  allowed_filesystem:
    - dir: /tmp/data
      permission: "read"       # "read" | "write" | "mutate"
    - file: /etc/config.toml
      permission: "read"
  allowed_network:
    - "httpbin.org"            # exact match or subdomain match
    - "api.example.com"
  allowed_env:
    - "API_KEY"                # only these env vars are visible to the plugin
    - "LOG_LEVEL"
  resources:
    max_memory_bytes: 10485760   # 10 MB linear memory cap
    max_fuel: 1000000000         # instruction budget (session-level)
    max_execution_time_ms: 5000  # per-invocation wall-clock timeout
    max_instances: 10            # max WASM module instances
    max_tables: 10               # max WASM tables
```

## Enforcement Mechanisms

### Filesystem

- Uses WASI preopened directories (`WasiCtxBuilder::preopened_dir`)
- Only explicitly listed paths are visible inside the WASM sandbox
- Permission levels:
  - `read` — `DirPerms::READ` + `FilePerms::READ`
  - `write` / `mutate` — `DirPerms::READ | MUTATE` + `FilePerms::READ | WRITE`
- File rules preopen the parent directory with the specified permission

### Network

- Outbound HTTP is provided via `wasi:http/outgoing-handler`
- A `NetworkPolicy` hook intercepts every outbound request before it leaves the host
- The request's authority (host) is checked against `allowed_network`:
  - Exact match: `authority == allowed_host`
  - Subdomain match: `authority.ends_with(".{allowed_host}")`
- Denied requests return `ErrorCode::HttpRequestDenied`

### Environment Variables

- Only env vars listed in `allowed_env` are injected into the WASI context
- The host reads the actual value from `std::env::var(key)` at plugin load time
- Unlisted variables are invisible to the plugin

### Resource Limits

| Limit | Scope | Enforcement |
|-------|-------|-------------|
| `max_memory_bytes` | Store lifetime | `StoreLimitsBuilder::memory_size` — traps on grow failure |
| `max_fuel` | Session (across all invocations) | `Store::set_fuel` — traps when fuel exhausted |
| `max_execution_time_ms` | Per invocation | Epoch interruption — deadline reset each `invoke()` call |
| `max_instances` | Store lifetime | `StoreLimitsBuilder::instances` |
| `max_tables` | Store lifetime | `StoreLimitsBuilder::tables` |

### Epoch-based Timeout

- A background thread increments the engine epoch every ~1ms
- Each invocation resets the epoch deadline to `max_execution_time_ms` ticks
- If the deadline is exceeded during execution, the store traps
- This prevents any single plugin call from hanging indefinitely

### Fuel Budget

- Fuel is a session-level budget — it is NOT reset between invocations
- Once exhausted, all subsequent calls trap immediately
- This bounds the total compute a plugin can consume across its lifetime
