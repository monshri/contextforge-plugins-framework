// Location: ./crates/cpex-wasm-host/examples/wasm_fs_test_demo.rs
// Copyright 2026
// SPDX-License-Identifier: Apache-2.0
// Authors: Shriti Priya
//
// Filesystem Sandbox Enforcement Demo (Interactive)
//
// The user enters a file path to access (e.g. /etc/passwd, /tmp/notes.txt).
// The sandbox policy is loaded from config/config_fs_test.yaml — edit that
// file to change which paths are allowed.
//
// The fs-test plugin attempts to read the file inside the WASM sandbox.
// Whether the read succeeds depends entirely on the sandbox policy — the same
// plugin binary runs in all cases.
//
// Prerequisites:
//   cd crates/cpex-wasm-host && make build-test-plugins
//
// Run:
//   cargo run -p cpex-wasm-host --example wasm_fs_test_demo

use std::io::{self, Write};
use std::path::PathBuf;
use std::sync::Arc;

use cpex_core::cmf::{CmfHook, ContentPart, Message, MessagePayload, Role, ToolCall};
use cpex_core::config::parse_config;
use cpex_core::context::PluginContextTable;
use cpex_core::extensions::container::Extensions;
use cpex_core::extensions::meta::MetaExtension;
use cpex_core::manager::PluginManager;

use cpex_wasm_host::factory::WasmPluginFactory;

const CYAN_BOLD: &str = "\x1b[1;36m";
const WHITE: &str = "\x1b[97m";
const RED: &str = "\x1b[31m";
const DIM: &str = "\x1b[2m";
const RESET: &str = "\x1b[0m";

fn prompt(msg: &str) -> String {
    print!("{}", msg);
    io::stdout().flush().unwrap();
    let mut buf = String::new();
    io::stdin().read_line(&mut buf).unwrap();
    buf.trim().to_string()
}

fn print_result(result: &cpex_core::executor::PipelineResult) {
    if result.continue_processing {
        println!("\n  {}Result: ALLOWED — path is permitted by policy{}", WHITE, RESET);
    } else {
        let violation = result.violation.as_ref().unwrap();
        println!(
            "\n  {}Result: DENIED — {} [{}]{}",
            RED, violation.reason, violation.code, RESET,
        );
    }
}

async fn run_check(target_file: &str, config_path: &PathBuf, wasm_dir: &PathBuf) {
    println!(
        "\n{}Checking access to '{}'...{}",
        CYAN_BOLD, target_file, RESET
    );
    println!("{}  Policy loaded from: {}{}", DIM, config_path.display(), RESET);
    println!();

    let config_yaml = std::fs::read_to_string(config_path)
        .unwrap_or_else(|e| panic!("failed to read {}: {}", config_path.display(), e));
    let cpex_config = parse_config(&config_yaml)
        .unwrap_or_else(|e| panic!("failed to parse config: {}", e));

    let mgr = PluginManager::default();
    mgr.register_factory(
        "wasm://fs-test.wasm",
        Box::new(WasmPluginFactory::with_builtin_payloads(wasm_dir.clone())),
    );
    mgr.load_config(cpex_config).unwrap();
    mgr.initialize().await.unwrap();

    // Pass the target file path to the plugin via global context
    let mut ctx_table = PluginContextTable::new();
    ctx_table
        .global_state
        .insert("fs_target_path".into(), serde_json::json!(target_file));

    let payload = MessagePayload {
        message: Message {
            schema_version: cpex_core::cmf::constants::SCHEMA_VERSION.into(),
            role: Role::Assistant,
            content: vec![ContentPart::ToolCall {
                content: ToolCall {
                    tool_call_id: "tc_001".into(),
                    name: "fs_access_check".into(),
                    arguments: [("path".to_string(), serde_json::json!(target_file))].into(),
                    namespace: None,
                },
            }],
            channel: None,
        },
    };

    let extensions = Extensions {
        meta: Some(Arc::new(MetaExtension {
            entity_type: Some("tool".into()),
            entity_name: Some("fs_access_check".into()),
            ..Default::default()
        })),
        ..Default::default()
    };

    println!("{}  Payload: {}{}", DIM, serde_json::to_string_pretty(&payload).unwrap_or_else(|_| format!("{:?}", payload)), RESET);

    let (result, bg) = mgr
        .invoke_named::<CmfHook>(
            "cmf.tool_pre_invoke",
            payload,
            extensions,
            Some(ctx_table),
        )
        .await;

    // Print what the plugin observed
    for ctx in result.context_table.local_states.values() {
        if let Some(path) = ctx.get("fs_target_path") {
            println!("  Target file     : {}", path);
        }
        if let Some(success) = ctx.get("fs_read_success") {
            println!("  Read succeeded  : {}", success);
        }
        if let Some(len) = ctx.get("fs_read_length") {
            println!("  Bytes read      : {}", len);
        }
        if let Some(err) = ctx.get("fs_read_error") {
            println!("  Sandbox error   : {}", err);
        }
    }

    print_result(&result);
    bg.wait_for_background_tasks().await;
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("warn".parse().unwrap()),
        )
        .init();

    let wasm_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("wasm");
    let config_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("config/config_fs_test.yaml");

    println!("{}=== WASM Filesystem Sandbox Enforcement Demo ==={}", CYAN_BOLD, RESET);
    println!();
    println!("The fs-test plugin attempts to read a file inside its WASM sandbox.");
    println!("Policy is loaded from config/config_fs_test.yaml — edit that file to change access rules.");
    println!("Type 'quit' to exit.\n");

    loop {
        // --- Get target file ---
        let target_file = prompt(&format!("{}File to access{} (e.g. /etc/passwd): ", CYAN_BOLD, RESET));
        if target_file.eq_ignore_ascii_case("quit") || target_file.is_empty() {
            break;
        }

        run_check(&target_file, &config_path, &wasm_dir).await;

        // --- Ask to continue ---
        println!();
        let again = prompt("Try another? [Y/n]: ");
        if again.eq_ignore_ascii_case("n") {
            break;
        }
        println!();
    }

    println!("\n{}=== Demo complete ==={}", CYAN_BOLD, RESET);
}
