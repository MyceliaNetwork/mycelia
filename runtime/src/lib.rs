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
        let mut ctx = WasiCtxBuilder::new()
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

// #[cfg(test)]
// mod test {

//     // NOTE
//     // These tests will always fail unless you `$make` the new components
//     use wasmtime::{
//         component::{Component, Linker, Resource},
//         Store,
//     };
//     use wasmtime_wasi::preview2::{command::add_to_linker, Table, WasiCtxBuilder};

//     use crate::{
//         engine::new_engine,
//         mycelia::execution::function_interface::{self, Host},
//         Cool, FooBar, FunctionWorld, FunctionWorldImports, HostCool, ServerWasiView,
//     };

//     #[tokio::test]
//     async fn it_builds_component() {
//         #[async_trait::async_trait]
//         impl HostCool for ServerWasiView {
//             async fn new(&mut self) -> anyhow::Result<Resource<Cool>> {
//                 Ok(Resource::new_own(80))
//             }

//             fn drop(&mut self, _val: Resource<Cool>) -> anyhow::Result<()> {
//                 Ok(())
//             }

//             async fn name(&mut self, val: Resource<Cool>) -> anyhow::Result<String> {
//                 Ok(format!("basil_hazel {}", val.rep()))
//             }
//         }

//         impl FunctionWorldImports for ServerWasiView {};

//         impl Host for ServerWasiView {};

//         let engine = new_engine().unwrap();
//         let mut linker = Linker::new(&engine);

//         let component =
//             Component::from_file(&engine, "../components/function-component.wasm").unwrap();

//         let wasi_view = ServerWasiView::new();

//         let mut store = Store::new(&engine, wasi_view);

//         // Add the command world (aka WASI CLI) to the linker
//         let _ = add_to_linker(&mut linker).unwrap();

//         // Add the Resource to the FunctionWorld(?)
//         FunctionWorld::add_to_linker(&mut linker, |f| f);

//         let (bindings, _instance) =
//             FunctionWorld::instantiate_async(&mut store, &component, &linker)
//                 .await
//                 .unwrap();

//         let result = bindings.call_init(&mut store).await;
//         assert!(result.is_ok());

//         let foo: function_interface::FooBar = FooBar {
//             name: "Hazel".into(),
//         };

//         let result = bindings.call_test(&mut store, &foo).await.unwrap();
//         assert_eq!(result, "Hello, Hazel!");

//         let out = bindings
//             .call_test_resource(&mut store, Resource::new_own(80))
//             .await
//             .unwrap();
//         assert_eq!(out, "basil_hazel 80");

//     }
// }
