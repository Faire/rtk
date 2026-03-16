/// Returns true if the task name is any kind of test task (unit, integration, component, Android).
pub fn matches_task(task_name: &str) -> bool {
    // Unit test tasks
    task_name == "test"
        || (task_name.starts_with("test") && task_name.ends_with("UnitTest"))
        // Integration/component test tasks
        || task_name == "integrationTest"
        || task_name == "componentTest"
        // Android instrumented tests
        || task_name.contains("AndroidTest")
        || task_name.starts_with("connected")
}

/// Returns true if the task name specifically refers to an integration/component/instrumented test.
/// Used to enable integration-specific noise filtering (Hibernate, Spring, etc.)
pub fn is_integration_task_name(task_name: &str) -> bool {
    task_name == "integrationTest"
        || task_name == "componentTest"
        || task_name.contains("AndroidTest")
        || task_name.starts_with("connected")
}

/// TEST-specific filtering + stack trace truncation.
/// Full implementation in Commit 4.
pub fn filter_test(input: &str, _is_integration: bool) -> String {
    input.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- matches_task tests (unified matcher) ---

    #[test]
    fn test_matches_unit_test() {
        assert!(matches_task("test"));
    }

    #[test]
    fn test_matches_integration_test() {
        assert!(matches_task("integrationTest"));
    }

    #[test]
    fn test_matches_component_test() {
        assert!(matches_task("componentTest"));
    }

    #[test]
    fn test_no_match_compile() {
        assert!(!matches_task("compileTestKotlin"));
    }

    // Android variant tests

    #[test]
    fn test_matches_android_unit_test() {
        assert!(matches_task("testDebugUnitTest"));
        assert!(matches_task("testReleaseUnitTest"));
    }

    #[test]
    fn test_matches_connected_android_test() {
        assert!(matches_task("connectedDebugAndroidTest"));
        assert!(matches_task("connectedAndroidTest"));
    }

    // --- is_integration_task_name tests ---

    #[test]
    fn test_integration_task_name_positive() {
        assert!(is_integration_task_name("integrationTest"));
        assert!(is_integration_task_name("componentTest"));
        assert!(is_integration_task_name("connectedDebugAndroidTest"));
    }

    #[test]
    fn test_integration_task_name_negative() {
        assert!(!is_integration_task_name("test"));
        assert!(!is_integration_task_name("testDebugUnitTest"));
    }
}
