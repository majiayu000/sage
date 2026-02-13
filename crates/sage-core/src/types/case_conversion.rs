//! Case conversion helpers for tool argument lookup

/// Convert snake_case to camelCase
pub(super) fn to_camel_case(s: &str) -> String {
    let mut result = String::new();
    let mut capitalize_next = false;

    for c in s.chars() {
        if c == '_' {
            capitalize_next = true;
        } else if capitalize_next {
            result.push(c.to_ascii_uppercase());
            capitalize_next = false;
        } else {
            result.push(c);
        }
    }
    result
}

/// Convert camelCase to snake_case
pub(super) fn to_snake_case(s: &str) -> String {
    let mut result = String::new();

    for (i, c) in s.chars().enumerate() {
        if c.is_ascii_uppercase() {
            if i > 0 {
                result.push('_');
            }
            result.push(c.to_ascii_lowercase());
        } else {
            result.push(c);
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_camel_case() {
        assert_eq!(to_camel_case("file_path"), "filePath");
        assert_eq!(to_camel_case("working_directory"), "workingDirectory");
        assert_eq!(to_camel_case("simple"), "simple");
        assert_eq!(to_camel_case("a_b_c"), "aBC");
    }

    #[test]
    fn test_to_snake_case() {
        assert_eq!(to_snake_case("filePath"), "file_path");
        assert_eq!(to_snake_case("workingDirectory"), "working_directory");
        assert_eq!(to_snake_case("simple"), "simple");
        assert_eq!(to_snake_case("ABC"), "a_b_c");
    }
}
