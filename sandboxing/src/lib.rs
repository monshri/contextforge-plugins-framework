pub mod policy_loader;

use wasmtime::component::*;
use wasmtime_wasi::{WasiCtx, WasiView};

wasmtime::component::bindgen!({
    path: "plugin/wit",
    world: "plugin",
});

pub struct MyState {
    pub wasi: WasiCtx,
    pub table: ResourceTable,
}

impl WasiView for MyState {
    fn ctx(&mut self) -> &mut WasiCtx {
        &mut self.wasi
    }
    
    fn table(&mut self) -> &mut ResourceTable {
        &mut self.table
    }
}
