---
title: "Concepts"
weight: 5
---

# WebAssembly and the Component Model

This page is for readers who are new to WebAssembly or the Component Model. It explains just enough to understand how CPEX WASM plugins work — you do not need any prior Wasm experience to write one.

## What is WebAssembly?

WebAssembly (Wasm) is a binary instruction format that any conforming runtime can execute. It was designed with four properties that make it attractive as a plugin substrate:

**Sandboxed by default.** A Wasm module cannot read the host's memory, call the host's functions, or access the filesystem, network, or environment unless the host explicitly grants each capability. This is the opposite of a native shared library, where the loaded code inherits everything the host process can do.

**Language-neutral.** Rust, C, C++, Go, Python, and others compile to Wasm. Plugin authors pick their language; the host does not care.

**Deterministic execution.** Given the same inputs and sandbox configuration, a Wasm module produces the same outputs on any platform. This makes behaviour reproducible across development, CI, and production.

**Small and fast to load.** A `.wasm` binary is compact and compiles to native code quickly. CPEX compiles each plugin once at startup; subsequent invocations run at near-native speed.

## The Core/Component split

The Wasm ecosystem has two related specifications:

| Spec | What it defines |
|------|-----------------|
| **Wasm Core** | The low-level instruction set: integers, floats, linear memory, a stack machine. All values are scalars. |
| **Wasm Component Model** | A higher-level layer on top of Core. Adds rich types (strings, records, variants, lists), a typed interface language (WIT), and a linking model so components can import and export typed functions. |

CPEX plugins use the **Component Model**. This matters for one practical reason: Core Wasm only speaks scalars, so passing a string between a host and a plugin requires custom serialisation glue. The Component Model standardises that glue. When the host calls `handle-hook` with a `hook-payload` argument, the runtime handles all the memory layout and copying — the plugin author receives a typed Rust struct.

## WIT: the interface language

WIT (Wasm Interface Types) is the IDL for the Component Model, similar in role to protobuf or OpenAPI but designed for Wasm boundaries.

A CPEX plugin implements the `plugin` WIT world:

```wit
world plugin {
    import host-logging
    export handle-hook: func(
        hook-name: string,
        payload:   hook-payload,
        extensions: extensions,
        ctx:       plugin-context,
    ) -> hook-result
}
```

The `export` line declares what the plugin must provide — the host calls it on every hook invocation. The `import` line declares what the host provides — the plugin calls it to emit structured log messages. WIT enforces this contract at link time; a plugin that does not export `handle-hook` will not load.

## Capability-based access control

A Wasm component starts with zero ambient authority: no files, no network, no environment variables. Every capability must be explicitly threaded in by the host via WASI (the WebAssembly System Interface).

CPEX models this through a per-plugin `SandboxPolicy`:

```yaml
sandbox_policy:
  allowed_filesystem:
    - dir: "/data/shared"
      permission: read-only
  allowed_network:
    - "api.example.com"
  allowed_env:
    - "API_KEY"
  resources:
    max_memory_bytes: 67108864   # 64 MiB
    max_fuel: 1000000000         # ~1B instructions
    max_execution_time_ms: 5000
```

An absent or empty policy means full lockdown — the plugin can do nothing except compute. The policy is the complete, auditable statement of what a plugin is allowed to touch.

## How a CPEX WASM plugin runs

Putting the pieces together, here is what happens when a plugin handles a hook:

```
Host (CPEX)                          Plugin (.wasm)
───────────────────────────────────────────────────
1. Hook arrives at PluginManager
2. WasmPluginFactory resolves the plugin
3. SandboxManager calls handle-hook ──────────────▶
   (WIT marshals the payload)                       4. register_wasm_plugin! dispatches
                                                       to HookHandler<H>
                                                    5. Plugin logic runs (pure compute,
                                                       or uses granted capabilities)
                                    ◀────────────── 6. Returns HookResult via WIT
7. SandboxManager converts result
8. PluginManager continues pipeline
```

Steps 3 and 6 cross the Wasm boundary. The `register_wasm_plugin!` macro in `cpex-wasm-plugin` generates all the code for steps 4 and 6 automatically, so plugin authors only write step 5.

## What this means for plugin authors

- **Isolation is structural, not conventional.** A buggy or malicious plugin cannot corrupt the host process. The worst it can do is exhaust the resources its sandbox policy permits.
- **The API surface is identical to native plugins.** You implement `HookHandler<H>`, return a `PluginResult`, and register with a macro. There is no separate "Wasm plugin API" to learn.
- **Capabilities are declared up front.** If a plugin needs network access, that is visible in its sandbox policy before it ever runs. Reviewers can audit what a plugin can do without reading its source.

## Next steps

- [Plugin SDK]({{< relref "/docs/wasm/plugin" >}}): implement and build a WASM plugin.
- [Host Runtime]({{< relref "/docs/wasm/host" >}}): configure sandboxes, resource limits, and the `WasmPluginFactory`.
- [WASM Plugins overview]({{< relref "/docs/wasm" >}}): architecture diagram and quick-start checklist.
