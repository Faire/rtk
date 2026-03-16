/// Returns true if the task name is a project health task.
/// Case-insensitive via internal lowercasing.
pub fn matches_task(task_name: &str) -> bool {
    task_name.to_ascii_lowercase().starts_with("projecthealth")
}

/// HEALTH passthrough + global filters.
pub fn filter_health(input: &str) -> String {
    input.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_matches_project_health() {
        assert!(matches_task("projectHealth"));
    }

    #[test]
    fn test_matches_project_health_case_insensitive() {
        assert!(matches_task("ProjectHealth"));
        assert!(matches_task("PROJECTHEALTH"));
    }

    #[test]
    fn test_no_match_health_alone() {
        assert!(!matches_task("health"));
    }

    #[test]
    fn test_no_match_test() {
        assert!(!matches_task("test"));
    }
}
