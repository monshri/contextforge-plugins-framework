use anyhow::{Context, Result};
use serde::Deserialize;
use std::fs;
use std::net::IpAddr;
use std::str::FromStr;
use wasmtime_wasi::{DirPerms, FilePerms, WasiCtx, WasiCtxBuilder};
use ipnet::IpNet;

#[derive(Debug, Deserialize)]
pub struct PolicyConfig {
    pub plugin: PluginConfig,
}

#[derive(Debug, Deserialize)]
pub struct PluginConfig {
    pub name: String,
    pub sandbox: SandboxConfig,
}

#[derive(Debug, Deserialize)]
pub struct SandboxConfig {
    #[serde(rename = "type")]
    pub sandbox_type: String,
    pub wasm_version: String,
    pub policy: Policy,
}

#[derive(Debug, Deserialize)]
pub struct Policy {
    pub dir_name: Vec<String>,
    pub permissions: Permissions,
    /// Hostnames the guest is allowed to reach over outbound HTTP.
    /// Empty/missing list means HTTP is denied. "*" allows any host.
    /// Patterns like "*.example.com" match exactly one subdomain level.
    #[serde(default)]
    pub allowed_hosts: Vec<String>,
    /// Environment variables the guest is allowed to access.
    /// Empty/missing list means no env vars are exposed.
    /// "*" allows all environment variables (not recommended for security).
    #[serde(default)]
    pub allowed_env_vars: Vec<String>,
    /// Socket-level network policies for TCP and UDP connections.
    #[serde(default)]
    pub socket_policy: SocketPolicy,
    /// Clock access policies for time-related operations.
    #[serde(default)]
    pub clock_policy: ClockPolicy,
}

#[derive(Debug, Deserialize, Default, Clone)]
pub struct ClockPolicy {
    /// Allow access to monotonic clock (for measuring elapsed time)
    #[serde(default = "default_true")]
    pub allow_monotonic_clock: bool,
    /// Allow access to wall clock (for getting current date/time)
    #[serde(default = "default_true")]
    pub allow_wall_clock: bool,
    /// Minimum clock resolution in nanoseconds (reduces timing precision)
    /// None = no restriction, Some(n) = round to nearest n nanoseconds
    pub min_resolution_ns: Option<u64>,
    /// Maximum number of clock queries per second (rate limiting)
    pub max_queries_per_second: Option<u32>,
    /// Virtual time offset in seconds (for testing/privacy)
    /// Adds this offset to all wall clock readings
    pub time_offset_seconds: Option<i64>,
}

#[derive(Debug, Deserialize, Default, Clone)]
pub struct SocketPolicy {
    /// TCP socket policies
    #[serde(default)]
    pub tcp: TcpPolicy,
    /// UDP socket policies
    #[serde(default)]
    pub udp: UdpPolicy,
    /// General socket restrictions
    #[serde(default)]
    pub restrictions: SocketRestrictions,
}

#[derive(Debug, Deserialize, Default, Clone)]
pub struct TcpPolicy {
    /// List of allowed TCP destinations (IP/CIDR + ports)
    #[serde(default)]
    pub allowed_destinations: Vec<SocketDestination>,
    /// Maximum number of concurrent TCP connections
    pub max_connections: Option<u32>,
}

#[derive(Debug, Deserialize, Default, Clone)]
pub struct UdpPolicy {
    /// List of allowed UDP destinations (IP/CIDR + ports)
    #[serde(default)]
    pub allowed_destinations: Vec<SocketDestination>,
    /// Maximum UDP packet size in bytes
    pub max_packet_size: Option<usize>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct SocketDestination {
    /// IP address or CIDR range (e.g., "192.168.1.0/24")
    pub ip: String,
    /// List of allowed ports. Empty means all ports allowed for this IP.
    #[serde(default)]
    pub ports: Vec<u16>,
}

#[derive(Debug, Deserialize, Default, Clone)]
pub struct SocketRestrictions {
    /// Block access to privileged ports (< 1024)
    #[serde(default)]
    pub block_privileged_ports: bool,
    /// Allow bind operations
    #[serde(default = "default_true")]
    pub allow_bind: bool,
    /// Allow listen operations (for TCP servers)
    #[serde(default)]
    pub allow_listen: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Deserialize)]
pub struct Permissions {
    pub dir: Vec<String>,
    pub file: Vec<String>,
}

pub fn load_policy(path: &str) -> Result<PolicyConfig> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("reading policy file {path}"))?;
    let policy: PolicyConfig = serde_yaml::from_str(&content)
        .with_context(|| format!("parsing policy file {path}"))?;
    Ok(policy)
}

