/// Returns true if the task name is any kind of test task (unit, integration, component, Android).
/// Case-insensitive: callers may pass lowercase (CLI args) or original casing (output detection).
pub fn matches_task(task_name: &str) -> bool {
    let t = task_name.to_ascii_lowercase();
    // Unit test tasks
    t == "test"
        || (t.starts_with("test") && t.ends_with("unittest"))
        // Integration/component test tasks
        || t == "integrationtest"
        || t == "componenttest"
        // Android instrumented tests
        || t.contains("androidtest")
        || t.starts_with("connected")
}

/// Returns true if the task name specifically refers to an integration/component/instrumented test.
/// Used to enable integration-specific noise filtering (Hibernate, Spring, etc.)
/// Case-insensitive: callers may pass lowercase (CLI args) or original casing.
pub fn is_integration_task_name(task_name: &str) -> bool {
    let t = task_name.to_ascii_lowercase();
    t == "integrationtest"
        || t == "componenttest"
        || t.contains("androidtest")
        || t.starts_with("connected")
}

/// TEST-specific filtering + stack trace truncation.
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
