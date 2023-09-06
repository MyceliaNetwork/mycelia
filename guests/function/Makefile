all: build post_process

build:
	cargo build --target wasm32-wasi --release

post_process:
	wasm-tools component new ../../target/wasm32-wasi/release/function.wasm --adapt wasi_snapshot_preview1=../../wasi_snapshot_preview1.reactor.wasm -o ../../components/function-component.wasm
