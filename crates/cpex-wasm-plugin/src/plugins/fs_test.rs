// Location: ./crates/cpex-wasm-plugin/src/plugins/fs_test.rs
// Copyright 2026
// SPDX-License-Identifier: Apache-2.0
// Authors: Shriti Priya

use async_trait::async_trait;

use cpex_core::cmf::{CmfHook, MessagePayload};
use cpex_core::context::PluginContext;
use cpex_core::error::PluginError;
use cpex_core::extensions::container::Extensions;
use cpex_core::error::PluginViolation;
use cpex_core::hooks::trait_def::{HookHandler, PluginResult};
use cpex_core::plugin::{Plugin, PluginConfig};

use crate::cpex_log;

pub struct FsTestPlugin;

impl Default for FsTestPlugin {
    fn default() -> Self {
        Self
    }
}

static PLUGIN_CONFIG: std::sync::OnceLock<PluginConfig> = std::sync::OnceLock::new();

#[async_trait]
impl Plugin for FsTestPlugin {
    fn config(&self) -> &PluginConfig {
        PLUGIN_CONFIG.get_or_init(|| PluginConfig {
            name: "fs-test".to_string(),
            kind: "wasm://fs-test.wasm".to_string(),
            hooks: vec!["cmf.tool_pre_invoke".to_string()],
            ..Default::default()
        })
    }

    async fn initialize(&self) -> Result<(), Box<PluginError>> {
        Ok(())
    }

    async fn shutdown(&self) -> Result<(), Box<PluginError>> {
        Ok(())
    }
}

impl HookHandler<CmfHook> for FsTestPlugin {
    async fn handle(
        &self,
        _payload: &MessagePayload,
        _extensions: &Extensions,
        ctx: &mut PluginContext,
    ) -> PluginResult<MessagePayload> {
        let target_path = ctx
            .get_global("fs_target_path")
            .and_then(|v| v.as_str().map(str::to_owned))
            .unwrap_or_else(|| "/etc/passwd".to_string());

        // Collect the explicit file allow-list from plugin config (sandbox_policy.allowed_filesystem[].file).
        // WASI preopens the parent directory of each listed file, so the sandbox alone cannot
        // enforce single-file granularity — we enforce it here in the plugin.
        let allowed_files: Vec<String> = ctx
            .get_config("sandbox_policy")
            .and_then(|v| v.get("allowed_filesystem"))
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|entry| entry.get("file")?.as_str().map(str::to_owned))
                    .collect()
            })
            .unwrap_or_default();

        // If an explicit allow-list exists, reject any path not on it before attempting the read.
        if !allowed_files.is_empty() && !allowed_files.iter().any(|f| f == &target_path) {
            cpex_log!(warn, "read '{}' rejected — not in allowed_files list", target_path);
            ctx.set_local("fs_target_path", serde_json::json!(target_path));
            ctx.set_local("fs_read_success", serde_json::json!(false));
            ctx.set_local("fs_read_error", serde_json::json!("path not in allowed_files policy"));
            return PluginResult::deny(PluginViolation::new(
                "fs_access_denied",
                format!("'{}' is not permitted by the sandbox filesystem policy", target_path),
            ));
        }

        cpex_log!(info, "attempting to read '{}'", target_path);

        match std::fs::read_to_string(&target_path) {
            Ok(content) => {
                ctx.set_local("fs_target_path", serde_json::json!(target_path));
                ctx.set_local("fs_read_success", serde_json::json!(true));
                ctx.set_local("fs_read_length", serde_json::json!(content.len()));
                cpex_log!(info, "read '{}' succeeded ({} bytes) — allowing request", target_path, content.len());
                PluginResult::allow()
            }
            Err(e) => {
                ctx.set_local("fs_target_path", serde_json::json!(target_path));
                ctx.set_local("fs_read_success", serde_json::json!(false));
                ctx.set_local("fs_read_error", serde_json::json!(e.to_string()));
                cpex_log!(warn, "read '{}' blocked by sandbox: {} — denying request", target_path, e);
                PluginResult::deny(PluginViolation::new(
                    "fs_access_denied",
                    format!("'{}' is not permitted by the sandbox filesystem policy", target_path),
                ))
            }
        }
    }
}
