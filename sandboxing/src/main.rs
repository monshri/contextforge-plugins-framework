use anyhow::{Context, Result};
use sandboxing::policy_loader::{build_wasi_ctx, load_policy};
use sandboxing::{MyState, Plugin};
use wasmtime::component::{Component, Linker, ResourceTable};
use wasmtime::{Config, Engine, Store};
use wasmtime_wasi_http::WasiHttpCtx;

#[tokio::main]
async fn main() -> Result<()> {
    // 1. Engine. Async support is required because wasmtime-wasi-http's
    //    host implementation is async, and we want the plugin to be able
    //    to await outbound HTTP responses.
    let mut config = Config::new();
    config.wasm_component_model(true);
    config.async_support(true);
    let engine = Engine::new(&config)?;

    // 2. Linker. Add WASI preview 2 + WASI HTTP, plus any of our own
    //    custom imports the plugin world declares (none, in this example).
    let mut linker: Linker<MyState> = Linker::new(&engine);
    wasmtime_wasi::add_to_linker_async(&mut linker)
        .context("adding WASI to linker")?;
    wasmtime_wasi_http::add_only_http_to_linker_async(&mut linker)
        .context("adding WASI HTTP to linker")?;

    // 3. Policy. Build the WASI filesystem ctx from policy.yaml and
    //    grab the host allowlist out for use in WasiHttpView::send_request.
    let policy = load_policy("config/policy.yaml")?;
    let wasi = build_wasi_ctx(&policy)?;
    let allowed_hosts = policy.plugin.sandbox.policy.allowed_hosts.clone();

    let state = MyState {
        wasi,
        http: WasiHttpCtx::new(),
        table: ResourceTable::new(),
        allowed_hosts,
    };
    let mut store = Store::new(&engine, state);

    // 4. Load the compiled plugin component.
    let component = Component::from_file(
        &engine,
        "plugin/target/wasm32-wasip2/release/plugin.wasm",
    )
    .context("loading plugin.wasm")?;
    let plugin = Plugin::instantiate_async(&mut store, &component, &linker).await?;

    // 5. Call into the plugin. The plugin exposes check_key, create_file, and make_http_request.
    let result = plugin
        .example_plugin_policy()
        .call_check_key(&mut store, r#"{"action": "allow"}"#, "action")
        .await?;
    println!("--- check_key result ---\n{result}");

    // Test file creation (will be restricted by the sandbox policy)
    let file_result = plugin
        .example_plugin_policy()
        .call_create_file(&mut store, "data/test_output.txt", "Hello from sandboxed plugin!")
        .await?;
    println!("--- create_file result ---\n{file_result}");

    // Test HTTP request to an allowed host (httpbin.org should be in allowed_hosts)
    println!("\n--- Testing HTTP request to allowed host ---");
    let http_response = plugin
        .example_plugin_policy()
        .call_make_http_request(&mut store, "https://httpbin.org/get")
        .await?;
    println!("Response from httpbin.org:\n{}", http_response);

    // Test HTTP request to a denied host (should be blocked by the allowlist)
    println!("\n--- Testing HTTP request to denied host ---");
    let denied_response = plugin
        .example_plugin_policy()
        .call_make_http_request(&mut store, "https://evil.example.test/")
        .await?;
    println!("Response from denied host:\n{}", denied_response);

    // Test HTTP request to an IP address (127.0.0.1 should be in allowed_hosts)
    println!("\n--- Testing HTTP request to allowed IP address ---");
    let ip_response = plugin
        .example_plugin_policy()
        .call_make_http_request(&mut store, "http://127.0.0.1:8080/test")
        .await?;
    println!("Response from 127.0.0.1:\n{}", ip_response);

    // Test environment variable access
    println!("\n--- Testing environment variable access ---");
    
    // Test allowed env var (PATH is in the allowlist)
    let path_result = plugin
        .example_plugin_policy()
        .call_get_env_var(&mut store, "PATH")
        .await?;
    println!("Allowed env var (PATH): {}", path_result);
    
    // Test allowed env var (USER is in the allowlist)
    let user_result = plugin
        .example_plugin_policy()
        .call_get_env_var(&mut store, "USER")
        .await?;
    println!("Allowed env var (USER): {}", user_result);
    
    // Test denied env var (SECRET_KEY is NOT in the allowlist)
    let secret_result = plugin
        .example_plugin_policy()
        .call_get_env_var(&mut store, "SECRET_KEY")
        .await?;
    println!("Denied env var (SECRET_KEY): {}", secret_result);
    
    // Test another denied env var (SHELL is NOT in the allowlist)
    let shell_result = plugin
        .example_plugin_policy()
        .call_get_env_var(&mut store, "SHELL")
        .await?;
    println!("Denied env var (SHELL): {}", shell_result);

    Ok(())
}