use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn greet(name: &str) -> String {
    format!("Hello, {}! Welcome to ZNES-WASM!", name)
}

#[wasm_bindgen]
pub fn add(left: u64, right: u64) -> u64 {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_basic_arithmetic() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
