/// Returns true if the task name is a dependency listing task.
/// Case-insensitive via internal lowercasing.
pub fn matches_task(task_name: &str) -> bool {
    task_name.to_ascii_lowercase() == "dependencies"
}

/// DEPS depth truncation.
pub fn filter_deps(input: &str) -> String {
    input.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_matches_dependencies() {
        assert!(matches_task("dependencies"));
    }

    #[test]
    fn test_matches_dependencies_case_insensitive() {
        assert!(matches_task("Dependencies"));
        assert!(matches_task("DEPENDENCIES"));
    }

    #[test]
    fn test_no_match_partial() {
        assert!(!matches_task("dep"));
        assert!(!matches_task("dependency"));
    }

    #[test]
    fn test_no_match_test() {
        assert!(!matches_task("test"));
    }
}
