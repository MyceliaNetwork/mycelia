pub mod runtime_view {
    use wasmtime_wasi::preview2::{Table, WasiCtx, WasiCtxBuilder, WasiView};

    // In the future this will be where we provide guests
    // access to resources.
    pub struct RuntimeView {
        table: Table,
        ctx: WasiCtx,
    }

    impl RuntimeView {
        pub fn new() -> Self {
            let mut table = Table::new();
            let ctx = WasiCtxBuilder::new()
                .inherit_stdio()
                .build(&mut table)
                .unwrap();

            Self { table, ctx }
        }
    }

    impl WasiView for RuntimeView {
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
}

pub mod runtime {
    use std::path::PathBuf;

    use lazy_static::lazy_static;
    use tower::{service_fn, util::BoxCloneService, BoxError};
    use wasmtime::{
        component::{Component, Linker},
        Config, Engine, Store,
    };

    use crate::runtime_view::RuntimeView;
    use wasmtime_wasi::preview2::command::add_to_linker;

    lazy_static! {
        static ref ENGINE: wasmtime::Engine = {
            let mut cfg = Config::new();
            cfg.wasm_component_model(true);
            cfg.async_support(true);
            Engine::new(&cfg).expect("Failed to create the wasmtime engine")
        };
    }

    // Produces new stores for guest components
    //

    pub type StoreProducer = BoxCloneService<(), Store<RuntimeView>, BoxError>;

    pub fn make_store_producer() -> StoreProducer {
        let maker = |_| async move {
            let view = RuntimeView::new();
            Ok(Store::new(&ENGINE, view))
        };

        let svc = service_fn(maker);

        return BoxCloneService::new(svc);
    }

    pub fn new_linker() -> Linker<RuntimeView> {
        let mut linker = Linker::new(&ENGINE);
        let _ = add_to_linker(&mut linker).unwrap();
        linker
    }

    pub fn new_component_from_path(path: PathBuf) -> anyhow::Result<Component> {
        Component::from_file(&ENGINE, path)
    }

    pub fn new_component_from_bytes(b: &[u8]) -> anyhow::Result<Component> {
        Component::from_binary(&ENGINE, b)
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn it_works() {}
}
