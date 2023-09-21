//! A convenient rust crate providing a wasm http client
//! see `resource_providers::http` for the host side code

// TODO: More comprehensive documentation on the guest-side API
// for the wit-bindgen generated guest code.
// TODO: Move into new crate
mod bindgen {
    wit_bindgen::generate!({
        // the name of the world in the `*.wit` input file
        world: "command",
    });
}

use bindgen::mycelia_alpha::http::types::*;

pub type Client = bindgen::mycelia_alpha::http::interfaces::Client;
pub type HttpRequest = ClientRequest;
pub type HttpResponse = ClientResponse;
pub type HttpResult = ClientResult;

// facade for producing a new client
pub fn new_http_client() -> HttpClient {
    HttpClient {
        inner: Client::new(),
    }
}

pub struct HttpClient {
    inner: Client,
}

impl HttpClient {
    // send a http request
    // note this operation appears blocking to you but is async in the host
    // if this becomes an issue we can expand here to provide a poll method
    pub fn send(&mut self, request: &HttpRequest) -> HttpResult {
        self.inner.send(request)
    }
}