pub fn build_wasi_ctx(policy: &PolicyConfig) -> Result<WasiCtx> {
    let dir_perms = parse_dir_permissions(&policy.plugin.sandbox.policy.permissions.dir);
    let file_perms = parse_file_permissions(&policy.plugin.sandbox.policy.permissions.file);

    let mut builder = WasiCtxBuilder::new();
    builder.inherit_stdio();

    // Configure environment variables based on allowlist
    configure_env_vars(&mut builder, &policy.plugin.sandbox.policy.allowed_env_vars);

    for dir in &policy.plugin.sandbox.policy.dir_name {
        builder
            .preopened_dir(dir, ".", dir_perms, file_perms)
            .with_context(|| format!("preopening dir {dir}"))?;
    }

    Ok(builder.build())
}

/// Configure environment variables for the WASI context based on the allowlist.
///
/// Rules:
/// * empty list -> no env vars exposed
/// * "*" -> inherit all env vars from host
/// * specific names -> only expose those env vars if they exist in the host environment
fn configure_env_vars(builder: &mut WasiCtxBuilder, allowed: &[String]) {
    if allowed.is_empty() {
        // No environment variables allowed - don't inherit any
        return;
    }

    // Check if wildcard is present
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
        // If the env var doesn't exist in the host, we simply don't add it
    }
}

fn parse_dir_permissions(perms: &[String]) -> DirPerms {
    if perms.iter().any(|p| p == "mutate") {
        DirPerms::all()
    } else {
        DirPerms::READ
    }
}

fn parse_file_permissions(perms: &[String]) -> FilePerms {
    if perms.iter().any(|p| p == "write") {
        FilePerms::all()
    } else {
        FilePerms::READ
    }
}

/// Decide whether `host` is allowed under a list of allowlist patterns.
///
/// Rules:
/// * empty list  -> deny everything (fail-closed)
/// * "*"         -> allow everything
/// * exact match -> allow (hostname or IP)
/// * "*.suffix"  -> allow if `host` is exactly one label deeper than `suffix`
///                  (so "*.example.com" matches "api.example.com" but not
///                  "a.b.example.com" and not "example.com")
/// * IP address  -> allow exact IP match (IPv4 or IPv6)
/// * CIDR range  -> allow if IP is within the CIDR range (e.g., "192.168.1.0/24")
///
/// Matching is case-insensitive on hostnames. IP addresses are matched exactly.
pub fn is_host_allowed(host: &str, allowed: &[String]) -> bool {
    if allowed.is_empty() {
        return false;
    }
    let host_lower = host.to_ascii_lowercase();

    for pattern in allowed {
        let pattern = pattern.to_ascii_lowercase();
        
        // Allow all
        if pattern == "*" {
            return true;
        }
        
        // Check if host is an IP address and pattern matches it
        if is_ip_match(host, &pattern) {
            return true;
        }
        
        // Wildcard hostname matching
        if let Some(suffix) = pattern.strip_prefix("*.") {
            // "*.example.com" -> require host to end with ".example.com"
            // and the part before that to be a single non-empty label.
            if let Some(prefix) = host_lower.strip_suffix(&format!(".{suffix}")) {
                if !prefix.is_empty() && !prefix.contains('.') {
                    return true;
                }
            }
            continue;
        }
        
        // Exact hostname match
        if host_lower == pattern {
            return true;
        }
    }
    false
}

/// Check if a host (which might be an IP) matches an IP pattern.
///
/// Supports:
/// * Exact IP match: "192.168.1.1" matches "192.168.1.1"
/// * CIDR ranges: "192.168.1.50" matches "192.168.1.0/24"
/// * IPv6: "2001:db8::1" matches "2001:db8::1" or "2001:db8::/32"
fn is_ip_match(host: &str, pattern: &str) -> bool {
    // Try to parse host as an IP address
    let host_ip = match IpAddr::from_str(host) {
        Ok(ip) => ip,
        Err(_) => return false, // Not an IP address
    };
    
    // Check if pattern is a CIDR range
    if pattern.contains('/') {
        if let Ok(cidr) = IpNet::from_str(pattern) {
            return cidr.contains(&host_ip);
        }
        return false;
    }
    
    // Check for exact IP match
    if let Ok(pattern_ip) = IpAddr::from_str(pattern) {
        return host_ip == pattern_ip;
    }
    
    false
}

