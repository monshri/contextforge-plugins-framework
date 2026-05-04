use anyhow::{Context, Result};
use sandboxing::policy_loader::{build_wasi_ctx, load_policy};
use sandboxing::{MyState, Plugin};
use wasmtime::component::{Component, Linker, ResourceTable};
use wasmtime::{Config, Engine, Store};
use wasmtime_wasi_http::WasiHttpCtx;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<()> {
    // 1. Load policy first to configure engine with resource limits
    let policy = load_policy("config/policy.yaml")?;
    let resource_limits = policy.plugin.sandbox.policy.resource_limits.clone();
    
    // 2. Engine. Async support is required because wasmtime-wasi-http's
    //    host implementation is async, and we want the plugin to be able
    //    to await outbound HTTP responses.
    let mut config = Config::new();
    config.wasm_component_model(true);
    config.async_support(true);
    
    // Enable fuel metering for CPU limits
    if resource_limits.max_fuel.is_some() || resource_limits.cpu_timeout_ms.is_some() {
        config.consume_fuel(true);
    }
    
    // Enable epoch interruption for timeouts
    if resource_limits.wall_clock_timeout_ms.is_some() {
        config.epoch_interruption(true);
    }
    
    let engine = Engine::new(&config)?;

    // 3. Linker. Add WASI preview 2 + WASI HTTP, plus any of our own
    //    custom imports the plugin world declares (none, in this example).
    let mut linker: Linker<MyState> = Linker::new(&engine);
    wasmtime_wasi::add_to_linker_async(&mut linker)
        .context("adding WASI to linker")?;
    wasmtime_wasi_http::add_only_http_to_linker_async(&mut linker)
        .context("adding WASI HTTP to linker")?;

    // 4. Build the WASI filesystem ctx from policy.yaml and
    //    grab the host allowlist out for use in WasiHttpView::send_request.
    let wasi = build_wasi_ctx(&policy)?;
    let allowed_hosts = policy.plugin.sandbox.policy.allowed_hosts.clone();

    let state = MyState {
        wasi,
        http: WasiHttpCtx::new(),
        table: ResourceTable::new(),
        allowed_hosts,
        resource_limits: resource_limits.clone(),
        start_time: std::time::Instant::now(),
    };
    let mut store = Store::new(&engine, state);
    
    // Set fuel limit if configured
    if let Some(fuel) = resource_limits.max_fuel {
        store.set_fuel(fuel).context("setting fuel limit")?;
        println!("Set fuel limit: {} units", fuel);
    }
    
    // Set epoch deadline for timeout interruption
    if resource_limits.wall_clock_timeout_ms.is_some() {
        store.set_epoch_deadline(1);
    }
    
    // Start epoch ticker for wall clock timeout
    let engine_clone = engine.clone();
    if let Some(timeout_ms) = resource_limits.wall_clock_timeout_ms {
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(timeout_ms)).await;
            engine_clone.increment_epoch();
        });
        println!("Set wall clock timeout: {}ms", timeout_ms);
    }

    // 5. Load the compiled plugin component.
    let component = Component::from_file(
        &engine,
        "plugin/target/wasm32-wasip2/release/plugin.wasm",
    )
    .context("loading plugin.wasm")?;
    let plugin = Plugin::instantiate_async(&mut store, &component, &linker).await?;

    println!("\n=== Resource Limits Active ===");
    if let Some(mem) = resource_limits.max_memory_bytes {
        println!("Max Memory: {} bytes ({} MB)", mem, mem / 1024 / 1024);
    }
    if let Some(fuel) = resource_limits.max_fuel {
        println!("Max Fuel: {} units", fuel);
    }
    if let Some(timeout) = resource_limits.cpu_timeout_ms {
        println!("CPU Timeout: {}ms", timeout);
    }
    if let Some(timeout) = resource_limits.wall_clock_timeout_ms {
        println!("Wall Clock Timeout: {}ms", timeout);
    }
    println!("==============================\n");

    // 6. Call into the plugin. The plugin exposes check_key, create_file, and make_http_request.
    println!("--- Calling check_key ---");
    let result = plugin
        .example_plugin_policy()
        .call_check_key(&mut store, r#"{"action": "allow"}"#, "action")
        .await?;
    println!("Result: {result}");
    print_resource_usage(&store, &resource_limits);

    // Test file creation (will be restricted by the sandbox policy)
    println!("\n--- Calling create_file ---");
    let file_result = plugin
        .example_plugin_policy()
        .call_create_file(&mut store, "data/test_output.txt", "Hello from sandboxed plugin!")
        .await?;
    println!("Result: {file_result}");
    print_resource_usage(&store, &resource_limits);

    // Test HTTP request to an allowed host (httpbin.org should be in allowed_hosts)
    println!("\n--- Testing HTTP request to allowed host ---");
    let http_response = plugin
        .example_plugin_policy()
        .call_make_http_request(&mut store, "https://httpbin.org/get")
        .await?;
    println!("Response from httpbin.org:\n{}", http_response);
    print_resource_usage(&store, &resource_limits);

    // Test HTTP request to a denied host (should be blocked by the allowlist)
    println!("\n--- Testing HTTP request to denied host ---");
    let denied_response = plugin
        .example_plugin_policy()
        .call_make_http_request(&mut store, "https://evil.example.test/")
        .await?;
    println!("Response from denied host:\n{}", denied_response);
    print_resource_usage(&store, &resource_limits);

    // Test HTTP request to an IP address (127.0.0.1 should be in allowed_hosts)
    println!("\n--- Testing HTTP request to allowed IP address ---");
    let ip_response = plugin
        .example_plugin_policy()
        .call_make_http_request(&mut store, "http://127.0.0.1:8080/test")
        .await?;
    println!("Response from 127.0.0.1:\n{}", ip_response);
    print_resource_usage(&store, &resource_limits);

    // Test environment variable access
    println!("\n--- Testing environment variable access ---");
    
    // Test allowed env var (PATH is in the allowlist)
    let path_result = plugin
        .example_plugin_policy()
        .call_get_env_var(&mut store, "PATH")
        .await?;
    println!("Allowed env var (PATH): {}", path_result);
    print_resource_usage(&store, &resource_limits);
    
    // Test allowed env var (USER is in the allowlist)
    let user_result = plugin
        .example_plugin_policy()
        .call_get_env_var(&mut store, "USER")
        .await?;
    println!("Allowed env var (USER): {}", user_result);
    print_resource_usage(&store, &resource_limits);
    
    // Test denied env var (SECRET_KEY is NOT in the allowlist)
    let secret_result = plugin
        .example_plugin_policy()
        .call_get_env_var(&mut store, "SECRET_KEY")
        .await?;
    println!("Denied env var (SECRET_KEY): {}", secret_result);
    print_resource_usage(&store, &resource_limits);
    
    // Test another denied env var (SHELL is NOT in the allowlist)
    let shell_result = plugin
        .example_plugin_policy()
        .call_get_env_var(&mut store, "SHELL")
        .await?;
    println!("Denied env var (SHELL): {}", shell_result);
    print_resource_usage(&store, &resource_limits);

    println!("\n=== Final Resource Usage ===");
    print_resource_usage(&store, &resource_limits);
    
    Ok(())
}

/// Print current resource usage statistics
fn print_resource_usage(store: &Store<MyState>, limits: &sandboxing::policy_loader::ResourceLimits) {
    // Print fuel consumption if enabled
    if let Some(max_fuel) = limits.max_fuel {
        if let Ok(remaining) = store.get_fuel() {
            let consumed = max_fuel.saturating_sub(remaining);
            println!("  Fuel: consumed={}, remaining={}", consumed, remaining);
        }
    }
    
    // Print elapsed time
    let elapsed = store.data().start_time.elapsed();
    println!("  Elapsed time: {:.2}ms", elapsed.as_secs_f64() * 1000.0);
    
    // Check if approaching limits
    if let Some(timeout) = limits.wall_clock_timeout_ms {
        let elapsed_ms = elapsed.as_millis() as u64;
        if elapsed_ms > timeout * 80 / 100 {
            println!("  ⚠️  WARNING: Approaching wall clock timeout!");
        }
    }
}