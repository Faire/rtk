/// TEST-specific filtering + stack trace truncation.
/// Full implementation in Commit 4.
pub fn filter_test(input: &str, _is_integration: bool) -> String {
    input.to_string()
}
