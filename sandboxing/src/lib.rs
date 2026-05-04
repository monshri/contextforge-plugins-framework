//! Host-side library for the sandboxing crate.
//!
//! Wires together:
//!   - WASI preview 2 (filesystem, stdio, env)  via wasmtime-wasi
//!   - WASI HTTP outbound                       via wasmtime-wasi-http,
//!     with an allowlist enforced in WasiHttpView::send_request
//!   - The plugin's own custom interface        via bindgen!
//!
//! The allowlist behaviour is the same shape capsule-core uses
//! (see https://github.com/mavdol/capsule, crates/capsule-core/src/wasm/state.rs):
//! check the request URL's host against a configured set of patterns,
//! reject with HttpRequestDenied if it doesn't match, otherwise delegate
//! to default_send_request.

pub mod policy_loader;

use wasmtime::component::ResourceTable;
use wasmtime_wasi::{WasiCtx, WasiView};
use wasmtime_wasi_http::bindings::http::types::ErrorCode;
use wasmtime_wasi_http::types::{
    HostFutureIncomingResponse, OutgoingRequestConfig, default_send_request,
};
use wasmtime_wasi_http::{HttpResult, WasiHttpCtx, WasiHttpView};
use http_body_util::combinators::BoxBody;
use bytes::Bytes;

use crate::policy_loader::{is_host_allowed, ResourceLimits};

// Plugin world bindings. This regenerates against your WIT every build.
// `async: true` is required so the plugin's exports return futures, which
// matches the async wasmtime config we need for wasmtime-wasi-http.
wasmtime::component::bindgen!({
    path: "plugin/wit",
    world: "plugin",
    async: true,
});

/// Shared per-instance state. One of these lives in each Store.
pub struct MyState {
    pub wasi: WasiCtx,
    pub http: WasiHttpCtx,
    pub table: ResourceTable,
    /// Hostnames the guest is allowed to reach. Cloned out of the policy
    /// at startup so the WasiHttpView impl can read it without locking.
    pub allowed_hosts: Vec<String>,
    /// Resource limits for this plugin instance
    pub resource_limits: ResourceLimits,
    /// Start time for tracking wall clock timeout
    pub start_time: std::time::Instant,
}

impl WasiView for MyState {
    fn ctx(&mut self) -> &mut WasiCtx {
        &mut self.wasi
    }
    fn table(&mut self) -> &mut ResourceTable {
        &mut self.table
    }
}

impl WasiHttpView for MyState {
    fn ctx(&mut self) -> &mut WasiHttpCtx {
        &mut self.http
    }

    fn table(&mut self) -> &mut ResourceTable {
        &mut self.table
    }

    /// Allowlist gate for outbound HTTP. Runs on every request the guest
    /// makes via wasi:http/outgoing-handler.
    fn send_request(
        &mut self,
        request: http::Request<BoxBody<Bytes, ErrorCode>>,
        config: OutgoingRequestConfig,
    ) -> HttpResult<HostFutureIncomingResponse> {
        let host = request.uri().host().unwrap_or("");
        if !is_host_allowed(host, &self.allowed_hosts) {
            // Tells the guest that the request was denied. The guest sees
            // this as a wasi:http error-code, not a panic.
            return Err(ErrorCode::HttpRequestDenied.into());
        }
        Ok(default_send_request(request, config))
    }
}