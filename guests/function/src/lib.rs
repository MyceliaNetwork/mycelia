// src/lib.rs

// Use a procedural macro to generate bindings for the world we specified in
// `host.wit`
wit_bindgen::generate!({
    // the name of the world in the `*.wit` input file
    world: "function-world",

    // For all exported worlds, interfaces, and resources, this specifies what
    // type they're corresponding to in this module. In this case the `MyHost`
    // struct defined below is going to define the exports of the `world`,
    // namely the `run` function.
    exports: {
        world: TestFunction,
    },
});

pub struct TestFunction;

impl Guest for TestFunction {
    fn handle_request(req: HttpRequest) -> HttpResponse {
        let mut client = mycelia_http::new_http_client();

        let request = mycelia_http::HttpRequest {
            method: mycelia_http::HttpMethod::Get,
            headers: vec![],
            body: vec![],
            uri: "https://google.com".to_string(),
        };

        client.send(&request);
        let body = if req.body.len() > 0 {
            req.body
        } else {
            "Hello World!".into()
        };

        HttpResponse {
            status: 200,
            headers: vec![],
            body,
        }
    }
}
