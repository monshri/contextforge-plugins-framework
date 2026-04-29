use serde::Deserialize;
use std::fs;
use wasmtime_wasi::{DirPerms, FilePerms, WasiCtx, WasiCtxBuilder};



#[derive(Debug, Deserialize)]
pub struct PolicyConfig {
    pub plugin: PluginConfig,
}

#[derive(Debug, Deserialize)]
pub struct PluginConfig {
    pub name: String,
    pub sandbox: SandboxConfig,
}

#[derive(Debug, Deserialize)]
pub struct SandboxConfig {
    #[serde(rename = "type")]
    pub sandbox_type: String,
    pub wasm_version: String,
    pub policy: Policy,
}

#[derive(Debug, Deserialize)]
pub struct Policy {
    pub dir_name: Vec<String>,
    pub permissions: Permissions,
}

#[derive(Debug, Deserialize)]
pub struct Permissions {
    pub dir: Vec<String>,
    pub file: Vec<String>,
}

pub fn load_policy(path: &str) -> Result<PolicyConfig, Box<dyn std::error::Error>> {
    let content = fs::read_to_string(path)?;
    let policy: PolicyConfig = serde_yaml::from_str(&content)?;
    Ok(policy)
}

pub fn configure_wasi_from_policy(policy: &PolicyConfig) -> Result<WasiCtx, Box<dyn std::error::Error>> {
    let dir_perms = parse_dir_permissions(&policy.plugin.sandbox.policy.permissions.dir);
    let file_perms = parse_file_permissions(&policy.plugin.sandbox.policy.permissions.file);
    
    let mut builder = WasiCtxBuilder::new();
    builder.inherit_stdio();
    
    for dir in &policy.plugin.sandbox.policy.dir_name {
        builder.preopened_dir(dir, ".", dir_perms, file_perms)?;
    }
    
    Ok(builder.build())
}


fn parse_dir_permissions(perms: &[String]) -> DirPerms {
    if perms.contains(&"mutate".to_string()) {
        DirPerms::all()
    } else {
        DirPerms::READ
    }
}

fn parse_file_permissions(perms: &[String]) -> FilePerms {
    if perms.contains(&"write".to_string()) {
        FilePerms::all()
    } else {
        FilePerms::READ
    }
}
