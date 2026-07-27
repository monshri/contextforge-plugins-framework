// Location: ./crates/cpex-wasm-plugin/src/examples/mod.rs
// Copyright 2026
// SPDX-License-Identifier: Apache-2.0
// Authors: Shriti Priya

// Read this first — the simplest possible plugin (~40 lines)
pub mod minimal_plugin;

#[cfg(feature = "identity-checker")]
pub mod identity_checker;

#[cfg(feature = "header-injector")]
pub mod header_injector;

#[cfg(feature = "audit-logger")]
pub mod audit_logger;

#[cfg(feature = "token-attenuator")]
pub mod token_attenuator;

#[cfg(feature = "noop")]
pub mod noop;

#[cfg(feature = "fs-test")]
pub mod fs_test;

#[cfg(feature = "net-test")]
pub mod net_test;

#[cfg(feature = "env-test")]
pub mod env_test;

#[cfg(feature = "tool-invoke-checker")]
pub mod tool_invoke_checker;

#[cfg(feature = "compute-bench")]
pub mod compute_bench;

#[cfg(feature = "pii-guard")]
pub mod pii_guard;

#[cfg(feature = "audit-logger-custom")]
pub mod audit_logger_custom;

#[cfg(feature = "remote-authz")]
pub mod remote_authz;

#[cfg(feature = "fs-sandbox-demo")]
pub mod fs_sandbox_demo;

#[cfg(feature = "env-sandbox-demo")]
pub mod env_sandbox_demo;

#[cfg(feature = "resource-test")]
pub mod resource_test;

#[cfg(feature = "net-http-test")]
pub mod net_http_test;

