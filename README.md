# mycelia

Open Source Application Stack &amp; PaaS

## Installation

```sh
cargo run build
```

**IMPORTANT**: `cargo build` will fail because we have to use [cargo-xtask](https://github.com/matklad/cargo-xtask/) to build the ./components/ folder before building the project.

## CLI

### Info

```sh
cargo run
```

### Start Development Server

```sh
cargo run start
```

### Stop Development Server

```sh
cargo run stop
```

### Deploy

```sh
cargo run deploy
```

NOTE: We opted for [cargo-xtask](https://github.com/matklad/cargo-xtask) because Cargo build.rs is [not supported for workspaces](https://github.com/rust-lang/cargo/issues/8732#issuecomment-950252765)

## Development Server

```sh
RUST_LOG=info cargo run --package development_server
```

### Logging

We use [env_logger](https://docs.rs/env_logger/0.10.0/env_logger/) for logging. Please see their documentation for more information on setting custom log levels, filtering, and more.

## Community & Contributing & Help

Come join our [Discord](https://discord.gg/hKMtmdMJ)

## wasi_snapshot_preview1.reactor.wasm

> search_tags: import not found, guest wasm won't build, wasi, wasi build error

Current version in repo is taken from the [v12.0.1 Release](https://github.com/bytecodealliance/wasmtime/releases/tag/v12.0.1)

More information on "what the heck" this is can be found [here](https://github.com/bytecodealliance/wasmtime/tree/main/crates/wasi-preview1-component-adapter).
In the lliekly event that's not enough context, the repo previously hosting this file can be found here: [here](https://github.com/bytecodealliance/preview2-prototyping).

> Hazel: My best explanation is.. When compiling the wasm target to `wasm32-wasi` the guest
> program expects to find a bunch wasi resources in the table(?). The [build script](guests/function/Makefile) contains an `--adapt` argument which takes the `wasi_snapshot_preview1.reactor.wasm` and injects the correct links into the guest code. I think that is what [this](https://github.com/bytecodealliance/preview2-prototyping/blob/1af2a12699ea86449d3ba1f74b5df254f16faadc/crates/wasi-preview1-component-adapter/README.md?plain=1#L47) is expressing. Here is the generated output from a [working example](https://gist.github.com/SuddenlyHazel/bf0ce95f5753c70fd72cc0937066e569)

## Resources

1. The lifetime error you're seeing means you should think about using `async_trait`. Notice the name of the lifetime `async_trait` ;)
2. Everything you wanted to know about [Resources](https://github.com/bytecodealliance/wasmtime/blob/432b5471ec4bf6d51173def284cd418be6849a49/crates/wasmtime/src/component/resources.rs#L281)

### `Questions`

1. Whats the lifetime behavior of resources??

/// Todo..

- Lock more dep versions in workspace.deps

- Build `FunctionComponentService`
  - a thing that takes invocation requests and produces responses
    wrapped in a tower service
- ^ take above into `development_server` :D
