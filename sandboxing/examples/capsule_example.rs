//! Example demonstrating capsule-core usage for WASM sandboxing
//! 
//! This example shows how to:
//! 1. Create an execution policy with resource limits
//! 2. Set up a WASM runtime with capsule-core
//! 3. Execute WASM modules with security constraints

use capsule_core::wasm::execution_policy::{ExecutionPolicy, Compute};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Capsule-Core WASM Sandboxing Example ===\n");

    // 1. Create an execution policy with resource limits
    let policy = create_execution_policy();
    print_policy(&policy);

    // 2. Demonstrate policy configuration
    demonstrate_compute_levels();

    println!("\n=== Example Complete ===");
    Ok(())
}

/// Creates a comprehensive execution policy for WASM sandboxing
fn create_execution_policy() -> ExecutionPolicy {
    ExecutionPolicy::new()
        .name(Some("example-policy".to_string()))
        .compute(Some(Compute::Medium))  // 2 billion fuel units
        .ram(Some(128 * 1024 * 1024))    // 128 MB RAM limit
        .timeout(Some("30s".to_string())) // 30 second timeout
        .max_retries(Some(3))             // Retry up to 3 times on failure
        .allowed_files(vec![
            "/tmp/data".to_string(),
            "/app/config".to_string(),
        ])
        .allowed_hosts(vec![
            "api.example.com".to_string(),
            "*.trusted-domain.com".to_string(),
        ])
        .env_variables(vec![
            "API_KEY".to_string(),
            "LOG_LEVEL".to_string(),
        ])
}

/// Prints the execution policy details
fn print_policy(policy: &ExecutionPolicy) {
    println!("Execution Policy Configuration:");
    println!("  Name: {}", policy.name);
    println!("  Compute: {:?} ({} fuel units)", policy.compute, policy.compute.as_fuel());
    println!("  RAM Limit: {} MB", policy.ram.unwrap_or(0) / (1024 * 1024));
    println!("  Timeout: {}", policy.timeout.as_ref().unwrap_or(&"None".to_string()));
    println!("  Max Retries: {}", policy.max_retries);
    println!("  Allowed Files: {:?}", policy.allowed_files);
    println!("  Allowed Hosts: {:?}", policy.allowed_hosts);
    println!("  Environment Variables: {:?}", policy.env_variables);
    
    if let Some(duration) = policy.timeout_duration() {
        println!("  Timeout Duration: {:?}", duration);
    }
}

/// Demonstrates different compute levels available
fn demonstrate_compute_levels() {
    println!("\n--- Compute Levels ---");
    
    let levels = vec![
        ("Low", Compute::Low),
        ("Medium", Compute::Medium),
        ("High", Compute::High),
        ("Custom", Compute::Custom(5_000_000_000)),
    ];
    
    for (name, compute) in levels {
        println!("  {}: {} fuel units", name, compute.as_fuel());
    }
}

/// Example: Creating a restrictive policy for untrusted code
#[allow(dead_code)]
fn create_restrictive_policy() -> ExecutionPolicy {
    ExecutionPolicy::new()
        .name(Some("restrictive".to_string()))
        .compute(Some(Compute::Low))      // Minimal compute
        .ram(Some(32 * 1024 * 1024))      // 32 MB RAM
        .timeout(Some("5s".to_string()))  // Short timeout
        .max_retries(Some(0))             // No retries
        .allowed_files(vec![])            // No file access
        .allowed_hosts(vec![])            // No network access
        .env_variables(vec![])            // No environment variables
}

/// Example: Creating a permissive policy for trusted code
#[allow(dead_code)]
fn create_permissive_policy() -> ExecutionPolicy {
    ExecutionPolicy::new()
        .name(Some("permissive".to_string()))
        .compute(Some(Compute::High))     // Maximum compute
        .ram(Some(512 * 1024 * 1024))     // 512 MB RAM
        .timeout(Some("5m".to_string()))  // 5 minute timeout
        .max_retries(Some(5))             // Multiple retries
        .allowed_files(vec![
            "/data".to_string(),
            "/tmp".to_string(),
            "/app".to_string(),
        ])
        .allowed_hosts(vec![
            "*".to_string(),              // Allow all hosts (use with caution!)
        ])
        .env_variables(vec![
            "API_KEY".to_string(),
            "DATABASE_URL".to_string(),
            "LOG_LEVEL".to_string(),
        ])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_policy_creation() {
        let policy = create_execution_policy();
        assert_eq!(policy.name, "example-policy");
        assert_eq!(policy.compute, Compute::Medium);
        assert_eq!(policy.ram, Some(128 * 1024 * 1024));
        assert_eq!(policy.max_retries, 3);
    }

    #[test]
    fn test_compute_levels() {
        assert_eq!(Compute::Low.as_fuel(), 100_000_000);
        assert_eq!(Compute::Medium.as_fuel(), 2_000_000_000);
        assert_eq!(Compute::High.as_fuel(), 50_000_000_000);
        assert_eq!(Compute::Custom(1000).as_fuel(), 1000);
    }

    #[test]
    fn test_restrictive_policy() {
        let policy = create_restrictive_policy();
        assert_eq!(policy.compute, Compute::Low);
        assert_eq!(policy.max_retries, 0);
        assert!(policy.allowed_files.is_empty());
        assert!(policy.allowed_hosts.is_empty());
    }

    #[test]
    fn test_permissive_policy() {
        let policy = create_permissive_policy();
        assert_eq!(policy.compute, Compute::High);
        assert_eq!(policy.max_retries, 5);
        assert!(!policy.allowed_files.is_empty());
        assert!(!policy.allowed_hosts.is_empty());
    }
}

// Made with Bob
