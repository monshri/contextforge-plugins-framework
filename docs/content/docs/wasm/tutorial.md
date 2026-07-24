---
title: "Tutorial"
weight: 15
---

# WASM Plugin Tutorial

This tutorial walks through two concrete scenarios using the reference plugins that ship with `cpex-wasm-plugin`. No prior Wasm experience is needed — read [Concepts]({{< relref "/docs/wasm/concepts" >}}) first if you want the background.

**What you will see:**

1. **Fine-grained sandbox isolation** — three plugins probing the filesystem, network, and environment, each blocked or allowed exactly according to its declared policy
2. **A four-plugin pipeline** — PII guard, identity resolver, audit logger, and remote authorizer running as WASM components inside a single `PluginManager`

---

## Prerequisites

```bash
rustup target add wasm32-wasip2
cargo install cargo-component
```

Clone the repo and build the plugins used in this tutorial:

```bash
git clone https://github.com/contextforge-org/cpex.git && cd cpex
cd crates/cpex-wasm-host
make build-test-plugins    # fs-test, net-test, env-test, noop → wasm/
make build-all-plugins     # capability demo plugins → wasm/
```

---

## Part 1 — Fine-grained sandbox isolation

Each plugin in this part is a single Rust file compiled to a `.wasm` binary. The plugin probes a resource (filesystem, network, or environment variables) and writes what it found into the `PluginContext` local state. The host integration tests then assert on that state — no `unsafe`, no special test harness, just the normal CPEX pipeline.

### 1a. Filesystem isolation

**Source:** `crates/cpex-wasm-plugin/src/plugins/fs_test.rs`

The plugin attempts to read `/etc/passwd`:

```rust
match std::fs::read_to_string("/etc/passwd") {
    Ok(content) => {
        ctx.set_local("fs_read_success", true);
        ctx.set_local("content_len", content.len() as i64);
    }
    Err(e) => {
        ctx.set_local("fs_read_success", false);
        ctx.set_local("error", e.to_string());
    }
}
PluginResult::allow()
```

Run the integration test:

```bash
cargo test -p cpex-wasm-host test_filesystem_sandbox -- --include-ignored
```

**What the test asserts:**

| Sandbox policy | `fs_read_success` | Notes |
|----------------|-------------------|-------|
| No policy (full lockdown) | `false` | Error contains "denied" or "Capabilities insufficient" |
| `allowed_filesystem: [{dir: "/tmp", permission: full-access}]` | `false` | `/etc/passwd` is still outside the granted path |

The plugin code is identical in both cases. The only difference is the `SandboxPolicy` passed to `SandboxManager` at load time. The WASM binary itself cannot distinguish between the two — it just sees a permission error from the WASI filesystem interface.

### 1b. Environment variable isolation

**Source:** `crates/cpex-wasm-plugin/src/plugins/env_test.rs`

The plugin reads four environment variables and writes their values to context:

```rust
for key in ["HOME", "PATH", "CPEX_TEST_ALLOWED", "SECRET_API_KEY"] {
    let val = std::env::var(key).unwrap_or_default();
    ctx.set_local(key, val);
}
PluginResult::allow()
```

Run the test:

```bash
cargo test -p cpex-wasm-host test_sandbox_env -- --include-ignored
```

**What the test asserts:**

| Policy | `HOME` | `CPEX_TEST_ALLOWED` | `SECRET_API_KEY` |
|--------|--------|---------------------|------------------|
| No policy | `""` | `""` | `""` |
| `allowed_env: [CPEX_TEST_ALLOWED]` | `""` | `"test_value"` | `""` |

A variable not listed in `allowed_env` is invisible to the plugin — `std::env::var()` returns `Err(NotPresent)` as if it was never set. `SECRET_API_KEY` never leaks regardless of policy.

The sandbox policy for this scenario:

```yaml
sandbox_policy:
  allowed_env:
    - CPEX_TEST_ALLOWED
  resources:
    max_memory_bytes: 10485760
    max_fuel: 1000000000
    max_execution_time_ms: 5000
```

### 1c. Network isolation

**Source:** `crates/cpex-wasm-plugin/src/plugins/net_test.rs`

The plugin attempts a DNS resolution for `httpbin.org` and records the outcome:

```rust
match ("httpbin.org", 80).to_socket_addrs() {
    Ok(_) => ctx.set_local("net_access", "dns_resolved"),
    Err(_) => ctx.set_local("net_access", "denied"),
}
PluginResult::allow()
```

Run the test:

