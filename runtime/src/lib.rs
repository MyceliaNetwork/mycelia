mod function;

use std::path::Path;

use wasmtime::{component::*, Engine};

use wasmtime_wasi::preview2::{Table, WasiCtx, WasiCtxBuilder, WasiView};

pub fn create_function_component(
    path: &Path,
    engine: &Engine,
) -> Result<Component, wasmtime::Error> {
    Component::from_file(engine, path)
}

pub mod engine {
    use wasmtime::{Config, Engine};

    pub fn get_config() -> Config {
        let mut cfg = Config::new();
        cfg.wasm_component_model(true);
        cfg.async_support(true);
        cfg
    }
    pub fn new_engine() -> Result<Engine, wasmtime::Error> {
        Engine::new(&get_config())
    }
}

struct ServerWasiView {
    table: Table,
    ctx: WasiCtx,
}

impl ServerWasiView {
    pub fn new() -> Self {
        let mut table = Table::new();
        let ctx = WasiCtxBuilder::new()
            .inherit_stdio()
            .build(&mut table)
            .unwrap();

        Self { table, ctx }
    }
}

impl WasiView for ServerWasiView {
    fn table(&self) -> &Table {
        &self.table
    }

    fn table_mut(&mut self) -> &mut Table {
        &mut self.table
    }

    fn ctx(&self) -> &WasiCtx {
        &self.ctx
    }

    fn ctx_mut(&mut self) -> &mut WasiCtx {
        &mut self.ctx
    }
}
