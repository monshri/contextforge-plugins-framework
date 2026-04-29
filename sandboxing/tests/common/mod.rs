use sandboxing::{MyState, Plugin};
use sandboxing::policy_loader::{load_policy, configure_wasi_from_policy};
use wasmtime::{Config, Engine, Store};
use wasmtime::component::{Component, Linker, ResourceTable};

pub fn setup_plugin() -> Result<(Store<MyState>, Plugin), Box<dyn std::error::Error>> {
    let mut config = Config::new();
    config.wasm_component_model(true);
    
    let engine = Engine::new(&config)?;
    let mut linker = Linker::new(&engine);
    wasmtime_wasi::add_to_linker_sync(&mut linker)?;
    
    let policy = load_policy("config/policy.yaml")?;
    let wasi = configure_wasi_from_policy(&policy)?;
    
    let state = MyState {
        wasi,
        table: ResourceTable::new(),
    };
    let mut store = Store::new(&engine, state);
    
    let component = Component::from_file(&engine, "plugin/target/wasm32-wasip2/release/plugin.wasm")?;
    let plugin = Plugin::instantiate(&mut store, &component, &linker)?;
    
    Ok((store, plugin))
}
