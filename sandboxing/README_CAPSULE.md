# Capsule-Core Integration Guide

This document explains how to use the `capsule-core` library for WASM sandboxing in this project.

## Overview

[Capsule](https://github.com/capsulerun/capsule) is a WASM runtime that provides secure sandboxing with fine-grained resource controls. The `capsule-core` crate provides the core functionality for:

- **Resource Limiting**: Control CPU (fuel), memory, and execution time
- **File System Isolation**: Restrict file access to specific directories
- **Network Control**: Allow/deny network access to specific hosts
- **Environment Variables**: Control which environment variables are accessible

## Installation

The dependency is already added to `Cargo.toml`:

```toml
[dependencies]
capsule-core = { git = "https://github.com/capsulerun/capsule.git", branch = "main" }
```

## Quick Start

### 1. Running the Example

```bash
cargo run --example capsule_example
```

This demonstrates:
- Creating execution policies with different security levels
- Configuring resource limits (CPU, RAM, timeout)
- Setting file and network access controls

### 2. Basic Usage

```rust
use capsule_core::wasm::execution_policy::{ExecutionPolicy, Compute};

// Create a basic execution policy
let policy = ExecutionPolicy::new()
    .name(Some("my-policy".to_string()))
    .compute(Some(Compute::Medium))      // 2 billion fuel units
    .ram(Some(128 * 1024 * 1024))        // 128 MB RAM limit
    .timeout(Some("30s".to_string()))    // 30 second timeout
    .max_retries(Some(3))                // Retry up to 3 times
    .allowed_files(vec![
        "/tmp/data".to_string(),
    ])
    .allowed_hosts(vec![
        "api.example.com".to_string(),
    ])
    .env_variables(vec![
        "API_KEY".to_string(),
    ]);
```

## Execution Policy Configuration

### Compute Levels

Control CPU resources using fuel units:

| Level | Fuel Units | Use Case |
|-------|------------|----------|
| `Compute::Low` | 100,000,000 | Quick operations, untrusted code |
| `Compute::Medium` | 2,000,000,000 | Standard operations |
| `Compute::High` | 50,000,000,000 | Complex computations |
| `Compute::Custom(n)` | Custom amount | Fine-tuned control |

```rust
// Use predefined levels
.compute(Some(Compute::Low))

// Or set custom fuel
.compute(Some(Compute::Custom(5_000_000_000)))
```

### Memory Limits

Set RAM limits in bytes:

```rust
// 32 MB
.ram(Some(32 * 1024 * 1024))

// 128 MB
.ram(Some(128 * 1024 * 1024))

// 512 MB
.ram(Some(512 * 1024 * 1024))
```

### Timeouts

Use human-readable duration strings:

```rust
.timeout(Some("5s".to_string()))    // 5 seconds
.timeout(Some("30s".to_string()))   // 30 seconds
.timeout(Some("2m".to_string()))    // 2 minutes
.timeout(Some("1h".to_string()))    // 1 hour
```

### File System Access

Restrict file access to specific paths:

```rust
.allowed_files(vec![
    "/tmp/data".to_string(),
    "/app/config".to_string(),
    "/var/log/app".to_string(),
])
```

### Network Access

Control which hosts can be accessed:

```rust
.allowed_hosts(vec![
    "api.example.com".to_string(),
    "*.trusted-domain.com".to_string(),  // Wildcard support
    "192.168.1.100".to_string(),         // IP addresses
])
```

### Environment Variables

Specify which environment variables are accessible:

```rust
.env_variables(vec![
    "API_KEY".to_string(),
    "DATABASE_URL".to_string(),
    "LOG_LEVEL".to_string(),
])
```

## Security Profiles

### Restrictive (Untrusted Code)

```rust
let restrictive = ExecutionPolicy::new()
    .name(Some("restrictive".to_string()))
    .compute(Some(Compute::Low))
    .ram(Some(32 * 1024 * 1024))
    .timeout(Some("5s".to_string()))
    .max_retries(Some(0))
    .allowed_files(vec![])      // No file access
    .allowed_hosts(vec![])      // No network access
    .env_variables(vec![]);     // No environment variables
```

### Balanced (Standard Operations)

```rust
let balanced = ExecutionPolicy::new()
    .name(Some("balanced".to_string()))
    .compute(Some(Compute::Medium))
    .ram(Some(128 * 1024 * 1024))
    .timeout(Some("30s".to_string()))
    .max_retries(Some(3))
    .allowed_files(vec!["/tmp/data".to_string()])
    .allowed_hosts(vec!["api.example.com".to_string()])
    .env_variables(vec!["API_KEY".to_string()]);
```

### Permissive (Trusted Code)

```rust
let permissive = ExecutionPolicy::new()
    .name(Some("permissive".to_string()))
    .compute(Some(Compute::High))
    .ram(Some(512 * 1024 * 1024))
    .timeout(Some("5m".to_string()))
    .max_retries(Some(5))
    .allowed_files(vec![
        "/data".to_string(),
        "/tmp".to_string(),
        "/app".to_string(),
    ])
    .allowed_hosts(vec!["*".to_string()])  // All hosts (use with caution!)
    .env_variables(vec![
        "API_KEY".to_string(),
        "DATABASE_URL".to_string(),
        "LOG_LEVEL".to_string(),
    ]);
```

## Testing

Run the tests:

```bash
# Run all tests
cargo test

# Run example tests
cargo test --example capsule_example
```

## Integration with Existing Code

The `capsule-core` library can be integrated with your existing Wasmtime-based sandboxing:

1. **Replace manual resource limits** with `ExecutionPolicy`
2. **Use capsule's State** for WASI context management
3. **Leverage built-in security features** like host validation

See `examples/capsule_example.rs` for a complete working example.

## Resources

- [Capsule GitHub Repository](https://github.com/capsulerun/capsule)
- [Capsule Documentation](https://github.com/capsulerun/capsule/tree/main/docs)
- [Wasmtime Documentation](https://docs.wasmtime.dev/)

## Next Steps

1. Review the example: `cargo run --example capsule_example`
2. Adapt the execution policies to your use case
3. Integrate with your existing WASM runtime setup
4. Test with your WASM modules

## Support

For issues or questions:
- Capsule: https://github.com/capsulerun/capsule/issues
- This project: [Your issue tracker]