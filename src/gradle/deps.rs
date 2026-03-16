/// Returns true if the task name is a dependency listing task.
pub fn matches_task(task_name: &str) -> bool {
    task_name == "dependencies"
}

/// DEPS depth truncation.
/// Full implementation in Commit 6.
pub fn filter_deps(input: &str) -> String {
    input.to_string()
}
