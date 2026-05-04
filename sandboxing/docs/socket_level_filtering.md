# Socket-Level Network Filtering

## Overview

Socket-level filtering provides fine-grained control over TCP and UDP network access at the socket layer. This goes beyond HTTP-level filtering to control all network protocols, specific ports, and connection types.

## Why Socket-Level Filtering?

**HTTP-level filtering** (via `allowed_hosts`) is great for:
- ✅ Simple HTTP/HTTPS requests
- ✅ Hostname-based policies
- ✅ Easy configuration

**Socket-level filtering** adds control for:
- ✅ **All protocols**: TCP, UDP, not just HTTP
- ✅ **Port-specific rules**: Allow only specific ports per IP
- ✅ **Protocol separation**: Different rules for TCP vs UDP
- ✅ **Connection control**: Bind, listen, connect operations
- ✅ **Advanced restrictions**: Privileged ports, connection limits

## Configuration

Socket policies are configured in `config/policy.yaml` under `socket_policy`:

```yaml
plugin:
  sandbox:
    policy:
      socket_policy:
        tcp:
          allowed_destinations:
            - ip: "192.168.1.0/24"
              ports: [80, 443, 8080]
            - ip: "10.0.0.50"
              ports: [5432, 5433]
          max_connections: 50
        
        udp:
          allowed_destinations:
            - ip: "8.8.8.8"
              ports: [53]
          max_packet_size: 1500
        
        restrictions:
          block_privileged_ports: true
          allow_bind: true
          allow_listen: false
```

## TCP Policy

### Allowed Destinations

Control which TCP connections are permitted:

```yaml
tcp:
  allowed_destinations:
    # Specific IP + specific ports
    - ip: "192.168.1.100"
      ports: [80, 443]
    
    # CIDR range + specific ports
    - ip: "10.0.0.0/8"
      ports: [5432, 5433, 6379]
    
    # Specific IP + all ports (empty ports list)
    - ip: "127.0.0.1"
      ports: []
```

**Rules:**
- Empty `allowed_destinations` = **deny all TCP**
- Empty `ports` list = **all ports allowed** for that IP
- IP can be exact address or CIDR range
- Supports both IPv4 and IPv6

### Max Connections

Limit concurrent TCP connections:

```yaml
tcp:
  max_connections: 50  # Maximum 50 concurrent TCP connections
```

- `null` or omitted = no limit
- Prevents resource exhaustion
- Applies to all TCP connections combined

## UDP Policy

### Allowed Destinations

Control which UDP communications are permitted:

```yaml
udp:
  allowed_destinations:
    # DNS servers
    - ip: "8.8.8.8"
      ports: [53]
    - ip: "1.1.1.1"
      ports: [53]
    
    # NTP server
    - ip: "time.google.com"  # Note: Will be resolved to IP
      ports: [123]
    
    # Local broadcast
    - ip: "255.255.255.255"
      ports: [67, 68]  # DHCP
```

### Max Packet Size

Limit UDP packet size to prevent abuse:

```yaml
udp:
  max_packet_size: 1500  # bytes
```

- Typical Ethernet MTU is 1500 bytes
- Prevents sending oversized packets
- `null` or omitted = no limit (not recommended)

## Socket Restrictions

### Block Privileged Ports

Prevent access to ports < 1024:

```yaml
restrictions:
  block_privileged_ports: true
```

**Blocked ports include:**
- 22 (SSH)
- 23 (Telnet)
- 25 (SMTP)
- 80 (HTTP)
- 443 (HTTPS)
- All ports 1-1023

**Use cases:**
- Prevent plugins from impersonating system services
- Security hardening
- Compliance requirements

### Allow Bind

Control whether plugins can bind to local addresses:

```yaml
restrictions:
  allow_bind: true  # Default: true
```

- `true`: Plugin can bind to local addresses (client sockets)
- `false`: Plugin cannot bind (very restrictive)

**When to disable:**
- Plugin should only make outbound connections
- No need for specific source ports
- Maximum security posture

### Allow Listen

Control whether plugins can create TCP servers:

```yaml
restrictions:
  allow_listen: false  # Default: false
```

- `true`: Plugin can listen for incoming TCP connections
- `false`: Plugin cannot act as a server

