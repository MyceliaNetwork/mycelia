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
        "mycelia:execution/function-interface": TestFunction
    },
});

// Todo produce exports..

// Todo check how the macro is being expanded
// We might actually be able to provide this via a lib
// Using wit-bindgen :(

// A Simple Test Function that echos what
// is passed to it. Or, returns "hello world"
struct TestFunction;

impl Guest for TestFunction {
    fn handle_request(req: HttpRequest) -> HttpResponse {
        let body = if req.body.len() > 0 {
            req.body
        } else {
            "hello world".into()
        };

        HttpResponse {
            status: 200,
            headers: vec![],
            body,
        }
    }
}
