# Resource Limits for Plugin Sandboxing

## Overview

The sandboxing framework now supports comprehensive resource limits per plugin, including CPU, memory, and execution time constraints. This prevents plugins from consuming excessive resources and ensures fair resource allocation across multiple plugins.

## Configuration

Resource limits are configured in the `policy.yaml` file under the `resource_limits` section:

```yaml
plugin:
  name: "samplePlugin"
  sandbox:
    policy:
      # ... other policies ...
      
      resource_limits:
        # Maximum memory in bytes (e.g., 128MB = 134217728 bytes)
        # null means no limit
        max_memory_bytes: 134217728
        
        # CPU execution timeout in milliseconds
        # Uses Wasmtime's fuel metering to limit CPU instructions
        cpu_timeout_ms: 5000
        
        # Wall clock timeout in milliseconds
        # Maximum real-world time the plugin can run
        wall_clock_timeout_ms: 10000
        
        # Maximum fuel units for CPU metering
        # Fuel is consumed per WASM instruction executed
        # Higher values allow more computation
        max_fuel: 1000000
```

## Resource Limit Types

### 1. Memory Limits (`max_memory_bytes`)

Controls the maximum amount of memory a plugin can allocate.

- **Type**: Optional `u64` (bytes)
- **Default**: No limit
- **Example**: `134217728` (128 MB)
- **Enforcement**: Configured at Wasmtime engine level

**Use Cases:**
- Prevent memory exhaustion attacks
- Ensure fair memory allocation in multi-tenant environments
- Protect against memory leaks in plugins

### 2. CPU Fuel Limits (`max_fuel`)

Limits the number of WASM instructions a plugin can execute using Wasmtime's fuel metering system.

- **Type**: Optional `u64` (fuel units)
- **Default**: No limit
- **Example**: `1000000` (1 million instructions)
- **Enforcement**: Tracked per Store instance

**How Fuel Works:**
- Each WASM instruction consumes fuel
- When fuel reaches zero, execution is interrupted
- Provides deterministic CPU usage control

**Use Cases:**
- Prevent infinite loops
- Limit computational complexity
- Ensure predictable execution costs

### 3. CPU Timeout (`cpu_timeout_ms`)

Alternative to fuel-based limiting, provides a time-based CPU constraint.

- **Type**: Optional `u64` (milliseconds)
- **Default**: No limit
- **Example**: `5000` (5 seconds)
- **Enforcement**: Can be used alongside fuel limits

**Use Cases:**
- Time-boxed operations
- Simpler configuration than fuel units
- User-friendly timeout specification

### 4. Wall Clock Timeout (`wall_clock_timeout_ms`)

Limits the total real-world time a plugin can run, including I/O wait time.

- **Type**: Optional `u64` (milliseconds)
- **Default**: No limit
- **Example**: `10000` (10 seconds)
- **Enforcement**: Uses Wasmtime's epoch interruption mechanism

**Use Cases:**
- Prevent plugins from hanging indefinitely
- Limit total execution time including network/disk I/O
- Timeout for long-running operations

## Implementation Details

### Architecture

```
┌─────────────────────────────────────────────────────────┐
│                     policy.yaml                         │
│  ┌───────────────────────────────────────────────────┐ │
│  │ resource_limits:                                  │ │
│  │   max_memory_bytes: 134217728                     │ │
│  │   max_fuel: 1000000                               │ │
│  │   cpu_timeout_ms: 5000                            │ │
│  │   wall_clock_timeout_ms: 10000                    │ │
│  └───────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────────┐
│              policy_loader.rs                           │
│  ┌───────────────────────────────────────────────────┐ │
│  │ struct ResourceLimits {                           │ │
│  │   max_memory_bytes: Option<u64>,                  │ │
│  │   cpu_timeout_ms: Option<u64>,                    │ │
│  │   wall_clock_timeout_ms: Option<u64>,             │ │
│  │   max_fuel: Option<u64>,                          │ │
│  │ }                                                  │ │
│  └───────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────────┐
│                    main.rs                              │
│  ┌───────────────────────────────────────────────────┐ │
│  │ 1. Configure Wasmtime Engine                      │ │
│  │    - Enable fuel metering                         │ │
│  │    - Enable epoch interruption                    │ │
│  │                                                    │ │
│  │ 2. Initialize Store with limits                   │ │
│  │    - Set fuel limit                               │ │
│  │    - Set epoch deadline                           │ │
│  │    - Start timeout ticker                         │ │
│  │                                                    │ │
│  │ 3. Monitor resource usage                         │ │
│  │    - Track fuel consumption                       │ │
│  │    - Track elapsed time                           │ │
│  │    - Warn on approaching limits                   │ │
│  └───────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────────┐
│                   lib.rs (MyState)                      │
│  ┌───────────────────────────────────────────────────┐ │
│  │ struct MyState {                                  │ │
│  │   resource_limits: ResourceLimits,                │ │
│  │   start_time: Instant,                            │ │
│  │   // ... other fields                             │ │
│  │ }                                                  │ │
│  └───────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────┘
```

