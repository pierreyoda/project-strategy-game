use wasm_bindgen::prelude::wasm_bindgen;

use project_x_core::build_hello;

#[wasm_bindgen]
pub fn core_build_hello() -> String {
    build_hello()
}