/// Check if a socket destination (IP + port) is allowed by the policy.
///
/// Returns true if:
/// - The IP matches an allowed destination (exact or CIDR range)
/// - The port is in the allowed ports list (or ports list is empty = all allowed)
/// - Privileged port restrictions are respected
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

    // Empty list means deny all
    if destinations.is_empty() {
        return false;
    }

    for dest in destinations {
        // Check if IP matches
        if !is_ip_in_destination(ip, &dest.ip) {
            continue;
        }

        // If ports list is empty, all ports are allowed for this IP
        if dest.ports.is_empty() {
            return true;
        }

        // Check if port is in the allowed list
        if dest.ports.contains(&port) {
            return true;
        }
    }

    false
}

/// Check if an IP address matches a destination pattern (exact IP or CIDR).
fn is_ip_in_destination(ip: &IpAddr, pattern: &str) -> bool {
    // Try CIDR range first
    if pattern.contains('/') {
        if let Ok(cidr) = IpNet::from_str(pattern) {
            return cidr.contains(ip);
        }
        return false;
    }

    // Try exact IP match
    if let Ok(pattern_ip) = IpAddr::from_str(pattern) {
        return ip == &pattern_ip;
    }

    false
}

#[cfg(test)]
mod tests {
    use super::{is_host_allowed, is_socket_destination_allowed, SocketDestination};
    use std::net::IpAddr;

    #[test]
    fn empty_list_denies() {
        assert!(!is_host_allowed("example.com", &[]));
    }

    #[test]
    fn star_allows_everything() {
        let allow = vec!["*".to_string()];
        assert!(is_host_allowed("anything.test", &allow));
    }

    #[test]
    fn exact_match() {
        let allow = vec!["api.example.com".to_string()];
        assert!(is_host_allowed("api.example.com", &allow));
        assert!(!is_host_allowed("other.example.com", &allow));
        assert!(!is_host_allowed("api.example.com.evil.test", &allow));
    }

    #[test]
    fn wildcard_one_label() {
        let allow = vec!["*.example.com".to_string()];
        assert!(is_host_allowed("api.example.com", &allow));
        assert!(!is_host_allowed("example.com", &allow));
        assert!(!is_host_allowed("a.b.example.com", &allow));
    }

    #[test]
    fn case_insensitive() {
        let allow = vec!["API.Example.com".to_string()];
        assert!(is_host_allowed("api.example.COM", &allow));
    }

    #[test]
    fn test_ipv4_exact_match() {
        let allow = vec!["192.168.1.100".to_string()];
        assert!(is_host_allowed("192.168.1.100", &allow));
        assert!(!is_host_allowed("192.168.1.101", &allow));
    }

    #[test]
    fn test_ipv4_cidr_range() {
        let allow = vec!["192.168.1.0/24".to_string()];
        assert!(is_host_allowed("192.168.1.1", &allow));
        assert!(is_host_allowed("192.168.1.100", &allow));
        assert!(is_host_allowed("192.168.1.255", &allow));
        assert!(!is_host_allowed("192.168.2.1", &allow));
        assert!(!is_host_allowed("10.0.0.1", &allow));
    }

    #[test]
    fn test_ipv6_exact_match() {
        let allow = vec!["2001:db8::1".to_string()];
        assert!(is_host_allowed("2001:db8::1", &allow));
        assert!(!is_host_allowed("2001:db8::2", &allow));
    }

    #[test]
    fn test_ipv6_cidr_range() {
        let allow = vec!["2001:db8::/32".to_string()];
        assert!(is_host_allowed("2001:db8::1", &allow));
        assert!(is_host_allowed("2001:db8:1::1", &allow));
        assert!(!is_host_allowed("2001:db9::1", &allow));
    }

    #[test]
    fn test_mixed_hostnames_and_ips() {
        let allow = vec![
            "example.com".to_string(),
            "192.168.1.100".to_string(),
            "10.0.0.0/8".to_string(),
        ];
        
        // Hostname
        assert!(is_host_allowed("example.com", &allow));
        assert!(!is_host_allowed("other.com", &allow));
        
        // Exact IP
        assert!(is_host_allowed("192.168.1.100", &allow));
        assert!(!is_host_allowed("192.168.1.101", &allow));
        
        // CIDR range
        assert!(is_host_allowed("10.0.0.1", &allow));
        assert!(is_host_allowed("10.255.255.255", &allow));
        assert!(!is_host_allowed("11.0.0.1", &allow));
    }

    #[test]
    fn test_localhost_ips() {
        let allow = vec!["127.0.0.1".to_string(), "::1".to_string()];
        assert!(is_host_allowed("127.0.0.1", &allow));
        assert!(is_host_allowed("::1", &allow));
        assert!(!is_host_allowed("127.0.0.2", &allow));
    }