### Key Components

1. **ResourceLimits Struct** (`policy_loader.rs`)
   - Defines the structure for resource limit configuration
   - Provides default values (no limits)
   - Serializable from YAML

2. **MyState Extension** (`lib.rs`)
   - Stores resource limits per plugin instance
   - Tracks start time for wall clock timeout
   - Accessible during plugin execution

3. **Engine Configuration** (`main.rs`)
   - Enables fuel metering when fuel limits are set
   - Enables epoch interruption for timeouts
   - Configures memory limits

4. **Store Initialization** (`main.rs`)
   - Sets initial fuel amount
   - Configures epoch deadline
   - Starts background timeout ticker

5. **Resource Monitoring** (`main.rs`)
   - `print_resource_usage()` function
   - Displays fuel consumption
   - Shows elapsed time
   - Warns when approaching limits

## Usage Examples

### Example 1: High-Trust Plugin

For trusted plugins that need generous resources:

```yaml
resource_limits:
  max_memory_bytes: 536870912  # 512 MB
  max_fuel: 10000000           # 10 million instructions
  cpu_timeout_ms: 30000        # 30 seconds
  wall_clock_timeout_ms: 60000 # 1 minute
```

### Example 2: Low-Trust Plugin

For untrusted plugins with strict limits:

```yaml
resource_limits:
  max_memory_bytes: 67108864   # 64 MB
  max_fuel: 100000             # 100k instructions
  cpu_timeout_ms: 1000         # 1 second
  wall_clock_timeout_ms: 2000  # 2 seconds
```

### Example 3: I/O-Heavy Plugin

For plugins that do mostly I/O with minimal computation:

```yaml
resource_limits:
  max_memory_bytes: 134217728  # 128 MB
  max_fuel: 500000             # 500k instructions
  cpu_timeout_ms: 2000         # 2 seconds
  wall_clock_timeout_ms: 30000 # 30 seconds (allow time for I/O)
```

### Example 4: No Limits

For development or trusted environments:

```yaml
resource_limits:
  max_memory_bytes: null
  max_fuel: null
  cpu_timeout_ms: null
  wall_clock_timeout_ms: null
```

## Monitoring and Debugging

### Resource Usage Output

When running a plugin, resource usage is displayed after each operation:

```
=== Resource Limits Active ===
Max Memory: 134217728 bytes (128 MB)
Max Fuel: 1000000 units
CPU Timeout: 5000ms
Wall Clock Timeout: 10000ms
==============================

--- Calling check_key ---
Result: {"action":"allow"}
  Fuel: consumed=1234, remaining=998766
  Elapsed time: 2.45ms

--- Calling create_file ---
Result: File created successfully
  Fuel: consumed=5678, remaining=994322
  Elapsed time: 15.32ms
  ⚠️  WARNING: Approaching wall clock timeout!
```

### Handling Limit Violations

When a resource limit is exceeded, the plugin execution is interrupted:

- **Fuel exhaustion**: Returns a trap error
- **Epoch timeout**: Returns an interrupt error
- **Memory limit**: Allocation fails within the WASM module

Example error handling:

