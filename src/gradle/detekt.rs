/// Returns true if the task name is a detekt task.
/// Case-insensitive via internal lowercasing.
pub fn matches_task(task_name: &str) -> bool {
    task_name.to_ascii_lowercase().starts_with("detekt")
}

/// DETEKT violation grouping.
pub fn filter_detekt(input: &str) -> String {
    input.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_matches_detekt() {
        assert!(matches_task("detekt"));
    }

    #[test]
    fn test_matches_detekt_main() {
        assert!(matches_task("detektMain"));
    }

    #[test]
    fn test_matches_detekt_test() {
        assert!(matches_task("detektTest"));
    }

    #[test]
    fn test_matches_detekt_case_insensitive() {
        assert!(matches_task("Detekt"));
        assert!(matches_task("DetektMain"));
    }

    #[test]
    fn test_no_match_test() {
        assert!(!matches_task("test"));
    }

    #[test]
    fn test_no_match_compile() {
        assert!(!matches_task("compileKotlin"));
    }
}
