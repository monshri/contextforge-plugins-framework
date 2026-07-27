// Location: ./crates/cpex-wasm-plugin/src/examples/minimal_plugin.rs
// Copyright 2026
// SPDX-License-Identifier: Apache-2.0
// Authors: Shriti Priya
//
// Minimal Plugin — the simplest possible CPEX WASM plugin.
//
// This is the "hello world" reference. Read this first before looking at the
// other examples. It shows the complete pattern in ~40 lines:
//
//   1. Import everything from the prelude
//   2. Define your plugin struct (must impl Default)
//   3. Implement Plugin (lifecycle — initialize/shutdown)
//   4. Implement HookHandler<H> (your logic)
//   5. Call register_wasm_plugin! once
//
// To build this as a .wasm binary, add a feature flag for it in Cargo.toml,
// declare the module in mod.rs, add a registration block in lib.rs, and run:
//
//   cargo build --target wasm32-wasip2 --release \
//     --features minimal-plugin --no-default-features
//
// See BUILDING.md for the full guide.

use async_trait::async_trait;

use cpex_core::cmf::{CmfHook, MessagePayload};
use cpex_core::context::PluginContext;
use cpex_core::error::PluginError;
use cpex_core::extensions::container::Extensions;
use cpex_core::hooks::trait_def::{HookHandler, PluginResult};
use cpex_core::plugin::{Plugin, PluginConfig};

use crate::cpex_log;

// ---------------------------------------------------------------------------
// Step 1: Define your plugin struct
// ---------------------------------------------------------------------------

pub struct MinimalPlugin;

impl Default for MinimalPlugin {
    fn default() -> Self {
        Self
    }
}

// ---------------------------------------------------------------------------
// Step 2: Implement Plugin (lifecycle hooks)
// ---------------------------------------------------------------------------

static PLUGIN_CONFIG: std::sync::OnceLock<PluginConfig> = std::sync::OnceLock::new();

#[async_trait]
impl Plugin for MinimalPlugin {
    fn config(&self) -> &PluginConfig {
        PLUGIN_CONFIG.get_or_init(|| PluginConfig {
            name: "minimal-plugin".to_string(),
            // Must match the .wasm filename referenced in config.yaml
            kind: "wasm://minimal-plugin.wasm".to_string(),
            // The hook name(s) this plugin handles
            hooks: vec!["cmf.tool_pre_invoke".to_string()],
            ..Default::default()
        })
    }

    async fn initialize(&self) -> Result<(), Box<PluginError>> {
        cpex_log!(info, "minimal-plugin: initialized");
        Ok(())
    }

    async fn shutdown(&self) -> Result<(), Box<PluginError>> {
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Step 3: Implement HookHandler — your plugin logic goes here
// ---------------------------------------------------------------------------

impl HookHandler<CmfHook> for MinimalPlugin {
    async fn handle(
        &self,
        payload: &MessagePayload,
        _extensions: &Extensions,
        _ctx: &mut PluginContext,
    ) -> PluginResult<MessagePayload> {
        // Log every tool call that passes through
        let tool_name = payload
            .message
            .get_tool_calls()
            .first()
            .map(|tc| tc.name.as_str())
            .unwrap_or("unknown");

        cpex_log!(info, "minimal-plugin: saw tool call '{}'", tool_name);

        // Allow all requests through — modify this to add your logic
        PluginResult::allow()
    }
}

// ---------------------------------------------------------------------------
// Step 4: Wire the plugin to the WASM entry point (done in lib.rs)
//
//   register_wasm_plugin!(MinimalPlugin, [CmfHook]);
//
// This is the one-liner that generates all the WIT glue. You never need to
// touch anything below this line in your own plugin.
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use cpex_core::cmf::constants::SCHEMA_VERSION;
    use cpex_core::cmf::{CmfHook, ContentPart, Message, MessagePayload, Role, ToolCall};
    use cpex_core::context::PluginContext;
    use cpex_core::extensions::container::Extensions;
    use cpex_core::hooks::trait_def::{HookHandler, PluginResult};

    use super::MinimalPlugin;

    fn make_payload(tool: &str) -> MessagePayload {
        MessagePayload {
            message: Message {
                schema_version: SCHEMA_VERSION.into(),
                role: Role::Assistant,
                content: vec![ContentPart::ToolCall {
                    content: ToolCall {
                        tool_call_id: format!("tc_{}", tool),
                        name: tool.into(),
                        arguments: Default::default(),
                        namespace: None,
                    },
                }],
                channel: None,
            },
        }
    }

    #[tokio::test]
    async fn test_allows_all_tool_calls() {
        let payload = make_payload("any_tool");
        let ext = Extensions::default();
        let mut ctx = PluginContext::default();

        let result: PluginResult<_> =
            <MinimalPlugin as HookHandler<CmfHook>>::handle(
                &MinimalPlugin, &payload, &ext, &mut ctx,
            ).await;

        assert!(result.continue_processing, "minimal plugin should always allow");
    }
}
