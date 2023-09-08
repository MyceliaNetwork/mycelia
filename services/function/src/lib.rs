use wasmtime::component::*;

mod bindgen {
    use wasmtime::component::*;

    bindgen!({
      path: "../../guests/function/wit/function.wit",
      async: true
    });
}

pub mod types {
    pub type HttpRequest = crate::bindgen::mycelia::execution::types::HttpRequest;
    pub type HttpResponse = crate::bindgen::mycelia::execution::types::HttpResponse;
    pub type Method = crate::bindgen::mycelia::execution::types::Method;
    pub type FunctionWorld = crate::bindgen::FunctionWorld;
}

///! Notes
///! Engine::precompile_component is the same as precompile_module
///! instantiate_pre -> https://docs.rs/wasmtime/12.0.1/wasmtime/component/struct.Linker.html#method.instantiate_pre
///!

pub mod service {
    use std::{future::Future, pin::Pin};
    use tokio::sync::mpsc::{Sender, Receiver, channel};

    use tokio::sync::oneshot;
    use tokio::task::JoinHandle;
    use tower::{util::BoxService, BoxError, Service};
    use wasmtime::{AsContextMut, AsContext};

    use crate::types::*;

    // Note, we're not using a BoxCloneService here
    // its unclear how FunctionWorld & Instance behave when cloned.
    // Good place to dive in on that -> https://docs.rs/wasmtime/12.0.1/wasmtime/component/struct.Instance.html
    //                               -> https://docs.rs/wasmtime/latest/src/wasmtime/instance.rs.html#33

    pub type FunctionComponentService = BoxService<HttpRequest, HttpResponse, BoxError>;

    // The request to execute and a channel to respond on
    // with the response
    type InnerRequest = (HttpRequest, oneshot::Sender<InnerResponse>);
    type InnerResponse = Result<HttpResponse, BoxError>;

    type RequestSink = Sender<InnerRequest>;
    type RequestSource = Receiver<InnerRequest>;

    struct InnerService {
        request_sink: RequestSink,
        handle: JoinHandle<()>
    }


    async fn run_inner_service_loop<T: wasmtime::AsContextMut>(bindings: FunctionWorld, instance: wasmtime::component::Instance, mut store: T, mut rx : RequestSource)
        where <T as wasmtime::AsContext>::Data : Send
    {
        while let Some((request, reply)) = rx.recv().await {
            let response = bindings.call_handle_request(&mut store, &request).await.map_err(BoxError::from);
            let _ = reply.send(response);
        }
    }

    impl InnerService {
        pub fn new<T: wasmtime::AsContextMut + Send + 'static>(bindings: FunctionWorld, instance: wasmtime::component::Instance, store: T, buffer_size: usize) -> Self
            where <T as wasmtime::AsContext>::Data : Send
        {
            let (request_sink, request_source) = channel(buffer_size);

            let handle = tokio::spawn(run_inner_service_loop(bindings, instance, store, request_source));

            Self { request_sink, handle }
        }
    }

    impl Service<HttpRequest> for InnerService
    {
        type Response = HttpResponse;

        type Error = BoxError;

        type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

        fn poll_ready(&mut self, _cx: &mut std::task::Context<'_>) -> std::task::Poll<Result<(), Self::Error>> {
            // We can slow down the rate of invocation here if needed in the future
            std::task::Poll::Ready(Ok(()))
        }

        fn call(&mut self, req: HttpRequest) -> Self::Future {
            let pipe = self.request_sink.clone();
            Box::pin(async move {
                let pipe = pipe;
                let req = req;

                let (reply_tx, reply_rx) = oneshot::channel::<InnerResponse>();

                let _ = pipe.send((
                    req,
                    reply_tx
                )).await.map_err(BoxError::from)?;

                reply_rx.await?.map_err(BoxError::from)
            })
        }
    }

    // Notes
    // The underlaying calls proxied by the bindings to the FunctionWorld can be better understood by looking at
    // https://github.com/bytecodealliance/wasmtime/blob/181d005c45965b580b05df7bdaa29a8a4b4e5827/crates/wasmtime/src/func/typed.rs#L110
    // https://github.com/bytecodealliance/wasmtime/blob/main/crates/wasmtime/src/store.rs#L1615
    pub async fn new(
        bindings: FunctionWorld,
        instance: wasmtime::component::Instance,
    ) -> FunctionComponentService {
        todo!()
    }

    #[cfg(test)]
    mod test {
        use tower::Service;
        use wasmtime::{Store, component::{Component, Linker}};
        use crate::{types::{FunctionWorld, Method}, bindgen::HttpRequest};

        use super::InnerService;

        use wasmtime_wasi::preview2::command::add_to_linker;

        #[tokio::test]
        async fn it_creates_and_invokes_a_function_component_service() {
            let mut engine = crate::test::new_engine().unwrap();
            let mut linker = Linker::new(&engine);

            let mut view = crate::test::ServerWasiView::new();
            let mut store = Store::new(&engine, view);

            let _ = add_to_linker(&mut linker).unwrap();

            let test_function_component =
                Component::from_file(&engine, "../../components/function-component.wasm").unwrap();

            let (bindings, instance) =
                FunctionWorld::instantiate_async(&mut store, &test_function_component, &linker).await.unwrap();

            let mut service = InnerService::new(bindings, instance, store, 10);


            let should_echo = HttpRequest {
                method: Method::Get,
                headers: vec![],
                body: vec![2, 4, 6],
                uri: "foo".into(),
            };

            let future = service.call(should_echo);

            let result = future.await.unwrap();

            assert_eq!(result.status, 200u16);
            assert_eq!(result.body, vec![2, 4, 6]);
        }
    }
}

#[cfg(test)]
mod test {
    use super::types::*;

    use wasmtime::component::{Component, Linker};
    use wasmtime::{Config, Engine, Store};
    use wasmtime_wasi::preview2::{
        command::add_to_linker, Table, WasiCtx, WasiCtxBuilder, WasiView,
    };

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

    pub(crate) fn get_config() -> Config {
        let mut cfg = Config::new();
        cfg.wasm_component_model(true);
        cfg.async_support(true);
        cfg
    }

    pub(crate) fn new_engine() -> Result<Engine, wasmtime::Error> {
        Engine::new(&get_config())
    }

    pub(crate) struct ServerWasiView {
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

    #[tokio::test]
    async fn it_invokes_a_function() -> anyhow::Result<()> {
        let engine = new_engine()?;
        let mut linker = Linker::new(&engine);

        let test_function_component =
            Component::from_file(&engine, "../../components/function-component.wasm")?;

        let host_view = ServerWasiView::new();
        let mut store = Store::new(&engine, host_view);

        let _ = add_to_linker(&mut linker)?;

        let (bindings, _instance) =
            FunctionWorld::instantiate_async(&mut store, &test_function_component, &linker).await?;

        let should_echo = HttpRequest {
            method: Method::Get,
            headers: vec![],
            body: vec![2, 4, 6],
            uri: "foo".into(),
        };

        let result: HttpResponse = bindings
            .call_handle_request(&mut store, &should_echo)
            .await?;

        assert_eq!(result.status, 200u16);
        assert_eq!(result.body, vec![2, 4, 6]);
        Ok(())
    }

}
