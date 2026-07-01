// Location: ./crates/cpex-wasm-host/examples/wasm_plugin_demo.rs
// Copyright 2025
// SPDX-License-Identifier: Apache-2.0
// Authors: Shriti Priya
//
// Demonstrates invoking a WASM header-injector plugin through the PluginManager pipeline.
// The plugin modifies security labels and HTTP headers, and the host verifies the
// modified_extensions are propagated back correctly.

use std::path::PathBuf;
use std::sync::Arc;

use cpex_core::cmf::{CmfHook, ContentPart, Message, MessagePayload, Role, ToolCall};
use cpex_core::config::parse_config;
use cpex_core::extensions::{HttpExtension, RequestExtension, SecurityExtension};
use cpex_core::hooks::payload::{Extensions, MetaExtension};
use cpex_core::extensions::security::SubjectExtension;
use cpex_core::manager::PluginManager;

use cpex_wasm_host::factory::WasmPluginFactory;

#[tokio::main]
async fn main() {
    println!("=== WASM Header-Injector Plugin Demo ===\n");

    let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let config_path = crate_dir.join("config/config.yaml");
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

    // Build a test payload (assistant requesting a tool call)
    let payload = MessagePayload {
        message: Message {
            schema_version: cpex_core::cmf::constants::SCHEMA_VERSION.into(),
            role: Role::Assistant,
            content: vec![
                ContentPart::Text {
                    text: "Looking up compensation.".into(),
                },
                ContentPart::ToolCall {
                    content: ToolCall {
                        tool_call_id: "tc_001".into(),
                        name: "get_compensation".into(),
                        arguments: [("employee_id".to_string(), serde_json::json!(42))].into(),
                        namespace: None,
                    },
                },
            ],
            channel: None,
        },
    };

    // Build extensions with security + HTTP context
    let mut security = SecurityExtension::default();
    security.add_label("PII");
    security.add_label("HR_DATA");
    security.classification = Some("confidential".into());
    security.subject = Some(SubjectExtension {
        id: Some("alice".into()),
        subject_type: Some(cpex_core::extensions::security::SubjectType::User),
        roles: ["hr_admin".to_string()].into(),
        permissions: ["read_compensation".to_string()].into(),
        ..Default::default()
    });

    let mut http = HttpExtension::default();
    http.set_header("Authorization", "Bearer eyJ...");
    http.set_header("X-Request-ID", "req-abc-123");

    let ext = Extensions {
        request: Some(Arc::new(RequestExtension {
            environment: Some("production".into()),
            request_id: Some("req-abc-123".into()),
            ..Default::default()
        })),
        security: Some(Arc::new(security)),
        http: Some(Arc::new(http)),
        meta: Some(Arc::new(MetaExtension {
            entity_type: Some("tool".into()),
            entity_name: Some("get_compensation".into()),
            tags: ["pii".to_string(), "hr".to_string()].into(),
            ..Default::default()
        })),
        ..Default::default()
    };

    println!("--- Input extensions ---");
    println!("  Security labels: [\"PII\", \"HR_DATA\"]");
    println!("  HTTP headers: Authorization, X-Request-ID\n");

    // Invoke the header-injector plugin via pre-invoke hook
    println!("=== Invoking cmf.tool_pre_invoke (header-injector) ===\n");
    let (result, bg) = mgr
        .invoke_named::<CmfHook>(
            "cmf.tool_pre_invoke",
            payload,
            ext,
            None,
        )
        .await;

    println!();
    if result.continue_processing {
        println!("Result: ALLOWED");
        if let Some(ref modified_ext) = result.modified_extensions {
            if let Some(ref sec) = modified_ext.security {
                let labels: Vec<&String> = sec.labels.iter().collect();
                println!("  Modified labels: {:?}", labels);
            }
            if let Some(ref http) = modified_ext.http {
                println!("  Modified headers: {:?}", http.request_headers);
            }
        } else {
            println!("  (no extension modifications returned)");
        }
    } else {
        println!(
            "Result: DENIED — {}",
            result.violation.as_ref().unwrap().reason
        );
    }

    bg.wait_for_background_tasks().await;
    println!("\n=== Demo complete ===");
}
