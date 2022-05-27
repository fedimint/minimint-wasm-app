#[cfg(test)]
wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

pub fn add(a: i32, b: i32) -> i32 {
    a + b
}

#[cfg(test)]
mod tests {
    use wasm_bindgen_test::*;
    use super::*;

    #[wasm_bindgen_test]
    fn test_add() {
        assert_eq!(add(1, 2), 3);
    }
}