**When to enable:**
- Plugin needs to accept incoming connections
- Implementing a server component
- Peer-to-peer functionality

**Security note:** Enabling `allow_listen` significantly increases attack surface.

## Complete Examples

### Example 1: Web Client Only

Plugin can only make HTTP/HTTPS requests:

```yaml
socket_policy:
  tcp:
    allowed_destinations:
      - ip: "0.0.0.0/0"  # Any IP
        ports: [80, 443]  # Only HTTP/HTTPS
    max_connections: 10
  
  udp:
    allowed_destinations: []  # No UDP
  
  restrictions:
    block_privileged_ports: true
    allow_bind: true
    allow_listen: false
```

### Example 2: Database Client

Plugin can connect to specific database servers:

```yaml
socket_policy:
  tcp:
    allowed_destinations:
      - ip: "10.0.1.50"
        ports: [5432]  # PostgreSQL
      - ip: "10.0.1.51"
        ports: [3306]  # MySQL
      - ip: "10.0.1.52"
        ports: [6379]  # Redis
    max_connections: 20
  
  udp:
    allowed_destinations: []
  
  restrictions:
    block_privileged_ports: false  # Allow DB ports
    allow_bind: true
    allow_listen: false
```

### Example 3: Microservice

Plugin can communicate with other services in private network:

```yaml
socket_policy:
  tcp:
    allowed_destinations:
      - ip: "192.168.0.0/16"  # Entire private network
        ports: [8080, 8081, 8082, 9090]
      - ip: "127.0.0.1"
        ports: []  # All ports on localhost
    max_connections: 100
  
  udp:
    allowed_destinations:
      - ip: "192.168.0.0/16"
        ports: [53]  # DNS only
  
  restrictions:
    block_privileged_ports: true
    allow_bind: true
    allow_listen: false
```

### Example 4: P2P Application

Plugin needs to accept incoming connections:

```yaml
socket_policy:
  tcp:
    allowed_destinations:
      - ip: "0.0.0.0/0"
        ports: [6881, 6882, 6883]  # BitTorrent ports
    max_connections: 200
  
  udp:
    allowed_destinations:
      - ip: "0.0.0.0/0"
        ports: [6881, 6882, 6883]
    max_packet_size: 1500
  
  restrictions:
    block_privileged_ports: true
    allow_bind: true
    allow_listen: true  # ⚠️ Security risk!
```

### Example 5: Locked Down (Maximum Security)

Plugin has minimal network access:

```yaml
socket_policy:
  tcp:
    allowed_destinations:
      - ip: "127.0.0.1"
        ports: [8080]  # Only local service
    max_connections: 5
  
  udp:
    allowed_destinations: []  # No UDP
  
  restrictions:
    block_privileged_ports: true
    allow_bind: false  # No binding
    allow_listen: false  # No listening
```

## Implementation Details

### Policy Validation

The socket policy is validated at startup:

```rust
pub fn is_socket_destination_allowed(
    ip: &IpAddr,
    port: u16,
    destinations: &[SocketDestination],
    block_privileged: bool,
) -> bool {
    // Check privileged port restriction
    if block_privileged && port < 1024 {
        return false;
    }

    // Check against allowed destinations
    for dest in destinations {
        if is_ip_in_destination(ip, &dest.ip) {
            if dest.ports.is_empty() || dest.ports.contains(&port) {
                return true;
            }
        }
    }

    false
}
```

### How It Works

1. **Plugin attempts connection**: `connect("192.168.1.100:80")`
2. **Host intercepts**: WASI socket layer catches the call
3. **Policy check**: Validates IP + port against `allowed_destinations`
4. **Decision**: Allow or deny based on policy
5. **Enforcement**: Connection proceeds or fails with error

### Performance Considerations

- **Policy checks are fast**: O(n) where n = number of destinations
- **CIDR matching**: Uses efficient `ipnet` crate
- **No DNS lookups**: Policies use IPs, not hostnames
- **Minimal overhead**: Nanoseconds per check

## Testing

### Unit Tests

Run socket filtering tests:

```bash
cd sandboxing
cargo test --lib policy_loader::test_socket
```

