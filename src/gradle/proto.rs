/// Returns true if the task name is a proto generation task.
/// Case-insensitive via internal lowercasing.
pub fn matches_task(task_name: &str) -> bool {
    let t = task_name.to_ascii_lowercase();
    t == "buildprotos" || t == "generateprotos" || t.contains("proto")
}

/// PROTO error filtering.
pub fn filter_proto(input: &str) -> String {
    input.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_matches_build_protos() {
        assert!(matches_task("buildProtos"));
    }

    #[test]
    fn test_matches_generate_protos() {
        assert!(matches_task("generateProtos"));
    }

    #[test]
    fn test_matches_contains_proto() {
        assert!(matches_task("extractProto"));
        assert!(matches_task("generateTestProto"));
    }

    #[test]
    fn test_matches_proto_case_insensitive() {
        assert!(matches_task("BuildProtos"));
        assert!(matches_task("GenerateProtos"));
    }

    #[test]
    fn test_no_match_test() {
        assert!(!matches_task("test"));
    }

    #[test]
    fn test_no_match_compile() {
        assert!(!matches_task("compileKotlin"));
    }
}
