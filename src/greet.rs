/// Greet a person by their name.
///
/// # Examples
///
/// ```
/// assert_eq!(protochem::greet("World"), "Hello, World!");
/// ```
#[must_use]
pub fn greet(name: &str) -> String {
    format!("Hello, {name}!")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_greet() {
        assert_eq!(greet("Jim"), "Hello, Jim!");
    }
}
