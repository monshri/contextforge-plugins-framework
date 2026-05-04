# IP Address Filtering for HTTP Allowlist

## Overview

The sandboxing framework supports fine-grained control over outbound HTTP connections using IP addresses and CIDR ranges in addition to hostnames. This allows you to restrict plugin network access to specific IP addresses or IP ranges.

## Configuration

IP addresses and CIDR ranges are configured in the `allowed_hosts` field of `config/policy.yaml`:

```yaml
plugin:
  name: "samplePlugin"
  sandbox:
    policy:
      allowed_hosts:
        - "httpbin.org"           # Hostname
        - "*.example.com"         # Wildcard hostname
        - "192.168.1.100"         # Specific IPv4 address
        - "192.168.1.0/24"        # IPv4 CIDR range
        - "10.0.0.0/8"            # Large IPv4 range
        - "2001:db8::1"           # Specific IPv6 address
        - "2001:db8::/32"         # IPv6 CIDR range
        - "127.0.0.1"             # Localhost IPv4
        - "::1"                   # Localhost IPv6
```

## Supported Formats

### 1. Exact IPv4 Address
```yaml
allowed_hosts:
  - "192.168.1.100"
```
- Matches only the exact IP address `192.168.1.100`
- Does not match `192.168.1.101` or any other IP

### 2. IPv4 CIDR Range
```yaml
allowed_hosts:
  - "192.168.1.0/24"
```
- Matches all IPs from `192.168.1.0` to `192.168.1.255`
- Common ranges:
  - `/32` - Single IP (e.g., `192.168.1.1/32`)
  - `/24` - 256 IPs (e.g., `192.168.1.0/24`)
  - `/16` - 65,536 IPs (e.g., `192.168.0.0/16`)
  - `/8` - 16,777,216 IPs (e.g., `10.0.0.0/8`)

### 3. Exact IPv6 Address
```yaml
allowed_hosts:
  - "2001:db8::1"
```
- Matches only the exact IPv6 address
- Supports full and compressed IPv6 notation

### 4. IPv6 CIDR Range
```yaml
allowed_hosts:
  - "2001:db8::/32"
```
- Matches all IPs in the IPv6 range
- Common ranges:
  - `/128` - Single IPv6 address
  - `/64` - Standard subnet
  - `/32` - Large allocation

### 5. Mixed Configuration
```yaml
allowed_hosts:
  - "api.example.com"      # Hostname
  - "*.internal.corp"      # Wildcard
  - "192.168.1.100"        # Specific IP
  - "10.0.0.0/8"           # Private network
  - "2001:db8::/32"        # IPv6 range
```

## Use Cases

### 1. Internal Services
Allow access to internal services by IP:
```yaml
allowed_hosts:
  - "192.168.1.50"         # Internal API server
  - "192.168.1.100"        # Internal database proxy
  - "10.0.0.0/8"           # Entire internal network
```

### 2. Cloud Services
Allow specific cloud provider IP ranges:
```yaml
allowed_hosts:
  - "52.0.0.0/8"           # AWS IP range (example)
  - "35.0.0.0/8"           # GCP IP range (example)
```

### 3. Development/Testing
Allow localhost for local testing:
```yaml
allowed_hosts:
  - "127.0.0.1"            # IPv4 localhost
  - "::1"                  # IPv6 localhost
  - "127.0.0.0/8"          # All localhost IPs
```

### 4. Restricted Production
Only allow specific production IPs:
```yaml
allowed_hosts:
  - "203.0.113.10"         # Production API
  - "203.0.113.20"         # Production database proxy
  # No CIDR ranges - maximum security
```

## Security Considerations

### Best Practices

1. **Use Specific IPs When Possible**
   ```yaml
   # ✅ Good - Specific IP
   allowed_hosts:
     - "192.168.1.100"
   
   # ⚠️ Less secure - Broad range
   allowed_hosts:
     - "192.168.0.0/16"
   ```

2. **Avoid Overly Broad Ranges**
   ```yaml
   # ❌ Bad - Too broad
   allowed_hosts:
     - "0.0.0.0/0"          # Allows ALL IPv4 addresses!
   
   # ✅ Good - Specific range
   allowed_hosts:
     - "192.168.1.0/24"
   ```

3. **Document Why Each IP is Allowed**
   ```yaml
   allowed_hosts:
     - "192.168.1.50"       # Internal API server (ticket #1234)
     - "10.0.5.0/24"        # Database subnet (approved by security)
   ```

