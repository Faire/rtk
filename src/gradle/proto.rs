/// Returns true if the task name is a proto generation task.
pub fn matches_task(task_name: &str) -> bool {
    task_name == "buildProtos" || task_name == "generateProtos" || task_name.contains("Proto")
}

/// PROTO error filtering.
/// Full implementation in Commit 6.
pub fn filter_proto(input: &str) -> String {
    input.to_string()
}
