use wit_bindgen::generate;
use std::fs;

generate!({
    world: "plugin",
});

use exports::example::plugin::policy::Guest;

struct Component;

impl Guest for Component {
    fn check_key(json: String, key: String) -> String {
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap_or_default();
        match parsed.get(&key).and_then(|v| v.as_str()) {
            Some(value) if value.to_lowercase().contains("deny") => "deny".to_string(),
            _ => "allow".to_string(),
        }
    }

    fn create_file(filename: String, content: String) -> String {
        match fs::write(&filename, content) {
            Ok(_) => format!("Success: Created file {}", filename),
            Err(e) => format!("Error: {}", e),
        }
    }
}



export!(Component);
