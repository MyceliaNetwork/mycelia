# mycelia
Open Source Application Stack &amp; PaaS

## Community & Contributing & Help

Come join our [Discord](https://discord.gg/hKMtmdMJ)

## wasi_snapshot_preview1.reactor.wasm

> search_tags: import not found, guest wasm won't build, wasi, wasi build error

Current version in repo is taken from the [v12.0.1 Release](https://github.com/bytecodealliance/wasmtime/releases/tag/v12.0.1)

More information on "what the heck" this is can be found [here](https://github.com/bytecodealliance/wasmtime/tree/main/crates/wasi-preview1-component-adapter).
In the lliekly event that's not enough context, the repo previously hosting this file can be found here: [here](https://github.com/bytecodealliance/preview2-prototyping).

> Hazel: My best explanation is.. When compiling the wasm target to `wasm32-wasi` the guest
> program expects to find a bunch wasi resources in the table(?). The [build script](guests/function/Makefile) contains an `--adapt` argument which takes the `wasi_snapshot_preview1.reactor.wasm` and injects the correct links into the guest code. I think that is what [this](https://github.com/bytecodealliance/preview2-prototyping/blob/1af2a12699ea86449d3ba1f74b5df254f16faadc/crates/wasi-preview1-component-adapter/README.md?plain=1#L47) is expressing. Here is the generated output from a [working example](https://gist.github.com/SuddenlyHazel/bf0ce95f5753c70fd72cc0937066e569)
