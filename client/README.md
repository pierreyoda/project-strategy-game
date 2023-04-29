# Project X: Desktop game client

## Technical Overview

Uses Electron (with Forge) and Typescript. WasmPackPlugin for webpack allows this application to interact with the Rust "back-end" through the `wasm_bridge` crate, located at the root of the project.

## Development

For a better IDE experience (at least for Visual Studio Code), you should probably open this sub-project directly when reading or working on it.

Requirements:

- Latest Rust stable toolchain. To be precise, the 1.61 version should work as of writing this, but all development is done and targets the latest stable version: latest features *will* be used when needed.
- The `wasm32-unknown-unknown` target setup for the Rust toolchain
  - Run `rustup target add wasm32-unknown-unknown` with rustup
  - Or, for a custom toolchain, [follow this guide](https://rustwasm.github.io/wasm-pack/book/prerequisites/non-rustup-setups.html)
- The [wasm-pack](https://rustwasm.github.io/wasm-pack/installer/) tool installed (it's on `brew`, on NPM, has a Windows installer, a bash install script or you can just `cargo install wasm-pack`)

> Before invoking webpack in any way, you will need to run the command `yarn bootstrap:wasm`. This is needed for now since we use a Cargo Workspace to structure the Rust part of the project.
