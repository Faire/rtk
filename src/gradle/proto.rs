/// Returns true if the task name is a proto generation task.
/// Case-insensitive via internal lowercasing.
pub fn matches_task(task_name: &str) -> bool {
    let t = task_name.to_ascii_lowercase();
    t == "buildprotos" || t == "generateprotos" || t.contains("proto")
}

/// PROTO error filtering.
/// Full implementation in Commit 6.
pub fn filter_proto(input: &str) -> String {
    input.to_string()
}
