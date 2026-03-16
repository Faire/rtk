/// Returns true if the task name is a project health task.
pub fn matches_task(task_name: &str) -> bool {
    task_name.starts_with("projectHealth")
}

/// HEALTH passthrough + global filters.
/// Full implementation in Commit 6.
pub fn filter_health(input: &str) -> String {
    input.to_string()
}
