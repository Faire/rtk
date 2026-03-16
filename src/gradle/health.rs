/// Returns true if the task name is a project health task.
/// Case-insensitive via internal lowercasing.
pub fn matches_task(task_name: &str) -> bool {
    task_name.to_ascii_lowercase().starts_with("projecthealth")
}

/// HEALTH passthrough + global filters.
/// Full implementation in Commit 6.
pub fn filter_health(input: &str) -> String {
    input.to_string()
}
