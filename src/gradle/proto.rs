use lazy_static::lazy_static;
use regex::Regex;

lazy_static! {
    /// Proto-specific noise patterns
    static ref PROTO_NOISE: Vec<Regex> = vec![
        Regex::new(r"^> Task.*extract.*Proto").unwrap(),
    ];
    /// Proto error lines (keep these)
    static ref PROTO_ERROR: Regex = Regex::new(r"^e: ").unwrap();
}

/// Returns true if the task name is a proto generation task.
pub fn matches_task(task_name: &str) -> bool {
    task_name == "buildProtos" || task_name == "generateProtos" || task_name.contains("Proto")
}

/// Apply PROTO-specific filtering.
///
/// Keeps error lines, BUILD result, What went wrong.
/// Drops proto extraction noise.
pub fn filter_proto(input: &str) -> String {
    let mut result = Vec::new();

    for line in input.lines() {
        let trimmed = line.trim();

        // Drop proto-specific noise
        if PROTO_NOISE.iter().any(|re| re.is_match(trimmed)) {
            continue;
        }

        result.push(line.to_string());
    }

    // Trim leading/trailing blank lines
    let start = result
        .iter()
        .position(|l| !l.trim().is_empty())
        .unwrap_or(0);
    let end = result
        .iter()
        .rposition(|l| !l.trim().is_empty())
        .map(|i| i + 1)
        .unwrap_or(result.len());
    result[start..end].join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gradle::global::apply_global_filters;
    use insta::assert_snapshot;

    #[test]
    fn test_proto_failure_snapshot() {
        let input = include_str!("../../tests/fixtures/gradle/proto_failure_raw.txt");
        let globally_filtered = apply_global_filters(input);
        let output = filter_proto(&globally_filtered);
        assert_snapshot!(output);
    }

    #[test]
    fn test_proto_preserves_errors() {
        let input = include_str!("../../tests/fixtures/gradle/proto_failure_raw.txt");
        let globally_filtered = apply_global_filters(input);
        let output = filter_proto(&globally_filtered);
        assert!(output.contains("Field number 5 has already been used"));
        assert!(output.contains("is already defined"));
    }
}
