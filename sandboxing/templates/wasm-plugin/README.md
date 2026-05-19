# {{project-name}}

{{description}}

## Prerequisites

```bash
# Install the WASM target (one-time)
rustup target add wasm32-wasip2

# Install wasm-tools for verification (optional)
cargo install wasm-tools
```

## Build

```bash
make component
```

This produces `{{project-name}}.component.wasm` — the sandboxed plugin binary.

## Development Workflow

1. **Edit `src/lib.rs`** — your hook logic lives in the `handle_*` functions.
2. **Run `make component`** — compiles to WASM (~200KB typical).
3. **Register in CPEX config** — add the plugin entry to your YAML (see below).
4. **Test** — the host invokes your hooks automatically.

## Registering with CPEX

Add to your CPEX configuration YAML:

```yaml
plugins:
  - name: {{project-name}}
    kind: "wasm://path/to/{{project-name}}.component.wasm"
    hooks: [tool_pre_invoke, tool_post_invoke]
    mode: sequential
    priority: 100
    on_error: fail
    config:
      # Your plugin-specific config fields here.
      # These are passed as JSON to your init() function.
```

## Project Structure

```
├── src/lib.rs       # Your plugin logic (this is where you write code)
├── wit/world.wit    # Interface contract (don't need to modify)
├── policy.yaml      # Sandbox limits (memory, CPU, network access)
├── Makefile         # Build commands
└── Cargo.toml      # Dependencies
```

## What You Need to Know

- **You write normal Rust.** No WASM knowledge needed.
- **Payloads are JSON.** Define a struct, derive `Deserialize`, and parse from `request.payload_json`.
- **Return `HookResult::Allow`, `Deny`, or `Modify`.** That's the full API.
- **Host imports available:** `logging::log()` for structured logs, `state::get_global()`/`set_global()` for shared pipeline state.
- **Sandbox policy** in `policy.yaml` controls what your plugin can access (filesystem, network, memory limits).

## Adjusting the Sandbox Policy

Edit `policy.yaml` to change what the plugin is allowed to access:

```yaml
# Allow outbound HTTP to specific hosts:
allowed_hosts: ["api.example.com"]

# Allow reading environment variables:
allowed_env_vars: ["API_KEY", "ENV"]

# Increase memory limit:
resource_limits:
  max_memory_bytes: 20971520  # 20 MB
```

## Makefile Targets

| Target | Description |
|--------|-------------|
| `make component` | Build the WASM component (default) |
| `make verify` | Print the WIT exports of the built component |
| `make clean` | Remove build artifacts |
