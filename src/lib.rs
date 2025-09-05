/// Core library for rtrace
///
/// This library provides a simple hello world function that can be used
/// from Rust code, CLI applications, Node.js, and WebAssembly.
/// Returns a greeting message
///
/// # Examples
///
/// ```
/// use rtrace::hello_world;
///
/// let message = hello_world();
/// assert_eq!(message, "hello world");
/// ```
pub fn hello_world() -> String {
    "hello world".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hello_world() {
        assert_eq!(hello_world(), "hello world");
    }
}
