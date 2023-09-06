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
        world: MyFunction,
        "mycelia:execution/function-interface": MyFunction
    },
});

// Define a custom type and implement the generated `Guest` trait for it which
// represents implementing all the necessary exported interfaces for this
// component.

struct MyFunction;

impl Guest for MyFunction {
    fn init() {
        print!("Hello, world!");
    }

    fn test(v: FooBar) -> String {
        format!("Hello, {}!", v.name)
    }

    fn test_resource(v: Cool) -> String {
        v.name()
    }
}