```bash
cargo test -p cpex-wasm-host test_sandbox_network -- --include-ignored
```

**What the test asserts:**

| Policy | `net_access` |
|--------|-------------|
| No policy | `"denied"` |
| `allowed_network: ["internal.example.com"]` | `"denied"` — `httpbin.org` is not on the allow-list |

Network enforcement in CPEX happens at two levels: the WASI raw-socket layer (which WASM components cannot access directly) and the WASI HTTP outgoing-handler, where the host's `NetworkPolicy` intercepts every outbound HTTP request and checks the hostname against `allowed_network`. Adding `httpbin.org` to the list would make the DNS resolution succeed.

### What isolation actually means

All three plugins above run in the same host process — the same OS PID as your Rust application. There is no container, no subprocess, no VM boundary. The isolation is enforced entirely by the wasmtime sandbox:

- The plugin's linear memory is separate from the host's heap; it cannot read host data structures.
- WASI capabilities are the only way for the plugin to reach outside its sandbox; each is individually gated by `SandboxPolicy`.
- A plugin that crashes (trap) or exhausts its fuel budget does not affect the host process or other plugins.

---

## Part 2 — A four-plugin pipeline

This scenario runs four WASM plugins in sequence on a custom `ToolInvokePayload`. Each plugin handles a different concern; together they demonstrate that independent sandbox components can compose transparently inside a single `PluginManager`.

**Source:** `crates/cpex-wasm-host/examples/wasm_plugin_demo.rs`

Run it (plugins must be built first):

```bash
make run-plugin-demo
# or directly:
cargo run -p cpex-wasm-host --example wasm_plugin_demo
```

### The plugins

| Plugin | WASM binary | What it does |
|--------|-------------|--------------|
| `identity-resolver` | `wasm/tool-invoke-checker.wasm` | Denies if `payload.user` is empty |
| `pii-guard` | `wasm/pii-guard.wasm` | Denies if `ctx.get_global("pii_clearance")` is not `true` |
| `audit-logger-custom` | `wasm/audit-logger-custom.wasm` | Logs tool name and user; always allows |
| `remote-authz` | `wasm/remote-authz.wasm` | Checks user against an in-memory ACL; denies unknown users |

### The payload type

All four plugins share the same custom payload type, registered with the host's `PayloadSerializerRegistry`:

```rust
#[derive(Serialize, Deserialize)]
struct ToolInvokePayload {
    tool_name: String,
    user:      String,
    arguments: serde_json::Value,
}
impl_plugin_payload!(ToolInvokePayload);
impl_wasm_payload!(ToolInvokePayload, "cpex.tool_invoke");
```

The `"cpex.tool_invoke"` string is the discriminator the host and all four plugins use to match and decode the payload across the WASM boundary.

### The policy

From `config/config_plugin_demo.yaml`:

```yaml
plugins:
  - name: identity-resolver
    kind: "wasm://tool-invoke-checker.wasm"
    hooks: [tool_pre_invoke, tool_post_invoke]
    priority: 10

  - name: pii-guard
    kind: "wasm://pii-guard.wasm"
    hooks: [tool_pre_invoke]
    tags: [pii]
    priority: 20

  - name: audit-logger-custom
    kind: "wasm://audit-logger-custom.wasm"
    hooks: [tool_pre_invoke, tool_post_invoke]
    mode: fire_and_forget
    on_error: ignore
    priority: 100

  - name: remote-authz
    kind: "wasm://remote-authz.wasm"
    hooks: [tool_pre_invoke]
    tags: [external_authz]
    priority: 30

routes:
  - tool: get_compensation
    tags: [pii, external_authz]
  - tool: list_departments
    tags: []
  - tool: "*"
```

### Walkthrough of the scenarios

**Scenario 1 — No identity:**
`ToolInvokePayload { tool_name: "get_compensation", user: "", ... }`

The `identity-resolver` plugin runs first (priority 10) and denies immediately — `user` is empty. The pipeline stops; `pii-guard`, `remote-authz`, and `audit-logger` never run.

```
✗ DENIED  identity-resolver  [no_identity] user field is required
```

**Scenario 2 — PII tool, no clearance:**
`user: "alice"`, `tool_name: "get_compensation"` (tagged `pii`), `ctx.pii_clearance = false`

`identity-resolver` passes (user present). `pii-guard` runs and reads `ctx.get_global("pii_clearance")` — returns false — and denies.

```
✓ identity-resolver  allowed
✗ DENIED  pii-guard  [pii_access_denied] PII clearance required
```