    #[test]
    fn test_env_var_filtering() {
        use std::env;
        use super::WasiCtxBuilder;
        
        // Set some test env vars
        unsafe {
            env::set_var("TEST_ALLOWED", "allowed_value");
            env::set_var("TEST_DENIED", "denied_value");
        }
        
        let mut builder = WasiCtxBuilder::new();
        
        // Test with specific allowlist
        let allowed = vec!["TEST_ALLOWED".to_string()];
        super::configure_env_vars(&mut builder, &allowed);
        let _ctx = builder.build();
        
        // Clean up
        unsafe {
            env::remove_var("TEST_ALLOWED");
            env::remove_var("TEST_DENIED");
        }
    }

    #[test]
    fn test_env_var_wildcard() {
        use super::WasiCtxBuilder;
        
        let mut builder = WasiCtxBuilder::new();
        
        // Test with wildcard - should inherit all
        let allowed = vec!["*".to_string()];
        super::configure_env_vars(&mut builder, &allowed);
        let _ctx = builder.build();
    }

    #[test]
    fn test_env_var_empty_list() {
        use super::WasiCtxBuilder;
        
        let mut builder = WasiCtxBuilder::new();
        
        // Test with empty list - should not inherit any
        let allowed: Vec<String> = vec![];
        super::configure_env_vars(&mut builder, &allowed);
        let _ctx = builder.build();
    }
}
    #[test]
    fn test_socket_destination_exact_ip_and_port() {
        use std::net::Ipv4Addr;
        
        let destinations = vec![
            SocketDestination {
                ip: "192.168.1.100".to_string(),
                ports: vec![80, 443],
            },
        ];
        
        let ip = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100));
        
        assert!(is_socket_destination_allowed(&ip, 80, &destinations, false));
        assert!(is_socket_destination_allowed(&ip, 443, &destinations, false));
        assert!(!is_socket_destination_allowed(&ip, 8080, &destinations, false));
        
        let wrong_ip = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 101));
        assert!(!is_socket_destination_allowed(&wrong_ip, 80, &destinations, false));
    }

    #[test]
    fn test_socket_destination_cidr_range() {
        use std::net::Ipv4Addr;
        
        let destinations = vec![
            SocketDestination {
                ip: "192.168.1.0/24".to_string(),
                ports: vec![80],
            },
        ];
        
        let ip1 = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1));
        let ip2 = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 255));
        let ip3 = IpAddr::V4(Ipv4Addr::new(192, 168, 2, 1));
        
        assert!(is_socket_destination_allowed(&ip1, 80, &destinations, false));
        assert!(is_socket_destination_allowed(&ip2, 80, &destinations, false));
        assert!(!is_socket_destination_allowed(&ip3, 80, &destinations, false));
    }

    #[test]
    fn test_socket_destination_all_ports() {
        use std::net::Ipv4Addr;
        
        let destinations = vec![
            SocketDestination {
                ip: "127.0.0.1".to_string(),
                ports: vec![],  // Empty = all ports allowed
            },
        ];
        
        let ip = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
        
        assert!(is_socket_destination_allowed(&ip, 80, &destinations, false));
        assert!(is_socket_destination_allowed(&ip, 443, &destinations, false));
        assert!(is_socket_destination_allowed(&ip, 8080, &destinations, false));
        assert!(is_socket_destination_allowed(&ip, 65535, &destinations, false));
    }

    #[test]
    fn test_socket_privileged_port_blocking() {
        use std::net::Ipv4Addr;
        
        let destinations = vec![
            SocketDestination {
                ip: "192.168.1.100".to_string(),
                ports: vec![],  // All ports
            },
        ];
        
        let ip = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100));
        
        // Without blocking
        assert!(is_socket_destination_allowed(&ip, 22, &destinations, false));
        assert!(is_socket_destination_allowed(&ip, 80, &destinations, false));
        assert!(is_socket_destination_allowed(&ip, 1024, &destinations, false));
        
        // With blocking privileged ports
        assert!(!is_socket_destination_allowed(&ip, 22, &destinations, true));
        assert!(!is_socket_destination_allowed(&ip, 80, &destinations, true));
        assert!(!is_socket_destination_allowed(&ip, 1023, &destinations, true));
        assert!(is_socket_destination_allowed(&ip, 1024, &destinations, true));
    }

    #[test]
    fn test_socket_empty_destinations() {
        use std::net::Ipv4Addr;
        
        let destinations: Vec<SocketDestination> = vec![];
        let ip = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100));
        
        // Empty list should deny all
        assert!(!is_socket_destination_allowed(&ip, 80, &destinations, false));
    }

    #[test]
    fn test_socket_ipv6_support() {
        use std::net::Ipv6Addr;
        
        let destinations = vec![
            SocketDestination {
                ip: "2001:db8::1".to_string(),
                ports: vec![443],
            },
            SocketDestination {
                ip: "2001:db8::/32".to_string(),
                ports: vec![80],
            },
        ];
        
        let ip1 = IpAddr::V6(Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1));
        let ip2 = IpAddr::V6(Ipv6Addr::new(0x2001, 0xdb8, 1, 0, 0, 0, 0, 1));
        
        assert!(is_socket_destination_allowed(&ip1, 443, &destinations, false));
        assert!(is_socket_destination_allowed(&ip2, 80, &destinations, false));
        assert!(!is_socket_destination_allowed(&ip2, 443, &destinations, false));
    }

