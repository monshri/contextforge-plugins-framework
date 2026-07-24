# WASM Feature — File Mapping

This document maps every file on the `feat/plugin-wasm-e2e` branch to the workstream it belongs to.

Legend: `[committed]` = already committed on branch vs main | `[uncommitted]` = modified/untracked locally

---

## Cpex-wasm-plugin

> Full support for adding a plugin in plugin.rs and then compiling it to produce plugin.wasm file

### Build & Configuration

| File | Status | Purpose |
|------|--------|---------|
| `crates/cpex-wasm-plugin/Cargo.toml` | committed | Crate manifest (deps, wasm32-wasi target) |
| `crates/cpex-wasm-plugin/Makefile` | committed | Build commands to compile plugins → .wasm |
| `crates/cpex-wasm-plugin/README.md` | committed | Plugin crate documentation |

### WIT Interface Definition

| File | Status | Purpose |
|------|--------|---------|
| `crates/cpex-wasm-plugin/wit/world.wit` | committed | Main world definition (exports plugin interface) |
| `crates/cpex-wasm-plugin/wit/deps/cli.wit` | committed | WASI CLI types |
| `crates/cpex-wasm-plugin/wit/deps/clocks.wit` | committed | WASI clocks types |
| `crates/cpex-wasm-plugin/wit/deps/filesystem.wit` | committed | WASI filesystem types |
| `crates/cpex-wasm-plugin/wit/deps/http.wit` | committed | WASI HTTP types |
| `crates/cpex-wasm-plugin/wit/deps/io.wit` | committed | WASI I/O types |
| `crates/cpex-wasm-plugin/wit/deps/random.wit` | committed | WASI random types |
| `crates/cpex-wasm-plugin/wit/deps/sockets.wit` | committed | WASI sockets types |

### Plugin Framework (src)

| File | Status | Purpose |
|------|--------|---------|
| `crates/cpex-wasm-plugin/src/lib.rs` | committed | Plugin crate root — bindings, trait impls |
| `crates/cpex-wasm-plugin/src/conversions.rs` | committed | Type conversions between WIT and Rust types |
| `crates/cpex-wasm-plugin/src/plugins/mod.rs` | committed | Module declarations for all plugins |

### Individual Plugins (each compiles to a .wasm)

| File | Status | Purpose |
|------|--------|---------|
| `crates/cpex-wasm-plugin/src/plugins/noop.rs` | committed | No-op passthrough (baseline / smoke test) |
| `crates/cpex-wasm-plugin/src/plugins/header_injector.rs` | committed | Injects/modifies HTTP headers |
| `crates/cpex-wasm-plugin/src/plugins/pii_guard.rs` | committed | PII detection and redaction |
| `crates/cpex-wasm-plugin/src/plugins/identity_checker.rs` | committed | Identity validation logic |
| `crates/cpex-wasm-plugin/src/plugins/audit_logger.rs` | committed | Audit logging plugin |
| `crates/cpex-wasm-plugin/src/plugins/audit_logger_custom.rs` | committed | Custom audit logging with extra fields |
| `crates/cpex-wasm-plugin/src/plugins/token_attenuator.rs` | committed | Token scope attenuation |
| `crates/cpex-wasm-plugin/src/plugins/remote_authz.rs` | committed | Remote authorization check |
| `crates/cpex-wasm-plugin/src/plugins/tool_invoke_checker.rs` | committed | Tool invocation validation |
| `crates/cpex-wasm-plugin/src/plugins/compute_bench.rs` | committed | Compute benchmark plugin |
| `crates/cpex-wasm-plugin/src/plugins/env_test.rs` | committed | Environment variable access test |
| `crates/cpex-wasm-plugin/src/plugins/fs_test.rs` | uncommitted | Filesystem access test |
| `crates/cpex-wasm-plugin/src/plugins/net_test.rs` | committed | Network access test |

---

## Cpex-wasm-host

### WIT file and Sandbox/Policy Layer

