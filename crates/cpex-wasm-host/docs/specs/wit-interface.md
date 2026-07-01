# WIT Interface Specification

## Overview

The WIT (WebAssembly Interface Types) definition serves as the contract between the host (`cpex-wasm-host`) and the guest (`cpex-wasm-plugin`). Both sides share an identical `world.wit` file under their respective `wit/` directories.

Package: `cpex:plugin`

## World Definition

```wit
world plugin {
  import wasi:io/poll@0.2.6;
  import wasi:io/error@0.2.6;
  import wasi:io/streams@0.2.6;
  import wasi:clocks/monotonic-clock@0.2.6;
  import wasi:http/types@0.2.6;
  import wasi:http/outgoing-handler@0.2.6;

  use types.{message-payload, extensions, plugin-context, plugin-result};

  export handle-hook: func(
    payload: message-payload,
    extensions: extensions,
    ctx: plugin-context
  ) -> plugin-result;
}
```

The guest exports a single function `handle-hook` that the host calls for every hook invocation.

## WASI Imports

Plugins have access to the following WASI capabilities (subject to sandbox policy enforcement):

| Import | Purpose |
|--------|---------|
| `wasi:io/poll` | Async I/O polling |
| `wasi:io/error` | Error handling |
| `wasi:io/streams` | Stream read/write |
| `wasi:clocks/monotonic-clock` | Time measurement |
| `wasi:http/types` | HTTP request/response types |
| `wasi:http/outgoing-handler` | Outbound HTTP requests (filtered by NetworkPolicy) |

## Type Definitions

### Core Types

```wit
enum role { system, developer, user, assistant, tool }
enum channel { analysis, commentary, final }
enum resource-type { file, blob, uri, database, api, memory, artifact }
enum subject-type { user, agent, service, system }
```

### Message Payload

```wit
record message-payload {
  message: message,
}

record message {
  schema-version: string,
  role: role,
  content: list<content-part>,
  channel: option<channel>,
}
```

### Content Parts (Variant)

```wit
variant content-part {
  text(string),
  thinking(string),
  tool-call(tool-call),
  tool-result(tool-result),
  cmf-resource(cmf-resource),
  resource-ref(resource-reference),
  prompt-request(prompt-request),
  prompt-result(prompt-result),
  image(image-source),
  video(video-source),
  audio(audio-source),
  document(document-source),
}
```

### Extensions

```wit
record extensions {
  request: option<request-extension>,
  security: option<security-extension>,
  http: option<http-extension>,
  meta: option<meta-extension>,
}
```

### Plugin Context and Result

```wit
record plugin-context {
  local-state: string,   // JSON-serialized HashMap<String, Value>
  global-state: string,  // JSON-serialized HashMap<String, Value>
}

record plugin-result {
  continue-processing: bool,
  modified-payload: option<message-payload>,
  modified-extensions: option<extensions>,
  violation: option<plugin-violation>,
  metadata: option<string>,  // JSON-serialized
}

record plugin-violation {
  code: string,
  reason: string,
  description: option<string>,
  details: string,  // JSON-serialized HashMap
  proto-error-code: option<s64>,
}
```

## JSON-Serialized Fields

WIT does not support nested maps or arbitrary JSON. The following fields use JSON strings as an escape hatch:

| WIT Field | Native Type | Reason |
|-----------|-------------|--------|
| `tool-call.arguments` | `HashMap<String, Value>` | Arbitrary key-value pairs |
| `tool-result.content` | `serde_json::Value` | Arbitrary JSON content |
| `cmf-resource.annotations` | `HashMap<String, Value>` | Arbitrary metadata |
| `prompt-request.arguments` | `HashMap<String, Value>` | Arbitrary parameters |
| `prompt-result.messages` | `Vec<Message>` | Recursive message list |
| `plugin-context.local-state` | `HashMap<String, Value>` | Plugin state |
| `plugin-context.global-state` | `HashMap<String, Value>` | Shared state |
| `plugin-violation.details` | `HashMap<String, Value>` | Error details |
| `plugin-result.metadata` | `HashMap<String, Value>` | Result metadata |

## Code Generation

### Host side (cpex-wasm-host)

```rust
wasmtime::component::bindgen!({
    path: "wit",
    world: "plugin",
    exports: { default: async },
});
```

Generates: `Plugin` struct with `call_handle_hook()` and all WIT types under `cpex::plugin::types::*`.

### Guest side (cpex-wasm-plugin)

```rust
wit_bindgen::generate!({
    path: "wit",
    world: "plugin",
    generate_all,
});
```

Generates: `Guest` trait that the plugin must implement, all WIT types as Rust structs/enums, and the `export!()` macro for registration.