```rust
match plugin.call_function(&mut store).await {
    Ok(result) => println!("Success: {}", result),
    Err(e) => {
        if e.to_string().contains("fuel") {
            eprintln!("Plugin exceeded CPU fuel limit");
        } else if e.to_string().contains("interrupt") {
            eprintln!("Plugin exceeded timeout limit");
        } else {
            eprintln!("Plugin error: {}", e);
        }
    }
}
```

## Best Practices

### 1. Start Conservative

Begin with strict limits and relax them based on actual usage:

```yaml
resource_limits:
  max_memory_bytes: 67108864   # 64 MB
  max_fuel: 100000
  wall_clock_timeout_ms: 5000
```

### 2. Monitor in Production

Track resource usage patterns to optimize limits:

- Log fuel consumption per operation
- Track execution times
- Identify resource-intensive operations

### 3. Different Limits for Different Plugins

Use separate policy files for different plugin types:

- `policy-compute.yaml` - High CPU, low I/O
- `policy-io.yaml` - Low CPU, high I/O wait
- `policy-balanced.yaml` - Moderate all resources

### 4. Test Limit Violations

Ensure your application handles limit violations gracefully:

```rust
#[test]
fn test_fuel_limit_exceeded() {
    // Set very low fuel limit
    let policy = load_policy("test-policies/low-fuel.yaml")?;
    // Attempt expensive operation
    // Verify proper error handling
}
```

### 5. Document Plugin Requirements

Provide guidance to plugin developers:

```markdown
## Plugin Resource Requirements

Your plugin should operate within these limits:
- Memory: < 100 MB
- Execution time: < 5 seconds
- Fuel consumption: < 500k instructions

Test your plugin with these limits before deployment.
```

## Integration with Other Policies

Resource limits work alongside other sandbox policies:

```yaml
plugin:
  sandbox:
    policy:
      # Filesystem access
      dir_name: ["./data"]
      permissions:
        dir: ["read"]
        file: ["read"]
      
      # Network access
      allowed_hosts:
        - "api.example.com"
      
      # Resource limits
      resource_limits:
        max_memory_bytes: 134217728
        max_fuel: 1000000
        wall_clock_timeout_ms: 10000
      
      # Clock access
      clock_policy:
        allow_monotonic_clock: true
        allow_wall_clock: true
```

## Performance Considerations

### Fuel Metering Overhead

- Fuel metering adds ~10-20% overhead
- Only enable when limits are needed
- Consider disabling for trusted plugins

### Epoch Interruption

- Minimal overhead when not triggered
- Efficient for timeout enforcement
- Requires background task for ticker

### Memory Limits

- Enforced at allocation time
- No runtime overhead
- May cause allocation failures

## Troubleshooting

### Plugin Runs Slower Than Expected

**Cause**: Fuel metering overhead

**Solution**: 
- Increase fuel limit
- Disable fuel metering for trusted plugins
- Optimize plugin code

### Plugin Times Out Unexpectedly

**Cause**: Wall clock timeout too strict

**Solution**:
- Increase `wall_clock_timeout_ms`
- Profile plugin to identify slow operations
- Consider separate limits for I/O operations

### Memory Allocation Failures

**Cause**: Memory limit too low

**Solution**:
- Increase `max_memory_bytes`
- Optimize plugin memory usage
- Use streaming for large data

## Future Enhancements

Potential improvements to the resource limit system:

1. **Dynamic Limits**: Adjust limits based on system load
2. **Resource Pools**: Share resources across multiple plugins
3. **Priority Levels**: Different limits for different priority plugins
4. **Detailed Metrics**: Per-operation resource tracking
5. **Quota Systems**: Cumulative resource usage over time

## References

- [Wasmtime Fuel Metering](https://docs.wasmtime.dev/api/wasmtime/struct.Store.html#method.set_fuel)
- [Wasmtime Epoch Interruption](https://docs.wasmtime.dev/api/wasmtime/struct.Store.html#method.set_epoch_deadline)
- [WASM Memory Model](https://webassembly.github.io/spec/core/syntax/modules.html#memories)