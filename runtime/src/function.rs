use wasmtime::component::*;

bindgen!({
  path: "../guests/function/wit/function.wit",
  async: true
});

pub mod types {
    pub type HttpRequest = crate::function::mycelia::execution::types::HttpRequest;
    pub type HttpResponse = crate::function::mycelia::execution::types::HttpResponse;
    pub type Method = crate::function::mycelia::execution::types::Method;
}

#[cfg(test)]
mod test {
    use anyhow::Context;
    use wasmtime::{component::{Component, Linker}, Store};
    use wasmtime_wasi::preview2::command::add_to_linker;

    use crate::{engine::new_engine, ServerWasiView};
    use crate::function::FunctionWorld;
    use crate::function::types::*;

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

        let (bindings, _instance) = FunctionWorld::instantiate_async(&mut store, &test_function_component, &linker).await
            .context("Failed to get Function World")?;

        let should_echo = HttpRequest {
            method: Method::Get, headers: vec![], body: vec![2, 4, 6], uri: "foo".into() };

        let result: HttpResponse = bindings.call_handle_request(&mut store, &should_echo).await.context("Failed to invoke the test function")?;

        assert_eq!(result.status, 200u16);
        assert_eq!(result.body, vec![2, 4, 6]);
        Ok(())
    }
}
