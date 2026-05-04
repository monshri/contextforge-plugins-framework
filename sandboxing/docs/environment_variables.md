# Environment Variables Sandboxing

## Overview

The sandboxing framework provides fine-grained control over which environment variables are accessible to WASM plugins. This is a critical security feature that prevents plugins from accessing sensitive information stored in environment variables.

## Configuration

Environment variables are controlled through the `allowed_env_vars` field in the policy configuration file (`config/policy.yaml`):

```yaml
plugin:
  name: "samplePlugin"
  sandbox:
    type: "wasm"
    wasm_version: "p2"
    policy:
      allowed_env_vars:
        - "PATH"
        - "HOME"
        - "USER"
        - "RUST_LOG"
```

## Allowlist Rules

### Empty List (Default)
If `allowed_env_vars` is empty or not specified, **no environment variables** are exposed to the plugin:

```yaml
allowed_env_vars: []  # No env vars accessible
```

### Wildcard (Not Recommended)
Use `"*"` to allow **all environment variables**. This is not recommended for production as it may expose sensitive data:

```yaml
allowed_env_vars:
  - "*"  # All env vars accessible (use with caution!)
```

### Specific Variables (Recommended)
List only the environment variables that the plugin needs:

```yaml
allowed_env_vars:
  - "PATH"
  - "HOME"
  - "USER"
  - "LANG"
  - "TZ"
```

## Security Considerations

### What to Allow
- **Safe variables**: PATH, HOME, USER, LANG, TZ
- **Application-specific**: Variables your plugin explicitly needs
- **Non-sensitive**: Variables that don't contain secrets or credentials

### What to Deny
- **Credentials**: API keys, tokens, passwords
- **Secrets**: Database URLs, encryption keys
- **System info**: Variables that reveal system architecture or configuration
- **Session data**: Authentication tokens, session IDs

### Examples of Sensitive Variables to Avoid
```yaml
# ❌ DO NOT ALLOW THESE
allowed_env_vars:
  - "AWS_SECRET_ACCESS_KEY"
  - "DATABASE_URL"
  - "API_KEY"
  - "SECRET_KEY"
  - "PASSWORD"
  - "TOKEN"
```

## Implementation Details

### Host Side (Rust)

The environment variable filtering is implemented in `src/policy_loader.rs`:

```rust
fn configure_env_vars(builder: &mut WasiCtxBuilder, allowed: &[String]) {
    if allowed.is_empty() {
        // No environment variables allowed
        return;
    }

    if allowed.iter().any(|s| s == "*") {
        // Inherit all environment variables
        builder.inherit_env();
        return;
    }

    // Only expose specifically allowed environment variables
    for var_name in allowed {
        if let Ok(value) = std::env::var(var_name) {
            builder.env(var_name, &value);
        }
    }
}
```

### Plugin Side (WASM)

Plugins can access environment variables using standard Rust APIs:

```rust
fn get_env_var(var_name: String) -> String {
    match std::env::var(&var_name) {
        Ok(value) => format!("{}={}", var_name, value),
        Err(_) => format!("Environment variable '{}' not found or not allowed", var_name),
    }
}
```

## Testing

### Running Tests

```bash
cd sandboxing
cargo test
```

### Manual Testing

The example in `src/main.rs` demonstrates environment variable access:

```bash
cd sandboxing
cargo run --release
```

Expected output:
```
--- Testing environment variable access ---
Allowed env var (PATH): PATH=/usr/local/bin:/usr/bin:/bin
Allowed env var (USER): USER=username
Denied env var (SECRET_KEY): Environment variable 'SECRET_KEY' not found or not allowed
Denied env var (SHELL): Environment variable 'SHELL' not found or not allowed
```

## Best Practices

1. **Principle of Least Privilege**: Only allow the minimum set of environment variables needed
2. **Explicit Allowlist**: Never use wildcards in production
3. **Regular Audits**: Review allowed variables periodically
4. **Documentation**: Document why each variable is needed
5. **Testing**: Test both allowed and denied scenarios

## Example Configurations

### Minimal (Most Secure)
```yaml
allowed_env_vars: []  # No environment variables
```

### Basic Application
```yaml
allowed_env_vars:
  - "PATH"
  - "HOME"
  - "USER"
```

### Development Environment
```yaml
allowed_env_vars:
  - "PATH"
  - "HOME"
  - "USER"
  - "RUST_LOG"
  - "RUST_BACKTRACE"
```

### Web Service Plugin
```yaml
allowed_env_vars:
  - "PATH"
  - "TZ"
  - "LANG"
  - "SERVICE_NAME"  # Application-specific
```

## Troubleshooting

### Plugin Can't Access Expected Variable

**Problem**: Plugin reports "Environment variable not found or not allowed"

**Solution**: Add the variable to `allowed_env_vars` in `config/policy.yaml`

### Too Many Variables Exposed

**Problem**: Security audit shows unnecessary variables are accessible

**Solution**: Remove unused variables from the allowlist and test thoroughly

### Variable Value is Empty

**Problem**: Variable is in allowlist but returns empty

**Solution**: Ensure the variable exists in the host environment before starting the plugin

## Related Features

- [HTTP Allowlist](./limitations_sandboxing.md#http-allowlist)
- [Filesystem Permissions](./limitations_sandboxing.md#filesystem-access)
- [WASI Sandboxing](./limitations_sandboxing.md)