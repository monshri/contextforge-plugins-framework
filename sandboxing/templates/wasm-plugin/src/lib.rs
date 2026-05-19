//! CPEX WASM Plugin: {{project-name}}
//!
//! This plugin runs in a sandboxed WASM environment. You only need to write
//! standard Rust — the framework handles serialization and isolation.
//!
//! ## Quick guide
//!
//! 1. Define your payload struct(s) matching the hook payloads you handle.
//! 2. Write your logic inside the `handle_*` functions below.
//! 3. Run `make component` to build.
//! 4. Register the `.component.wasm` in your CPEX config YAML.

use serde::{Deserialize, Serialize};
use std::sync::Mutex;

wit_bindgen::generate!({
    world: "plugin",
    generate_all,
});

use exports::cpex::plugin::hooks::{Guest as HooksGuest, HookRequest, HookResult};
use exports::cpex::plugin::lifecycle::Guest as LifecycleGuest;
// use cpex::plugin::logging;
// use cpex::plugin::state;

// =============================================================================
// Plugin Configuration
// =============================================================================

/// Your plugin's configuration. This is deserialized from the `config:` section
/// of your plugin entry in the CPEX YAML config file.
///
/// Example YAML:
///   plugins:
///     - name: {{project-name}}
///       kind: "wasm://plugins/{{project-name}}.component.wasm"
///       hooks: [tool_pre_invoke]
///       config:
///         your_field: "value"
#[derive(Deserialize, Default)]
struct Config {
    // TODO: Add your configuration fields here.
    // Example:
    //   blocked_tools: Vec<String>,
    //   max_retries: u32,
}

static PLUGIN_CONFIG: Mutex<Option<Config>> = Mutex::new(None);

// =============================================================================
// Payload Types
// =============================================================================

/// Example payload for tool invocation hooks.
/// Replace or extend this to match the actual hook payloads you handle.
#[derive(Deserialize, Serialize)]
struct ToolInvokePayload {
    tool_name: String,
    user: String,
    arguments: String,
}

/// Violation structure returned when denying a request.
#[derive(Serialize)]
struct Violation {
    code: String,
    reason: String,
}

// =============================================================================
// Component Implementation
// =============================================================================

struct Component;

impl LifecycleGuest for Component {
    fn init(config_json: String) -> Result<(), String> {
        let cfg: Config = if config_json.is_empty() {
            Config::default()
        } else {
            serde_json::from_str(&config_json)
                .map_err(|e| format!("config parse error: {e}"))?
        };

        *PLUGIN_CONFIG.lock().unwrap() = Some(cfg);

        // logging::log(logging::Level::Info, "plugin initialized", "");
        Ok(())
    }

    fn shutdown() {
        *PLUGIN_CONFIG.lock().unwrap() = None;
        // logging::log(logging::Level::Info, "plugin shutdown", "");
    }
}

impl HooksGuest for Component {
    fn handle(request: HookRequest) -> HookResult {
        match request.hook_name.as_str() {
            "tool_pre_invoke" => handle_tool_pre_invoke(&request),
            "tool_post_invoke" => handle_tool_post_invoke(&request),
            // TODO: Add more hook handlers as needed.
            // "prompt_pre_fetch" => handle_prompt_pre_fetch(&request),
            _ => {
                // Unknown hooks pass through by default.
                HookResult::Allow
            }
        }
    }
}

// =============================================================================
// Hook Handlers — YOUR LOGIC GOES HERE
// =============================================================================

fn handle_tool_pre_invoke(request: &HookRequest) -> HookResult {
    // Parse the payload
    let payload: ToolInvokePayload = match serde_json::from_str(&request.payload_json) {
        Ok(p) => p,
        Err(e) => {
            // logging::log(logging::Level::Error, &format!("payload parse error: {e}"), "");
            return HookResult::Allow;
        }
    };

    // Access plugin config
    let _cfg = PLUGIN_CONFIG.lock().unwrap();

    // Access global pipeline state (shared across plugins in this invocation)
    // let _some_value = state::get_global("key_name");

    // TODO: Add your pre-invoke logic here.
    //
    // Examples:
    //
    //   Deny a request:
    //     let v = Violation { code: "forbidden".into(), reason: "Not allowed".into() };
    //     return HookResult::Deny(serde_json::to_string(&v).unwrap());
    //
    //   Modify the payload:
    //     let mut modified = payload;
    //     modified.arguments = "sanitized".into();
    //     return HookResult::Modify(serde_json::to_string(&modified).unwrap());
    //
    //   Write to shared state:
    //     state::set_global("my_key", "my_value");
    //
    //   Log something:
    //     logging::log(logging::Level::Info, "checked tool", "");

    let _ = payload; // Remove this line once you use the payload.
    HookResult::Allow
}

fn handle_tool_post_invoke(request: &HookRequest) -> HookResult {
    let payload: ToolInvokePayload = match serde_json::from_str(&request.payload_json) {
        Ok(p) => p,
        Err(_) => return HookResult::Allow,
    };

    // TODO: Add your post-invoke logic here (logging, metrics, cleanup).

    let _ = payload;
    HookResult::Allow
}

export!(Component);
