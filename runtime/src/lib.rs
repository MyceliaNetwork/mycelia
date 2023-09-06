use std::path::Path;

use wasmtime::{component::*, Engine};

use wasmtime_wasi::preview2::{WasiCtxBuilder, WasiView,command::Command, Table, WasiCtx};

bindgen!({
    path: "../guests/function/wit/function.wit",
    async: true
});

fn main() {
    println!("Hello, world!");
}

pub fn create_function_component(path : &Path, engine : &Engine) -> Result<Component, wasmtime::Error> {
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
        let ctx = WasiCtxBuilder::new().inherit_stdio().build(&mut table).unwrap();
        Self {
            table, ctx
        }
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

#[cfg(test)]
mod test {
    use wasmtime::{component::{Component, Linker}, Store};
    use wasmtime_wasi::preview2::{WasiCtxBuilder, Table, WasiView, command::{add_to_linker, self}};

    use crate::{engine::new_engine, FunctionWorld, ServerWasiView};

    #[tokio::test]
    async fn it_builds_component() {
        let engine = new_engine().unwrap();
        let mut linker = Linker::new(&engine);

        let component = Component::from_file(&engine, "../components/function-component.wasm").unwrap();

        let mut table = Table::new();
        let mut wasi_ctx = WasiCtxBuilder::new();

        let wasi_view = ServerWasiView::new();

        let mut store = Store::new(&engine, wasi_view);


        // Add the command world (aka WASI CLI) to the linker
        add_to_linker(&mut linker);

        let (bindings, instance) = FunctionWorld::instantiate_async(&mut store, &component, &linker).await.unwrap();
        let result = bindings.call_init(&mut store).await;
        assert!(result.is_ok());
    }
}
