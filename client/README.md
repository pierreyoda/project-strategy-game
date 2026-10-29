# Project X: Desktop game client

## Technical Overview

Uses Electron (with Forge and its Vite plugin) and Typescript, with a [Svelte](https://svelte.dev/) and [Tailwind CSS](https://tailwindcss.com/) renderer. This application interacts with the Rust "back-end" through the `wasm_bridge` crate, located at the root of the project: the `vite.wasm-pack.ts` plugin builds it with wasm-pack, and Vite loads the result through its [WebAssembly support](https://vite.dev/guide/features#webassembly).

The main process is built as an ES module, since Vite's WebAssembly ES module integration requires top-level await.

## Development

For a better IDE experience (at least for Visual Studio Code), you should probably open this sub-project directly when reading or working on it.

Requirements:

- Latest Rust stable toolchain. To be precise, the 1.61 version should work as of writing this, but all development is done and targets the latest stable version: latest features *will* be used when needed.
- The `wasm32-unknown-unknown` target setup for the Rust toolchain
  - Run `rustup target add wasm32-unknown-unknown` with rustup
  - Or, for a custom toolchain, [follow this guide](https://rustwasm.github.io/wasm-pack/book/prerequisites/non-rustup-setups.html)
- The [wasm-pack](https://rustwasm.github.io/wasm-pack/installer/) tool installed (it's on `brew`, on NPM, has a Windows installer, a bash install script or you can just `cargo install wasm-pack`)

The WASM package is built automatically by `yarn start` (debug build, rebuilt when the Rust crates change) and `yarn package` / `yarn make` (release build).

> Vite does not type-check: run `yarn typecheck` for that. It needs the WASM package's typings, so if no build was run yet, run `yarn bootstrap:wasm` first.
