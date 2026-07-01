// CPEX WASM Custom Payload Demo
//
// Demonstrates dispatching a custom (non-CMF) payload through a WASM plugin.
// Mirrors the ToolInvokePayload from cpex-core/examples/plugin_demo.rs,
// but routes it through the WASM sandbox as a Payload::Custom JSON envelope.
//
// Run with: cargo run --example wasm_custom_payload_demo

use std::path::PathBuf;
use std::sync::Arc;

use cpex_core::config::parse_config;
use cpex_core::extensions::{HttpExtension, RequestExtension};
use cpex_core::hooks::payload::{Extensions, MetaExtension};
use cpex_core::hooks::trait_def::{HookTypeDef, PluginResult};
use cpex_core::manager::PluginManager;

use cpex_wasm_host::factory::WasmPluginFactory;

// ---------------------------------------------------------------------------
// Step 1: Define a custom payload and hook type
// ---------------------------------------------------------------------------

/// Custom payload carried through the tool_pre_invoke hook.
/// This is NOT a CMF MessagePayload — it crosses the WASM boundary as
/// Payload::Custom { payload_type: "ToolInvokePayload", payload_json: "..." }.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct ToolInvokePayload {
    tool_name: String,
    user: String,
    arguments: String,
}

cpex_core::impl_plugin_payload_json!(ToolInvokePayload, "ToolInvokePayload");

/// Hook type definition for tool_pre_invoke with a custom payload.
struct ToolPreInvokeHook;
impl HookTypeDef for ToolPreInvokeHook {
    type Payload = ToolInvokePayload;
    type Result = PluginResult<ToolInvokePayload>;
    const NAME: &'static str = "tool_pre_invoke";
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

#[tokio::main]
async fn main() {
    println!("=== WASM Custom Payload Demo ===\n");

    let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let config_path = crate_dir.join("config/custom_payload_demo.yaml");
    println!("--- Loading config from {} ---\n", config_path.display());

    let yaml = std::fs::read_to_string(&config_path)
        .unwrap_or_else(|e| panic!("Failed to read {}: {}", config_path.display(), e));
    let cpex_config = parse_config(&yaml).unwrap();

    let mgr = PluginManager::default();
    mgr.register_factory(
        "wasm://plugin.wasm",
        Box::new(WasmPluginFactory::new(crate_dir.join("wasm"))),
    );

    mgr.load_config(cpex_config).unwrap();
    mgr.initialize().await.unwrap();

    // --- Scenario 1: Valid user with tool invocation ---
    println!("=== Scenario 1: get_compensation (custom payload, valid user) ===\n");
    let payload = ToolInvokePayload {
        tool_name: "get_compensation".into(),
        user: "alice".into(),
        arguments: r#"{"employee_id": 42}"#.into(),
    };

    let ext = Extensions {
        request: Some(Arc::new(RequestExtension {
            environment: Some("production".into()),
            request_id: Some("req-custom-001".into()),
            ..Default::default()
        })),
        http: Some(Arc::new(HttpExtension::default())),
        meta: Some(Arc::new(MetaExtension {
            entity_type: Some("tool".into()),
            entity_name: Some("get_compensation".into()),
            tags: ["pii".to_string()].into(),
            ..Default::default()
        })),
        ..Default::default()
    };

    let (result, bg) = mgr
        .invoke_named::<ToolPreInvokeHook>("tool_pre_invoke", payload, ext, None)
        .await;

    println!();
    if result.continue_processing {
        println!("  Result: ALLOWED (custom payload passed through WASM)");
    } else {
        println!(
            "  Result: DENIED — {}",
            result.violation.as_ref().unwrap().reason
        );
    }
    bg.wait_for_background_tasks().await;

    // --- Scenario 2: Another tool invocation ---
    println!("\n=== Scenario 2: list_departments (custom payload, different tool) ===\n");
    let payload = ToolInvokePayload {
        tool_name: "list_departments".into(),
        user: "bob".into(),
        arguments: "".into(),
    };

    let ext = Extensions {
        request: Some(Arc::new(RequestExtension {
            environment: Some("staging".into()),
            request_id: Some("req-custom-002".into()),
            ..Default::default()
        })),
        meta: Some(Arc::new(MetaExtension {
            entity_type: Some("tool".into()),
            entity_name: Some("list_departments".into()),
            ..Default::default()
        })),
        ..Default::default()
    };

    let (result, bg) = mgr
        .invoke_named::<ToolPreInvokeHook>("tool_pre_invoke", payload, ext, None)
        .await;

    println!();
    if result.continue_processing {
        println!("  Result: ALLOWED (custom payload passed through WASM)");
    } else {
        println!(
            "  Result: DENIED — {}",
            result.violation.as_ref().unwrap().reason
        );
    }
    bg.wait_for_background_tasks().await;

    println!("\n=== Demo complete ===");
}
