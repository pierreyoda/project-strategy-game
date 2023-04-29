use wasm_bindgen::prelude::wasm_bindgen;

use project_x_procgen::build_hello;

#[wasm_bindgen]
pub fn procgen_build_hello() -> String {
    build_hello()
}
