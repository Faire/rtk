/// Returns true if the task name is an integration/component/instrumented test task.
pub fn matches_integration_task(task_name: &str) -> bool {
    task_name == "integrationTest"
        || task_name == "componentTest"
        // Android instrumented tests
        || task_name.contains("AndroidTest")
        || task_name.starts_with("connected")
}

/// Returns true if the task name is a unit test task.
/// Matches exact "test" and Android variant unit tests (testDebugUnitTest, etc.)
pub fn matches_test_task(task_name: &str) -> bool {
    task_name == "test" || (task_name.starts_with("test") && task_name.ends_with("UnitTest"))
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

    // Android variant tests

    #[test]
    fn test_matches_android_unit_test() {
        assert!(matches_test_task("testDebugUnitTest"));
        assert!(matches_test_task("testReleaseUnitTest"));
    }

    #[test]
    fn test_matches_connected_android_test() {
        assert!(matches_integration_task("connectedDebugAndroidTest"));
        assert!(matches_integration_task("connectedAndroidTest"));
    }
}
