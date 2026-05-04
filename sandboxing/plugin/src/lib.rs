use wit_bindgen::generate;
use std::fs;

generate!({
    world: "plugin",
    generate_all,
});

use exports::example::plugin::policy::Guest;
use wasi::http::types::{Method, Scheme, OutgoingRequest, OutgoingBody, Fields};
use wasi::http::outgoing_handler;

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

    fn make_http_request(url: String) -> String {
        // Parse the URL to extract components
        let url_parts = parse_url(&url);
        
        // Create outgoing request
        let headers = Fields::new();
        let request = OutgoingRequest::new(headers);
        
        // Set the method (GET)
        request.set_method(&Method::Get).ok();
        
        // Set the scheme (http or https)
        request.set_scheme(Some(&url_parts.scheme)).ok();
        
        // Set the authority (host:port)
        request.set_authority(Some(&url_parts.authority)).ok();
        
        // Set the path
        request.set_path_with_query(Some(&url_parts.path)).ok();
        
        // Create empty body for GET request
        let outgoing_body = request.body().expect("Failed to get request body");
        OutgoingBody::finish(outgoing_body, None).expect("Failed to finish body");
        
        // Send the request
        match outgoing_handler::handle(request, None) {
            Ok(response) => {
                // Wait for the response
                let incoming_response = response.subscribe().block();
                let incoming_response = response.get().expect("Response not ready")
                    .expect("Request failed")
                    .expect("Request error");
                
                // Get the response body
                let response_body = incoming_response.consume().expect("Failed to consume response");
                let input_stream = response_body.stream().expect("Failed to get stream");
                
                // Read the body
                let mut body_bytes = Vec::new();
                loop {
                    match input_stream.blocking_read(8192) {
                        Ok(chunk) => {
                            if chunk.is_empty() {
                                break;
                            }
                            body_bytes.extend_from_slice(&chunk);
                        }
                        Err(_) => break,
                    }
                }
                
                String::from_utf8(body_bytes).unwrap_or_else(|_| "Invalid UTF-8 response".to_string())
            }
            Err(e) => format!("ERROR: {:?}", e),
        }
    }

    fn get_env_var(var_name: String) -> String {
        match std::env::var(&var_name) {
            Ok(value) => format!("{}={}", var_name, value),
            Err(_) => format!("Environment variable '{}' not found or not allowed", var_name),
        }
    }
}

// Simple URL parser
struct UrlParts {
    scheme: Scheme,
    authority: String,
    path: String,
}

fn parse_url(url: &str) -> UrlParts {
    let url = url.trim();
    
    // Parse scheme
    let (scheme, rest) = if url.starts_with("https://") {
        (Scheme::Https, &url[8..])
    } else if url.starts_with("http://") {
        (Scheme::Http, &url[7..])
    } else {
        (Scheme::Http, url)
    };
    
    // Split authority and path
    let (authority, path) = if let Some(pos) = rest.find('/') {
        (rest[..pos].to_string(), rest[pos..].to_string())
    } else {
        (rest.to_string(), "/".to_string())
    };
    
    UrlParts {
        scheme,
        authority,
        path,
    }
}

export!(Component);