> Having the WIT file and then adding a sandbox and policy layer to configure permissions in the wasm instances (make this future ready)

#### WIT Interface Definition

| File | Status | Purpose |
|------|--------|---------|
| `crates/cpex-wasm-host/wit/world.wit` | committed | Host-side world definition (imports/exports) |
| `crates/cpex-wasm-host/wit/deps/cli.wit` | committed | WASI CLI types |
| `crates/cpex-wasm-host/wit/deps/clocks.wit` | committed | WASI clocks types |
| `crates/cpex-wasm-host/wit/deps/filesystem.wit` | committed | WASI filesystem types |
| `crates/cpex-wasm-host/wit/deps/http.wit` | committed | WASI HTTP types |
| `crates/cpex-wasm-host/wit/deps/io.wit` | committed | WASI I/O types |
| `crates/cpex-wasm-host/wit/deps/random.wit` | committed | WASI random types |
| `crates/cpex-wasm-host/wit/deps/sockets.wit` | committed | WASI sockets types |

#### Sandbox & Policy (src)

| File | Status | Purpose |
|------|--------|---------|
| `crates/cpex-wasm-host/src/sandbox_manager.rs` | committed | Sandbox creation with per-instance capability grants |
| `crates/cpex-wasm-host/src/policy_loader.rs` | uncommitted | YAML policy config → sandbox permissions mapping |

#### Policy Configuration Files

| File | Status | Purpose |
|------|--------|---------|
| `crates/cpex-wasm-host/config/config.yaml` | committed | Base policy config |
| `crates/cpex-wasm-host/config/config_capabilities.yaml` | committed | Per-capability grants config |
| `crates/cpex-wasm-host/config/config_plugin_demo.yaml` | committed | Plugin demo policy config |
| `crates/cpex-wasm-host/config/config_fs_test.yaml` | uncommitted | Filesystem sandbox test config |
| `crates/cpex-wasm-host/config/config_sandbox_scenarios.yaml` | uncommitted | Multi-scenario sandbox config |

#### Sandbox Tests

| File | Status | Purpose |
|------|--------|---------|
| `crates/cpex-wasm-host/tests/test_policy_loader.rs` | uncommitted | Policy loader unit/integration tests |
| `crates/cpex-wasm-host/tests/test_sandbox_env.rs` | committed | Environment variable sandbox tests |
| `crates/cpex-wasm-host/tests/test_sandbox_isolation.rs` | uncommitted | Filesystem/network isolation tests |
| `crates/cpex-wasm-host/tests/test_sandbox_network.rs` | committed | Network sandbox tests |
| `crates/cpex-wasm-host/tests/test_security_enforcement.rs` | committed | Security policy enforcement tests |

#### Reference Material

| File | Status | Purpose |
|------|--------|---------|
| `crates/cpex-wasm-host/wasmtime-dirperms-fileperms-guide.md` | uncommitted | Wasmtime permissions reference guide |

---

### Integration with Plugin Manager & E2E Invocation

> Handling the integration with plugin manager and end to end invocation

#### Host Core (src)

| File | Status | Purpose |
|------|--------|---------|
| `crates/cpex-wasm-host/src/lib.rs` | committed | Crate root — WasmPluginHost, module loading, invocation |
| `crates/cpex-wasm-host/src/factory.rs` | committed | WasmPluginFactory — integrates with cpex-core PluginFactory trait |
| `crates/cpex-wasm-host/src/conversions.rs` | committed | CMF ↔ WIT type conversions |
| `crates/cpex-wasm-host/src/payload_registry.rs` | committed | Custom payload type registry for plugins |

#### Build & Configuration

| File | Status | Purpose |
|------|--------|---------|
| `crates/cpex-wasm-host/Cargo.toml` | committed | Crate manifest (wasmtime, cpex-core deps) |
| `crates/cpex-wasm-host/Makefile` | committed | Build/test commands |
| `crates/cpex-wasm-host/README.md` | committed | Host crate documentation |

