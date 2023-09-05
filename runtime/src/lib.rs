use std::path::Path;

use wasmtime::{component::*, Engine};

bindgen!("../guests/function/wit/function.wit");

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
        cfg
    }
    pub fn new_engine() -> Result<Engine, wasmtime::Error> {
        Engine::new(&get_config())
    }
}


#[cfg(test)]
mod test {
    #[test]
    fn it_builds_component() {
        
    }
}
