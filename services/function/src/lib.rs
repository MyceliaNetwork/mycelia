use wasmtime::component::*;

bindgen!({
  path: "../../guests/function/wit/function.wit",
  async: true
});

pub mod types {
    pub type HttpRequest = crate::mycelia::execution::types::HttpRequest;
    pub type HttpResponse = crate::mycelia::execution::types::HttpResponse;
    pub type Method = crate::mycelia::execution::types::Method;
    pub type FunctionWorld = crate::FunctionWorld;
}

mod test {

    use std::path::Path;
    use crate::types::*;
    use wasmtime::{component::*, Engine};
    use wasmtime_wasi::preview2::{Table, WasiCtx};

    use wasmtime_wasi::preview2::WasiView;

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

    use wasmtime::Config;

    pub fn get_config() -> Config {
        let mut cfg = Config::new();
        cfg.wasm_component_model(true);
        cfg.async_support(true);
        cfg
    }
    pub fn new_engine() -> Result<Engine, wasmtime::Error> {
        Engine::new(&get_config())
    }

    use wasmtime;

    pub fn create_function_component(
        path: &Path,
        engine: &Engine,
    ) -> Result<Component, wasmtime::Error> {
        Component::from_file(engine, path)
    }

    use wasmtime_wasi::preview2::WasiCtxBuilder;

    struct ServerWasiView {
        pub(crate) table: Table,
        pub(crate) ctx: WasiCtx,
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

    use anyhow::Context;
    use wasmtime::{
        component::{Component, Linker},
        Store,
    };
    use wasmtime_wasi::preview2::command::add_to_linker;

    use super::types::*;

    #[tokio::test]
    async fn it_invokes_a_function() -> anyhow::Result<()> {
        let engine = new_engine().context("Failed to get engine")?;
        let mut linker = Linker::new(&engine);

        let test_function_component =
            Component::from_file(&engine, "../components/function-component.wasm")
                .context("Failed to load component. Does it exist in ./components?")?;

        let host_view = ServerWasiView::new();
        let mut store = Store::new(&engine, host_view);

        let _ = add_to_linker(&mut linker).context("Failed to add command wolrd to linker")?;

        let (bindings, _instance) =
            FunctionWorld::instantiate_async(&mut store, &test_function_component, &linker)
                .await
                .context("Failed to get Function World")?;

        let should_echo = HttpRequest {
            method: Method::Get,
            headers: vec![],
            body: vec![2, 4, 6],
            uri: "foo".into(),
        };

        let result: HttpResponse = bindings
            .call_handle_request(&mut store, &should_echo)
            .await
            .context("Failed to invoke the test function")?;

        assert_eq!(result.status, 200u16);
        assert_eq!(result.body, vec![2, 4, 6]);
        Ok(())
    }
}
