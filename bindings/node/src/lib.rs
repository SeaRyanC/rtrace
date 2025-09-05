use napi_derive::napi;

/// Returns a hello world message (Node.js binding)
#[napi]
pub fn hello_world() -> String {
    rtrace::hello_world()
}

/// Advanced function that takes parameters (demonstration)
#[napi]
pub fn greet_with_name(name: String) -> String {
    format!("{}, {}", rtrace::hello_world(), name)
}
