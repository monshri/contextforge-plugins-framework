# Building a CPEX WASM Plugin

This guide covers everything you need to write a plugin and compile it to a
`.wasm` binary that the `cpex-wasm-host` can load.

---

## 1. Set up your plugin crate

In your own crate's `Cargo.toml`:

```toml
[dependencies]
cpex-wasm-plugin = { path = "../cpex-wasm-plugin" }
async-trait = "0.1"

[lib]
crate-type = ["cdylib"]
```

`cdylib` is required — it tells Rust to produce a dynamic library (`.wasm`)
rather than a regular Rust library.

---

## 2. Write your plugin

```rust
use async_trait::async_trait;
use cpex_wasm_plugin::prelude::*;

pub struct MyPlugin;

impl Default for MyPlugin {
    fn default() -> Self { Self }
}

static CONFIG: std::sync::OnceLock<PluginConfig> = std::sync::OnceLock::new();

#[async_trait]
impl Plugin for MyPlugin {
    fn config(&self) -> &PluginConfig {
        CONFIG.get_or_init(|| PluginConfig {
            name: "my-plugin".to_string(),
            kind: "wasm://my-plugin.wasm".to_string(),
            hooks: vec!["cmf.tool_pre_invoke".to_string()],
            ..Default::default()
        })
    }
    async fn initialize(&self) -> Result<(), Box<PluginError>> { Ok(()) }
    async fn shutdown(&self)    -> Result<(), Box<PluginError>> { Ok(()) }
}

impl HookHandler<CmfHook> for MyPlugin {
    async fn handle(
        &self,
        _payload: &MessagePayload,
        _extensions: &Extensions,
        _ctx: &mut PluginContext,
    ) -> PluginResult<MessagePayload> {
        PluginResult::allow()
    }
}

register_wasm_plugin!(MyPlugin, [CmfHook]);
```

See `src/examples/minimal_plugin.rs` for the full annotated version, and
the other files in `src/examples/` for more patterns (PII guard, identity
resolution, token delegation, audit logging, etc.).

---

## 3. Build the `.wasm` binary

```bash
cargo build --target wasm32-wasip2 --release
```

Output: `target/wasm32-wasip2/release/<your-crate-name>.wasm`

If you haven't installed the target yet:
```bash
rustup target add wasm32-wasip2
```

---

## 4. Register the plugin with the host

Copy the `.wasm` binary to `cpex-wasm-host/wasm/` and add an entry to your
`config.yaml`:

```yaml
plugins:
  - name: my-plugin
    kind: wasm://my-plugin.wasm
    hooks: [cmf.tool_pre_invoke]
    mode: sequential
    priority: 10
    on_error: fail
    config:
      sandbox_policy:
        allowed_filesystem: []
        allowed_network: []
        allowed_env: []
        resources:
          max_memory_bytes: 10485760     # 10 MB
          max_fuel: 1000000000           # ~1B instructions per call
          max_execution_time_ms: 5000    # 5s wall-clock timeout
```

---

## 5. Available hooks

| Hook type | Payload | Use for |
|---|---|---|
| `CmfHook` | `MessagePayload` | Tool pre/post invoke, LLM input/output |
| `IdentityHook` | `IdentityPayload` | Resolving subject identity from headers/tokens |
| `TokenDelegateHook` | `DelegationPayload` | Minting scoped outbound credentials |

Register multiple hooks in one macro call:
```rust
register_wasm_plugin!(MyPlugin, [CmfHook, IdentityHook]);
```

---

## 6. Sandbox permissions

By default the plugin has **no access** to the filesystem, network, or
environment variables. Grant access explicitly in `config.yaml`:

```yaml
sandbox_policy:
  # Filesystem — one rule per path
  allowed_filesystem:
    - dir: /tmp/my-plugin-data
      permission: read-only      # read-only | full-access | drop-box |
                                 # fixed-mutable | list-only | private-scratch

  # Network — one rule per host
  allowed_network:
    - host: api.example.com      # exact match
      schemes: [https]           # default: [https]
      ports: [443]               # default: any
      methods: [GET, POST]       # default: any
    - host: "*.internal.svc"     # wildcard matches subdomains only

  # Environment variables
  allowed_env:
    - API_KEY
    - LOG_LEVEL
```