/// Apply clock resolution rounding to a nanosecond timestamp.
/// 
/// If min_resolution_ns is set, rounds the timestamp to the nearest multiple
/// of that resolution. This reduces timing precision to prevent timing attacks.
pub fn apply_clock_resolution(timestamp_ns: u64, min_resolution_ns: Option<u64>) -> u64 {
    match min_resolution_ns {
        Some(resolution) if resolution > 0 => {
            // Round to nearest multiple of resolution
            (timestamp_ns / resolution) * resolution
        }
        _ => timestamp_ns,
    }
}

/// Apply time offset to a Unix timestamp (seconds since epoch).
///
/// Used for virtual time or privacy - adds the configured offset to the real time.
pub fn apply_time_offset(timestamp_seconds: u64, offset_seconds: Option<i64>) -> u64 {
    match offset_seconds {
        Some(offset) => {
            if offset >= 0 {
                timestamp_seconds.saturating_add(offset as u64)
            } else {
                timestamp_seconds.saturating_sub((-offset) as u64)
            }
        }
        None => timestamp_seconds,
    }
}

/// Check if a clock query should be rate-limited.
///
/// This is a simple check - actual implementation would need to track
/// query counts per time window in the MyState struct.
pub fn should_rate_limit_clock_query(
    queries_this_second: u32,
    max_queries_per_second: Option<u32>,
) -> bool {
    match max_queries_per_second {
        Some(max) => queries_this_second >= max,
        None => false,
    }
}

    #[test]
    fn test_clock_resolution_rounding() {
        // No rounding when resolution is None
        assert_eq!(apply_clock_resolution(1234567890, None), 1234567890);
        
        // Round to 1ms (1,000,000 ns)
        assert_eq!(apply_clock_resolution(1234567890, Some(1_000_000)), 1234000000);
        assert_eq!(apply_clock_resolution(1234999999, Some(1_000_000)), 1234000000);
        assert_eq!(apply_clock_resolution(1235000000, Some(1_000_000)), 1235000000);
        
        // Round to 1 second (1,000,000,000 ns)
        assert_eq!(apply_clock_resolution(1234567890, Some(1_000_000_000)), 1000000000);
        assert_eq!(apply_clock_resolution(2999999999, Some(1_000_000_000)), 2000000000);
    }

    #[test]
    fn test_time_offset_positive() {
        // Add 1 hour (3600 seconds)
        assert_eq!(apply_time_offset(1000, Some(3600)), 4600);
        
        // Add 1 day (86400 seconds)
        assert_eq!(apply_time_offset(1000, Some(86400)), 87400);
    }

    #[test]
    fn test_time_offset_negative() {
        // Subtract 1 hour
        assert_eq!(apply_time_offset(5000, Some(-3600)), 1400);
        
        // Subtract more than available (saturating)
        assert_eq!(apply_time_offset(1000, Some(-2000)), 0);
    }

    #[test]
    fn test_time_offset_none() {
        // No offset
        assert_eq!(apply_time_offset(1234567890, None), 1234567890);
    }

    #[test]
    fn test_rate_limiting() {
        // No limit
        assert!(!should_rate_limit_clock_query(1000, None));
        assert!(!should_rate_limit_clock_query(1000000, None));
        
        // With limit of 100
        assert!(!should_rate_limit_clock_query(50, Some(100)));
        assert!(!should_rate_limit_clock_query(99, Some(100)));
        assert!(should_rate_limit_clock_query(100, Some(100)));
        assert!(should_rate_limit_clock_query(101, Some(100)));
        assert!(should_rate_limit_clock_query(1000, Some(100)));
    }