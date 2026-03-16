/// Returns true if the task name is a detekt task.
pub fn matches_task(task_name: &str) -> bool {
    task_name.starts_with("detekt")
}

/// DETEKT violation grouping.
/// Full implementation in Commit 5.
pub fn filter_detekt(input: &str) -> String {
    input.to_string()
}