**Scenario 3 — PII tool, with clearance:**
Same as scenario 2 but `ctx.pii_clearance = true` and `user: "alice"` (alice is in the `remote-authz` ACL).

All four plugins run. `audit-logger` fires after `remote-authz` allows.

```
✓ identity-resolver  allowed
✓ pii-guard          allowed
✓ remote-authz       allowed  (alice in ACL)
✓ audit-logger       [fire-and-forget] logged
→ ALLOWED
```

**Scenario 4 — Remote authz ACL miss:**
`user: "charlie"` (not in the ACL), `pii_clearance = true`

`remote-authz` uses `static ACL: OnceLock<HashSet<String>>` — populated once on first call and reused across all subsequent invocations. This demonstrates that module-level state persists across WASM invocations: `SandboxManager` keeps the `Store` alive between calls, so a WASM static behaves like an in-memory cache in a long-running process.

```
✓ identity-resolver  allowed
✓ pii-guard          allowed
✗ DENIED  remote-authz  [remote_authz_denied] user not in ACL
```

**Scenario 5 — Non-PII tool (`list_departments`):**
The route has no `pii` tag, so `pii-guard` is not in the pipeline for this route. `remote-authz` is also not tagged. Only `identity-resolver` and `audit-logger` run.

```
✓ identity-resolver  allowed
✓ audit-logger       [fire-and-forget] logged
→ ALLOWED
```

### Context threading

The `audit-logger` plugin runs in `fire_and_forget` mode and receives the same `PluginContext` as the enforcement plugins. After a pre-invoke pass, the context table is threaded into the post-invoke pass — values set by `pii-guard` or `remote-authz` in pre-invoke are visible to `audit-logger` in post-invoke without any explicit wiring.

---

## Part 3 — Capability filtering (what each plugin can see)

**Source:** `crates/cpex-wasm-host/examples/wasm_capabilities_demo.rs`

Run it:

```bash
make run-capabilities-demo
# or:
cargo run -p cpex-wasm-host --example wasm_capabilities_demo
```

This demo uses three CMF-payload plugins with different `capabilities` lists. The `WasmBridgeHandler` filters the `extensions` record before passing it to each plugin — a plugin that lacks `read_labels` never receives the security labels in its context, even if the host has them.

From `config/config_capabilities.yaml`:

```yaml
plugins:
  - name: identity-checker
    capabilities: [read_labels, read_subject, read_roles]

  - name: header-injector
    capabilities: [read_headers, write_headers, append_labels]

  - name: audit-logger
    capabilities: [read_headers, read_labels]
    mode: audit
```

**What the demo shows:**

1. `identity-checker` sees the security labels and subject/roles; it never sees HTTP headers.
2. `header-injector` reads and writes HTTP headers and appends a label; it never sees the subject or roles.
3. `audit-logger` receives labels and headers for observation; its writes are discarded (`mode: audit`).
4. After the pipeline, the modified labels (from `header-injector`) and the original subject (never written by any plugin) are both present in the final extensions — slots that a plugin could not write are preserved from the host's copy during writeback.

The integration test `test_security_enforcement.rs` covers the full set of security invariants the host enforces on every WASM result regardless of what the plugin returns:

- **Immutable tier validation** — the host detects if a plugin replaced a protected pointer (`Arc` identity check on `request_id`).
- **Monotonic label enforcement** — a plugin may add security labels but never remove them; the host re-merges the original label set on every writeback.
- **Write authorization** — a plugin that modifies a field it lacks the `write_*` capability for has that modification silently dropped.

---

## Summary

| Scenario | Key takeaway |
|----------|-------------|
| Filesystem probe | A plugin outside its allowed path gets a permissions error — same code, different outcome |
| Env-var probe | Only variables named in `allowed_env` are visible; all others appear unset |
| Network probe | Outbound connections to unlisted hosts are blocked at the WASI HTTP layer |
| Four-plugin pipeline | Each WASM component enforces one concern; they compose without coupling |
| Capability filtering | Plugins only see and can only mutate the extension fields they were granted |

## Next steps

- [Plugin SDK]({{< relref "/docs/wasm/plugin" >}}): implement your own WASM plugin.
- [Host Runtime]({{< relref "/docs/wasm/host" >}}): configure `SandboxPolicy`, resource limits, and `WasmPluginFactory`.
- [Concepts]({{< relref "/docs/wasm/concepts" >}}): the underlying model — sandboxing, WIT, capability-based access control.
