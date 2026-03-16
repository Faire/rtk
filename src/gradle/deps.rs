/// Returns true if the task name is a dependency listing task.
/// Case-insensitive via internal lowercasing.
pub fn matches_task(task_name: &str) -> bool {
    task_name.to_ascii_lowercase() == "dependencies"
}

/// DEPS depth truncation.
/// Full implementation in Commit 6.
pub fn filter_deps(input: &str) -> String {
    input.to_string()
}
