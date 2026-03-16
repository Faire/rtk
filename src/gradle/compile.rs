/// Returns true if the task name is a compile task.
/// Matches any source set: compileKotlin, compileTestKotlin, compileIntegrationTestJava, etc.
/// Expects lowercase input from detect_task_type; output-based detection passes properly-cased names.
pub fn matches_task(task_name: &str) -> bool {
    let t = task_name.to_ascii_lowercase();
    (t.starts_with("compile") && (t.ends_with("kotlin") || t.ends_with("java")))
        || t.ends_with("classes")
}

/// COMPILE-specific filtering.
/// Full implementation in Commit 3.
pub fn filter_compile(input: &str) -> String {
    input.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_matches_compile_kotlin() {
        assert!(matches_task("compileKotlin"));
    }

    #[test]
    fn test_matches_compile_test_kotlin() {
        assert!(matches_task("compileTestKotlin"));
    }

    #[test]
    fn test_matches_compile_integration_test_kotlin() {
        assert!(matches_task("compileIntegrationTestKotlin"));
    }

    #[test]
    fn test_matches_compile_java() {
        assert!(matches_task("compileJava"));
    }

    #[test]
    fn test_matches_classes() {
        assert!(matches_task("testClasses"));
        assert!(matches_task("integrationTestClasses"));
    }

    #[test]
    fn test_matches_android_variant_compile() {
        assert!(matches_task("compileDebugKotlin"));
        assert!(matches_task("compileReleaseJava"));
    }

    #[test]
    fn test_no_match_test() {
        assert!(!matches_task("test"));
    }

    #[test]
    fn test_no_match_detekt() {
        assert!(!matches_task("detekt"));
    }
}
