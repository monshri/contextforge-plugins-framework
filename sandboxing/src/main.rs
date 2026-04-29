
// Potential Sandbox Manager
use wasmtime::component::*;
use wasmtime::{Config, Engine, Store};
use wasmtime_wasi::{WasiCtx, WasiCtxBuilder, WasiView};
use wasmtime_wasi::{DirPerms, FilePerms};

wasmtime::component::bindgen!({
    path: "plugin/wit",
    world: "plugin",
});

// Wrapper struct that implements WasiView
struct MyState {
    wasi: WasiCtx,
    table: ResourceTable,
}

impl WasiView for MyState {
    fn ctx(&mut self) -> &mut WasiCtx {
        &mut self.wasi
    }
    
    fn table(&mut self) -> &mut ResourceTable {
        &mut self.table
    }
}



fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut config = Config::new();
    config.wasm_component_model(true);
    
    let engine = Engine::new(&config)?;
    let mut linker = Linker::new(&engine);
    wasmtime_wasi::add_to_linker_sync(&mut linker)?;
    
    // let wasi = WasiCtxBuilder::new().inherit_stdio().preopened_dir("./host-directory", ".", DirPerms::all(), FilePerms::all());
    let wasi = WasiCtxBuilder::new().inherit_stdio().preopened_dir("./data", ".", DirPerms::all(), FilePerms::all())?.build();

    let state = MyState { 
        wasi,
        table: ResourceTable::new(),
    };
    let mut store = Store::new(&engine, state);
    
    let component = Component::from_file(&engine, "plugin/target/wasm32-wasip2/release/plugin.wasm")?;
    let plugin = Plugin::instantiate(&mut store, &component, &linker)?;
    
    let json = r#"{"status": "please allow this"}"#;
    let result = plugin.example_plugin_policy().call_check_key(&mut store, json, "status")?;
    
    println!("Result: {}", result);
    Ok(())
}
