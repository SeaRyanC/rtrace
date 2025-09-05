use wasm_bindgen::prelude::*;

// Import the `console.log` function from the `console` Web API
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

// Define a macro to make console.log easier to use
macro_rules! console_log {
    ($($t:tt)*) => (log(&format_args!($($t)*).to_string()))
}

/// Returns a hello world message (WASM binding)
#[wasm_bindgen]
pub fn hello_world() -> String {
    rtrace::hello_world()
}

/// Advanced function that takes parameters and logs to console
#[wasm_bindgen]
pub fn greet_with_name_and_log(name: &str) -> String {
    let message = format!("{}, {}", rtrace::hello_world(), name);
    console_log!("Generated message: {}", message);
    message
}

/// Initialize function (called when WASM module is loaded)
#[wasm_bindgen(start)]
pub fn init() {
    console_log!("rtrace WASM module initialized!");
}