Tests cover:
- ✅ Exact IP + port matching
- ✅ CIDR range matching
- ✅ All ports (empty list)
- ✅ Privileged port blocking
- ✅ IPv6 support
- ✅ Empty destinations (deny all)

### Integration Testing

Test with actual plugin:

```rust
// In plugin code
use std::net::TcpStream;

// This will be allowed if in policy
let stream = TcpStream::connect("192.168.1.100:80")?;

// This will be denied if not in policy
let denied = TcpStream::connect("10.0.0.1:22")?; // Error!
```

## Security Best Practices

### 1. Principle of Least Privilege

Only allow what's absolutely necessary:

```yaml
# ❌ Too permissive
tcp:
  allowed_destinations:
    - ip: "0.0.0.0/0"
      ports: []

# ✅ Specific and minimal
tcp:
  allowed_destinations:
    - ip: "192.168.1.100"
      ports: [80, 443]
```

### 2. Use CIDR Ranges Carefully

```yaml
# ⚠️ Very broad
- ip: "0.0.0.0/0"  # Entire internet!

# ✅ Specific subnet
- ip: "192.168.1.0/24"  # 256 IPs

# ✅ Even more specific
- ip: "192.168.1.100/32"  # Single IP
```

### 3. Block Privileged Ports

```yaml
# ✅ Always recommended
restrictions:
  block_privileged_ports: true
```

### 4. Disable Listen Unless Needed

```yaml
# ✅ Default and recommended
restrictions:
  allow_listen: false

# ⚠️ Only if absolutely necessary
restrictions:
  allow_listen: true  # Document why!
```

### 5. Set Connection Limits

```yaml
# ✅ Prevent resource exhaustion
tcp:
  max_connections: 50

# ⚠️ Unlimited (risky)
tcp:
  max_connections: null
```

### 6. Limit UDP Packet Size

```yaml
# ✅ Standard MTU
udp:
  max_packet_size: 1500

# ⚠️ Unlimited (risky)
udp:
  max_packet_size: null
```

## Troubleshooting

### Connection Denied

**Problem**: Plugin can't connect to a service

**Solution**: Add the IP + port to `allowed_destinations`:

```yaml
tcp:
  allowed_destinations:
    - ip: "192.168.1.100"
      ports: [8080]
```

### Privileged Port Blocked

**Problem**: Can't connect to port 80 or 443

**Solution**: Either:
1. Disable privileged port blocking (not recommended):
   ```yaml
   restrictions:
     block_privileged_ports: false
   ```
2. Use non-privileged ports (8080, 8443)

### UDP Not Working

**Problem**: UDP packets not being sent

**Solution**: Add UDP destinations:

```yaml
udp:
  allowed_destinations:
    - ip: "8.8.8.8"
      ports: [53]
```

### Too Many Connections

**Problem**: "Connection limit reached" error

**Solution**: Increase `max_connections`:

```yaml
tcp:
  max_connections: 100  # Increase from default
```

### Can't Bind to Port

**Problem**: Plugin can't bind to local address

**Solution**: Enable binding:

```yaml
restrictions:
  allow_bind: true
```

## Comparison: HTTP vs Socket Filtering

| Feature | HTTP Filtering | Socket Filtering |
|---------|---------------|------------------|
| **Protocols** | HTTP/HTTPS only | TCP, UDP, all protocols |
| **Granularity** | Hostname-based | IP + Port based |
| **Port Control** | No | Yes |
| **Connection Limits** | No | Yes |
| **Bind/Listen Control** | No | Yes |
| **Configuration** | Simple | More complex |
| **Use Case** | Web APIs | Any network service |

**Recommendation**: Use both together for defense in depth!

```yaml
# HTTP-level (simple, hostname-based)
allowed_hosts:
  - "api.example.com"
  - "*.internal.corp"

# Socket-level (granular, IP + port based)
socket_policy:
  tcp:
    allowed_destinations:
      - ip: "192.168.1.0/24"
        ports: [80, 443, 8080]
```

## Related Documentation

- [IP Address Filtering](./ip_address_filtering.md)
- [Environment Variables](./environment_variables.md)
- [WASI Sandboxing](./limitations_sandboxing.md)