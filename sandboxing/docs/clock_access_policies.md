# Clock Access Policies

## Overview

Clock access policies control how plugins interact with system time through the WASI clocks interface. This includes both monotonic clocks (for measuring elapsed time) and wall clocks (for getting current date/time).

## Why Control Clock Access?

### Security Concerns

1. **Timing Attacks**: High-precision clocks enable side-channel attacks
2. **Fingerprinting**: System time can identify users/systems
3. **Privacy**: Real date/time reveals information about user activity
4. **Resource Abuse**: Excessive time queries waste CPU cycles

### Use Cases for Clock Policies

- **High-security environments**: Prevent timing side-channels
- **Privacy-sensitive applications**: Hide real system time
- **Testing/simulation**: Use virtual time for deterministic tests
- **Resource management**: Limit excessive time polling

## Configuration

Clock policies are configured in `config/policy.yaml` under `clock_policy`:

```yaml
plugin:
  sandbox:
    policy:
      clock_policy:
        allow_monotonic_clock: true
        allow_wall_clock: true
        min_resolution_ns: 1000000
        max_queries_per_second: 10000
        time_offset_seconds: null
```

## Policy Options

### 1. Allow Monotonic Clock

Control access to monotonic clock (elapsed time measurement):

```yaml
clock_policy:
  allow_monotonic_clock: true  # Default: true
```

**Monotonic clock provides:**
- Elapsed time measurement
- Timeouts and delays
- Performance benchmarking
- Not affected by system time changes

**When to disable:**
- Plugin doesn't need timing
- Prevent timing attacks
- Maximum security posture

**Impact of disabling:**
- ❌ No `std::time::Instant`
- ❌ No timeouts
- ❌ No sleep/delays
- ❌ No performance measurement

### 2. Allow Wall Clock

Control access to wall clock (current date/time):

```yaml
clock_policy:
  allow_wall_clock: true  # Default: true
```

**Wall clock provides:**
- Current date and time
- Unix timestamps
- Time zone information
- Calendar operations

**When to disable:**
- Privacy requirements
- Prevent fingerprinting
- Hide user activity patterns
- Testing with fixed time

**Impact of disabling:**
- ❌ No `std::time::SystemTime`
- ❌ No current date/time
- ❌ No timestamps
- ❌ Time-based logic fails

### 3. Minimum Resolution

Reduce timing precision by rounding to a minimum resolution:

```yaml
clock_policy:
  min_resolution_ns: 1000000  # 1 millisecond
```

**Common values:**
- `1000` (1 microsecond) - High precision
- `1000000` (1 millisecond) - Standard precision
- `10000000` (10 milliseconds) - Low precision
- `1000000000` (1 second) - Very low precision
- `null` - No rounding (full precision)

**How it works:**
```
Real time:    1234567890 ns
Resolution:   1000000 ns (1ms)
Rounded time: 1234000000 ns
```

**Use cases:**
- Prevent high-precision timing attacks
- Reduce timing side-channels
- Limit fingerprinting accuracy
- Balance security vs functionality

### 4. Rate Limiting

Limit the number of clock queries per second:

```yaml
clock_policy:
  max_queries_per_second: 10000  # 10k queries/sec
```

**Common values:**
- `100` - Very restrictive
- `1000` - Moderate restriction
- `10000` - Light restriction
- `null` - No limit

**Use cases:**
- Prevent resource exhaustion
- Detect suspicious behavior
- Limit CPU usage
- Prevent timing attack attempts

**Note:** Actual enforcement requires tracking query counts in the host implementation.

### 5. Time Offset

Add an offset to all wall clock readings:

```yaml
clock_policy:
  time_offset_seconds: 3600  # Add 1 hour
```

**Values:**
- Positive number: Add seconds to real time
- Negative number: Subtract seconds from real time
- `null`: Use real time (no offset)

**Use cases:**
- **Testing**: Simulate different times
- **Privacy**: Hide real system time
- **Time zones**: Adjust for different zones
- **Deterministic execution**: Fixed time for tests

**Examples:**
```yaml
# Add 1 hour
time_offset_seconds: 3600

# Subtract 1 day
time_offset_seconds: -86400

# No offset (real time)
time_offset_seconds: null
```

## Complete Examples

### Example 1: Default (Permissive)

Allow full clock access with minimal restrictions:

```yaml
clock_policy:
  allow_monotonic_clock: true
  allow_wall_clock: true
  min_resolution_ns: null      # Full precision
  max_queries_per_second: null # No limit
  time_offset_seconds: null    # Real time
```

**Use for:** Most applications, development, testing

### Example 2: Privacy-Focused

Hide real time, reduce precision:

```yaml
clock_policy:
  allow_monotonic_clock: true
  allow_wall_clock: false       # No real date/time
  min_resolution_ns: 10000000   # 10ms resolution
  max_queries_per_second: 1000
  time_offset_seconds: null
```

**Use for:** Privacy-sensitive apps, anonymous systems

### Example 3: High Security

Minimal clock access, low precision:

```yaml
clock_policy:
  allow_monotonic_clock: true
  allow_wall_clock: false
  min_resolution_ns: 1000000000  # 1 second resolution
  max_queries_per_second: 100
  time_offset_seconds: null
```

**Use for:** High-security environments, untrusted plugins

### Example 4: Testing/Simulation

Fixed time for deterministic tests:

```yaml
clock_policy:
  allow_monotonic_clock: true
  allow_wall_clock: true
  min_resolution_ns: null
  max_queries_per_second: null
  time_offset_seconds: -86400  # Yesterday
```

**Use for:** Automated testing, simulations

### Example 5: No Clock Access

Complete clock denial:

```yaml
clock_policy:
  allow_monotonic_clock: false
  allow_wall_clock: false
  min_resolution_ns: null
  max_queries_per_second: null
  time_offset_seconds: null
```

**Use for:** Plugins that don't need time, maximum security

## Implementation Details

### Policy Functions

The clock policy is enforced through helper functions:

```rust
// Round timestamp to minimum resolution
pub fn apply_clock_resolution(
    timestamp_ns: u64,
    min_resolution_ns: Option<u64>
) -> u64

// Apply time offset
pub fn apply_time_offset(
    timestamp_seconds: u64,
    offset_seconds: Option<i64>
) -> u64

// Check rate limiting
pub fn should_rate_limit_clock_query(
    queries_this_second: u32,
    max_queries_per_second: Option<u32>
) -> bool
```

### How It Works

```
Plugin calls time API
        ↓
WASI clock layer
        ↓
Check allow_monotonic_clock / allow_wall_clock
        ↓
Get real time
        ↓
Apply resolution rounding
        ↓
Apply time offset
        ↓
Check rate limit
        ↓
Return time to plugin
```

### Performance

- **Resolution rounding**: ~5 nanoseconds overhead
- **Time offset**: ~3 nanoseconds overhead
- **Rate limiting**: ~10 nanoseconds overhead
- **Total overhead**: <20 nanoseconds per query

## Testing

### Unit Tests

Run clock policy tests:

```bash
cd sandboxing
cargo test --lib policy_loader::test_clock
cargo test --lib policy_loader::test_time
cargo test --lib policy_loader::test_rate
```

Tests cover:
- ✅ Clock resolution rounding
- ✅ Positive time offsets
- ✅ Negative time offsets
- ✅ No offset (passthrough)
- ✅ Rate limiting logic

### Integration Testing

Test with actual plugin:

```rust
// In plugin code
use std::time::{Instant, SystemTime};

// Monotonic clock (if allowed)
let start = Instant::now();
// ... do work ...
let elapsed = start.elapsed();

// Wall clock (if allowed)
let now = SystemTime::now();
let timestamp = now.duration_since(UNIX_EPOCH)?;
```

## Security Best Practices

### 1. Reduce Precision for Untrusted Code

```yaml
# ❌ Too precise (enables timing attacks)
min_resolution_ns: null

# ✅ Reasonable precision
min_resolution_ns: 1000000  # 1ms
```

### 2. Disable Wall Clock for Privacy

```yaml
# ✅ Privacy-focused
allow_wall_clock: false
allow_monotonic_clock: true  # Still allow timeouts
```

### 3. Use Rate Limiting

```yaml
# ✅ Prevent abuse
max_queries_per_second: 10000
```

### 4. Consider Time Offset for Testing

```yaml
# ✅ Deterministic tests
time_offset_seconds: -86400  # Fixed "yesterday"
```

### 5. Document Why Clocks Are Needed

```yaml
# Document in comments
clock_policy:
  allow_wall_clock: true  # Needed for log timestamps
  min_resolution_ns: 1000000  # 1ms sufficient for logging
```

## Common Use Cases

### Web Service Plugin

```yaml
clock_policy:
  allow_monotonic_clock: true  # For timeouts
  allow_wall_clock: true       # For timestamps
  min_resolution_ns: 1000000   # 1ms precision
  max_queries_per_second: 10000
  time_offset_seconds: null
```

### Data Processing Plugin

```yaml
clock_policy:
  allow_monotonic_clock: true  # For performance metrics
  allow_wall_clock: false      # No need for real time
  min_resolution_ns: 1000000
  max_queries_per_second: 1000
  time_offset_seconds: null
```

### Anonymous Plugin

```yaml
clock_policy:
  allow_monotonic_clock: true
  allow_wall_clock: false      # Privacy: hide real time
  min_resolution_ns: 10000000  # 10ms (low precision)
  max_queries_per_second: 100
  time_offset_seconds: null
```

### Testing Plugin

```yaml
clock_policy:
  allow_monotonic_clock: true
  allow_wall_clock: true
  min_resolution_ns: null      # Full precision for tests
  max_queries_per_second: null
  time_offset_seconds: 0       # Fixed epoch time
```

## Troubleshooting

### Plugin Can't Get Time

**Problem**: Time queries return errors

**Solution**: Enable clock access:
```yaml
clock_policy:
  allow_monotonic_clock: true
  allow_wall_clock: true
```

### Time Seems Rounded/Inaccurate

**Problem**: Timestamps are rounded to coarse values

**Solution**: Reduce minimum resolution:
```yaml
clock_policy:
  min_resolution_ns: 1000  # 1 microsecond
```

### Rate Limit Errors

**Problem**: "Too many clock queries" errors

**Solution**: Increase rate limit:
```yaml
clock_policy:
  max_queries_per_second: 100000  # Increase limit
```

### Wrong Time Returned

**Problem**: Time is offset from real time

**Solution**: Remove time offset:
```yaml
clock_policy:
  time_offset_seconds: null  # Use real time
```

## Comparison with Other Policies

| Policy Type | Scope | Complexity | Security Impact |
|-------------|-------|------------|-----------------|
| **Clock** | Time access | Low | Medium |
| **Network** | HTTP/Sockets | High | High |
| **Filesystem** | File I/O | Medium | High |
| **Environment** | Env vars | Low | Medium |

**Clock policies are:**
- ✅ Easy to configure
- ✅ Low performance overhead
- ✅ Effective against timing attacks
- ✅ Good for privacy

## Related Documentation

- [Socket-Level Filtering](./socket_level_filtering.md)
- [IP Address Filtering](./ip_address_filtering.md)
- [Environment Variables](./environment_variables.md)
- [WASI Sandboxing](./limitations_sandboxing.md)