#### Integration Tests

| File | Status | Purpose |
|------|--------|---------|
| `crates/cpex-wasm-host/tests/test_custom_payload_pipeline.rs` | committed | Custom payload end-to-end pipeline test |

#### Pre-compiled WASM Binaries (used by host tests/demos)

| File | Status | Purpose |
|------|--------|---------|
| `crates/cpex-wasm-host/wasm/noop.wasm` | committed | Compiled noop plugin |
| `crates/cpex-wasm-host/wasm/header-injector.wasm` | committed | Compiled header injector |
| `crates/cpex-wasm-host/wasm/pii-guard.wasm` | committed | Compiled PII guard |
| `crates/cpex-wasm-host/wasm/identity-checker.wasm` | committed | Compiled identity checker |
| `crates/cpex-wasm-host/wasm/audit-logger.wasm` | committed | Compiled audit logger |
| `crates/cpex-wasm-host/wasm/audit-logger-custom.wasm` | committed | Compiled custom audit logger |
| `crates/cpex-wasm-host/wasm/env-test.wasm` | committed | Compiled env test |
| `crates/cpex-wasm-host/wasm/fs-test.wasm` | committed | Compiled fs test |
| `crates/cpex-wasm-host/wasm/net-test.wasm` | committed | Compiled net test |
| `crates/cpex-wasm-host/wasm/remote-authz.wasm` | committed | Compiled remote authz |
| `crates/cpex-wasm-host/wasm/tool-invoke-checker.wasm` | committed | Compiled tool invoke checker |

---

## Documentation

### Introduction to the User

> Introduction to the user on how to use this feature

| File | Status | Purpose |
|------|--------|---------|
| `docs/content/docs/wasm/_index.md` | uncommitted | WASM section landing page |
| `docs/content/docs/wasm/concepts.md` | uncommitted | Core concepts (sandboxing, WIT, capabilities) |
| `docs/content/docs/wasm/host.md` | uncommitted | Host crate usage guide |
| `docs/content/docs/wasm/plugin.md` | uncommitted | Plugin authoring guide |
| `docs/content/docs/wasm/tutorial.md` | uncommitted | Step-by-step tutorial |
| `docs/specs/cpex-wasm-spec.md` | committed | Full WASM integration spec |

### Demos for Sandboxing Scenarios and Plugins

> Demos for each of the sandboxing scenarios and plugins

| File | Status | Purpose |
|------|--------|---------|
| `crates/cpex-wasm-host/examples/wasm_plugin_demo.rs` | uncommitted | Plugin loading & invocation demo |
| `crates/cpex-wasm-host/examples/wasm_capabilities_demo.rs` | uncommitted | Capability-based sandbox demo |
| `crates/cpex-wasm-host/examples/wasm_fs_test_demo.rs` | uncommitted | Filesystem sandboxing demo |
| `crates/cpex-wasm-host/examples/sandboxing_scenarios.md` | uncommitted | Written walkthrough of sandbox scenarios |
| `crates/cpex-wasm-host/examples/data/input.txt` | uncommitted | Sample input data for demos |
| `crates/cpex-wasm-host/examples/data/secrets.txt` | uncommitted | Secret data (tests sandbox blocks access) |
| `crates/cpex-wasm-host/demo/record_chat.py` | uncommitted | Script to record demo sessions |

### Benchmarking

> Benchmarking

| File | Status | Purpose |
|------|--------|---------|
| `crates/cpex-wasm-host/benchmarking/README.md` | committed | Benchmark methodology & results |
| `crates/cpex-wasm-host/benchmarking/comprehensive.rs` | committed | Full benchmark suite |
| `crates/cpex-wasm-host/benchmarking/invocation.rs` | committed | Invocation-focused benchmarks |
| `crates/cpex-wasm-host/benchmarking/plot_results.py` | committed | Python script to plot benchmark results |
| `crates/cpex-wasm-host/benchmarking/performance_comparison.png` | committed | Benchmark results chart |
