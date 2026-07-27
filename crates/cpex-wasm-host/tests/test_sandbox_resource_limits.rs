// Location: ./crates/cpex-wasm-host/tests/test_sandbox_resource_limits.rs
// Copyright 2026
// SPDX-License-Identifier: Apache-2.0
// Authors: Shriti Priya
//
//! Integration tests: verifies resource limits actually trap a running WASM plugin.
//!
//! Each test loads the resource-test plugin with a deliberately tiny limit,
//! invokes it in the mode that exercises that limit, and asserts the invocation
//! returns the expected error variant — proving the limit is enforced at runtime,
//! not just parsed from YAML.
//!
//! Requires: `wasm/resource-test.wasm` built with `make build-test-plugins`

use std::path::PathBuf;
use std::sync::Once;

use cpex_core::cmf::constants::SCHEMA_VERSION;
use cpex_core::cmf::{ContentPart, Message, MessagePayload, Role, ToolCall};
use cpex_core::context::PluginContext;
use cpex_core::extensions::container::Extensions;

use cpex_wasm_host::conversions::{
    native_context_to_wit, native_extensions_to_wit, native_payload_to_wit,
};
use cpex_wasm_host::policy_loader::{ResourceLimits, SandboxPolicy};
use cpex_wasm_host::sandbox_manager::{SandboxManager, SharedEngine};

static INIT: Once = Once::new();
fn init_tracing() {
    INIT.call_once(|| {
        tracing_subscriber::fmt()
            .with_test_writer()
            .with_env_filter("info")
            .init();
    });
}

fn wasm_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("wasm/resource-test.wasm")
}

fn make_payload(mode: &str) -> MessagePayload {
    let mut args = std::collections::HashMap::new();
    args.insert("mode".to_string(), serde_json::json!(mode));

    MessagePayload {
        message: Message {
            schema_version: SCHEMA_VERSION.into(),
            role: Role::Assistant,
            content: vec![ContentPart::ToolCall {
                content: ToolCall {
                    tool_call_id: "tc_resource".into(),
                    name: "resource_test".into(),
                    arguments: args,
                    namespace: None,
                },
            }],
            channel: None,
        },
    }
}

async fn load_with_limits(resources: ResourceLimits) -> SandboxManager {
    let path = wasm_path();
    assert!(
        path.exists(),
        "WASM binary not found: {}. Run `make build-test-plugins` from crates/cpex-wasm-host first.",
        path.display()
    );

    let policy = SandboxPolicy {
        resources,
        ..Default::default()
    };

    let shared = SharedEngine::new().unwrap();
    let mut mgr = SandboxManager::with_shared_engine(&shared);
    mgr.load_wasmplugin(&path, Some(&policy), "resource-test")
        .await
        .unwrap();
    mgr
}

// ── Fuel ─────────────────────────────────────────────────────────────────────

#[tokio::test]
#[ignore = "requires pre-built WASM plugins — run `make build-test-plugins` first"]
async fn test_fuel_limit_traps_plugin() {
    init_tracing();

    // 10 000 fuel units — enough to instantiate but far too little for the loop.
    let mut mgr = load_with_limits(ResourceLimits {
        max_fuel: Some(10_000),
        max_execution_time_ms: Some(10_000),
        ..Default::default()
    })
    .await;

    let payload = make_payload("burn_fuel");
    let wit_payload = cpex_wasm_host::sandbox_manager::types::HookPayload::Cmf(
        native_payload_to_wit(&payload),
    );
    let wit_ext = native_extensions_to_wit(&Extensions::default());
    let wit_ctx = native_context_to_wit(&PluginContext::default());

    let err = mgr
        .invoke("cmf.tool_pre_invoke", wit_payload, wit_ext, wit_ctx)
        .await
        .unwrap_err();

    let msg = err.to_string();
    assert!(
        msg.contains("fuel") || msg.contains("all fuel consumed"),
        "expected fuel exhaustion error, got: {}",
        msg
    );
}

// ── Epoch timeout ─────────────────────────────────────────────────────────────

#[tokio::test]
#[ignore = "requires pre-built WASM plugins — run `make build-test-plugins` first"]
async fn test_epoch_timeout_traps_plugin() {
    init_tracing();

    // 50ms timeout — the infinite loop will be interrupted almost immediately.
    let mut mgr = load_with_limits(ResourceLimits {
        max_execution_time_ms: Some(50),
        max_fuel: Some(u64::MAX),
        ..Default::default()
    })
    .await;

    let payload = make_payload("infinite_loop");
    let wit_payload = cpex_wasm_host::sandbox_manager::types::HookPayload::Cmf(
        native_payload_to_wit(&payload),
    );
    let wit_ext = native_extensions_to_wit(&Extensions::default());
    let wit_ctx = native_context_to_wit(&PluginContext::default());

    let err = mgr
        .invoke("cmf.tool_pre_invoke", wit_payload, wit_ext, wit_ctx)
        .await
        .unwrap_err();

    let msg = err.to_string();
    assert!(
        msg.contains("epoch") || msg.contains("interrupt") || msg.contains("deadline"),
        "expected epoch deadline error, got: {}",
        msg
    );
}

// ── Memory cap ───────────────────────────────────────────────────────────────

#[tokio::test]
#[ignore = "requires pre-built WASM plugins — run `make build-test-plugins` first"]
async fn test_memory_limit_traps_plugin() {
    init_tracing();

    // 5 MB — the alloc_memory mode allocates in 1 MB chunks until OOM.
    let mut mgr = load_with_limits(ResourceLimits {
        max_memory_bytes: Some(5 * 1024 * 1024),
        max_fuel: Some(u64::MAX),
        max_execution_time_ms: Some(10_000),
        ..Default::default()
    })
    .await;

    let payload = make_payload("alloc_memory");
    let wit_payload = cpex_wasm_host::sandbox_manager::types::HookPayload::Cmf(
        native_payload_to_wit(&payload),
    );
    let wit_ext = native_extensions_to_wit(&Extensions::default());
    let wit_ctx = native_context_to_wit(&PluginContext::default());

    let err = mgr
        .invoke("cmf.tool_pre_invoke", wit_payload, wit_ext, wit_ctx)
        .await
        .unwrap_err();

    let msg = err.to_string();
    assert!(
        msg.contains("memory") || msg.contains("grow") || msg.contains("trap"),
        "expected memory limit error, got: {}",
        msg
    );
}
