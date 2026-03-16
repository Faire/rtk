/// Returns true if the task name is an integration/component test task.
pub fn matches_integration_task(task_name: &str) -> bool {
    task_name == "integrationTest" || task_name == "componentTest"
}

/// Returns true if the task name is a unit test task (exact match).
pub fn matches_test_task(task_name: &str) -> bool {
    task_name == "test"
}

/// TEST-specific filtering + stack trace truncation.
/// Full implementation in Commit 4.
pub fn filter_test(input: &str, _is_integration: bool) -> String {
    input.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_matches_test() {
        assert!(matches_test_task("test"));
    }

    #[test]
    fn test_no_match_compile() {
        assert!(!matches_test_task("compileTestKotlin"));
    }

    #[test]
    fn test_matches_integration_test() {
        assert!(matches_integration_task("integrationTest"));
    }

    #[test]
    fn test_matches_component_test() {
        assert!(matches_integration_task("componentTest"));
    }

    #[test]
    fn test_integration_no_match_test() {
        assert!(!matches_integration_task("test"));
    }
}
