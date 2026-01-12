use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn titlecase(text: &str) -> String {
    crate::titlecase(text)
}
