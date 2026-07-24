---
title: "WASM Plugins"
weight: 60
bookCollapseSection: true
---

# WASM Plugin System

CPEX supports WebAssembly Component Model plugins as a first-class extension mechanism. WASM plugins run in sandboxed wasmtime environments with capability-based access control, providing strong isolation guarantees while participating transparently alongside native Rust plugins.

If you are new to WebAssembly or the Component Model, start with [Concepts]({{< relref "/docs/wasm/concepts" >}}) — it covers sandboxing, WIT interfaces, and capability-based access control before diving into the API.

The WASM subsystem is split into two crates:

| Crate | Role |
|-------|------|
| [`cpex-wasm-host`]({{< relref "/docs/wasm/host" >}}) | Host runtime — loads, sandboxes, and invokes WASM plugins |
| [`cpex-wasm-plugin`]({{< relref "/docs/wasm/plugin" >}}) | Plugin SDK — macros and conversions for authoring WASM plugins |

## Architecture

```
┌─────────────────────────────────────────────────────┐
│                   cpex-core                          │
│              PluginManager                           │
│     ┌──────────────┬──────────────────┐             │
│     │ Native Plugin│  WasmPluginFactory│             │
│     └──────────────┘──────────┬───────┘             │
└───────────────────────────────┼─────────────────────┘
                                │
                    ┌───────────▼───────────┐
                    │   cpex-wasm-host      │
                    │   SandboxManager      │
                    │   ┌───────────────┐   │
                    │   │ wasmtime      │   │
                    │   │  ┌─────────┐  │   │
                    │   │  │ Plugin  │  │   │
                    │   │  │ (.wasm) │  │   │
                    │   │  └─────────┘  │   │
                    │   └───────────────┘   │
                    └───────────────────────┘
```

A `WasmPluginFactory` implements cpex-core's `PluginFactory` trait, so the plugin manager routes hook invocations to WASM plugins exactly as it would to native ones. The `SandboxManager` handles the wasmtime lifecycle — engine, store, component instantiation, and resource enforcement.

## Key Features

- **Sandboxed execution** — each plugin runs in an isolated wasmtime store with configurable memory, fuel, and time limits
- **Capability-based access** — filesystem, network, and environment access are granted per-plugin via YAML policy
- **WIT Component Model** — plugins export a single `handle-hook` function; the host handles all type marshalling
- **Transparent integration** — WASM plugins register with `PluginManager` and participate in the same hook pipeline as native plugins
- **Structured logging** — plugins call a `host-logging` WIT import; the host routes messages to its tracing subscriber

## Quick Start

1. **Read [Concepts]({{< relref "/docs/wasm/concepts" >}})** if you are new to WebAssembly or the Component Model
2. **Run the [Tutorial]({{< relref "/docs/wasm/tutorial" >}})** to see sandbox isolation and a four-plugin pipeline against the reference plugins
3. **Write a plugin** using `cpex-wasm-plugin` (see [Plugin SDK]({{< relref "/docs/wasm/plugin" >}}))
4. **Compile to WASM** with `cargo component build --release`
5. **Configure a sandbox policy** in your plugin's YAML config
6. **Register with the host** via `WasmPluginFactory` (see [Host Runtime]({{< relref "/docs/wasm/host" >}}))
