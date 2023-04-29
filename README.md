A very new and quite ambitious video game project, developed with an open-source philosophy.

Basically a grand strategy 4X game with tactical-level war-gaming gameplay. Much more details on the [website](TODO:link).

> Current project codename is susceptible to change at some point in the future.

## Showcase website and development blog

TODO: in construction

## Main components

Besides the multi-platform desktop `client` (using Electron, in part for an advanced GUI system leveraging modern React), here are the different Rust crates the root Cargo Workspace contains:

- `core`: everything concerning the game and its underlying engine, apart from the more or less narrowly scoped crates that follow.
- `procgen`: procedural generation of everything generated for the start of a new game playthrough, like realistic terrain, factions placement, and so much more long term.
- `wasm_bridge`: bindings and wrappers for usage from Node.js/Typescript in the `client`. Uses [wasm-bindgen](https://rustwasm.github.io/docs/wasm-bindgen).