4. **Regular Audits**
   - Review allowed IPs quarterly
   - Remove IPs that are no longer needed
   - Update documentation when IPs change

### Common Pitfalls

1. **Allowing Private Ranges in Production**
   ```yaml
   # ⚠️ Dangerous in production
   allowed_hosts:
     - "10.0.0.0/8"         # Entire private network
     - "172.16.0.0/12"      # All private IPs
     - "192.168.0.0/16"     # All private IPs
   ```

2. **Forgetting IPv6**
   ```yaml
   # ⚠️ Incomplete - only blocks IPv4
   allowed_hosts:
     - "192.168.1.100"
   # Should also consider IPv6 if applicable
   ```

3. **Using IP Instead of Hostname**
   - IPs can change, hostnames are more stable
   - Use hostnames when possible, IPs when necessary

## Implementation Details

### How It Works

1. **Request Interception**: When a plugin makes an HTTP request, the host intercepts it
2. **Host Extraction**: The target host (hostname or IP) is extracted from the URL
3. **Allowlist Check**: The host is checked against the allowlist:
   - If it's an IP, check against IP patterns and CIDR ranges
   - If it's a hostname, check against hostname patterns
4. **Decision**: Allow or deny the request

### Code Location

The IP filtering logic is in `src/policy_loader.rs`:

```rust
pub fn is_host_allowed(host: &str, allowed: &[String]) -> bool {
    // Checks hostnames, IPs, and CIDR ranges
}

fn is_ip_match(host: &str, pattern: &str) -> bool {
    // Handles IP address and CIDR matching
}
```

### Dependencies

The implementation uses the `ipnet` crate for CIDR range matching:

```toml
[dependencies]
ipnet = "2.10"
```

## Testing

### Unit Tests

Run the IP filtering tests:
```bash
cd sandboxing
cargo test --lib policy_loader
```

Tests cover:
- ✅ Exact IPv4 matching
- ✅ IPv4 CIDR ranges
- ✅ Exact IPv6 matching
- ✅ IPv6 CIDR ranges
- ✅ Mixed hostnames and IPs
- ✅ Localhost IPs

### Integration Testing

Test with actual HTTP requests:
```bash
cd sandboxing
cargo run --release
```

## Examples

### Example 1: Microservices Architecture
```yaml
allowed_hosts:
  - "auth-service.internal"      # Authentication service
  - "192.168.10.50"              # User service
  - "192.168.10.51"              # Order service
  - "192.168.20.0/24"            # Database subnet
```

### Example 2: Multi-Cloud Setup
```yaml
allowed_hosts:
  - "api.example.com"            # Public API
  - "52.1.2.3"                   # AWS service
  - "35.4.5.6"                   # GCP service
  - "13.7.8.9"                   # Azure service
```

### Example 3: Development Environment
```yaml
allowed_hosts:
  - "127.0.0.1"                  # Local services
  - "localhost"                  # Local hostname
  - "192.168.1.0/24"             # Local network
  - "*.local"                    # mDNS services
```

### Example 4: Locked-Down Production
```yaml
allowed_hosts:
  - "203.0.113.10"               # Production API only
  # No wildcards, no ranges - maximum security
```

## Troubleshooting

### Plugin Can't Connect to IP

**Problem**: Request to IP address is denied

**Solution**: Add the IP or CIDR range to `allowed_hosts`:
```yaml
allowed_hosts:
  - "192.168.1.100"  # Add specific IP
```

### CIDR Range Not Working

**Problem**: IP within range is still denied

**Solution**: Verify CIDR notation is correct:
```yaml
# ✅ Correct
allowed_hosts:
  - "192.168.1.0/24"

# ❌ Wrong
allowed_hosts:
  - "192.168.1.0-255"  # Not valid CIDR
```

### IPv6 Not Matching

**Problem**: IPv6 address not recognized

**Solution**: Ensure proper IPv6 format:
```yaml
# ✅ Correct formats
allowed_hosts:
  - "2001:db8::1"           # Compressed
  - "2001:0db8:0000:0000:0000:0000:0000:0001"  # Full
  - "2001:db8::/32"         # CIDR

# ❌ Wrong
allowed_hosts:
  - "2001:db8:1"            # Invalid format
```

## Related Documentation

- [Environment Variables](./environment_variables.md)
- [HTTP Allowlist](./limitations_sandboxing.md#http-allowlist)
- [WASI Sandboxing](./limitations_sandboxing.md)