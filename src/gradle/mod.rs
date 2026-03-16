pub mod batch;
pub mod compile;
pub mod deps;
pub mod detekt;
pub mod global;
pub mod health;
pub mod paths;
pub mod proto;
pub mod test_filter;

use crate::tracking;
use anyhow::{Context, Result};
use std::process::Command;

#[derive(Debug, Clone, PartialEq)]
pub enum TaskType {
    Compile,
    Test,
    Detekt,
    Health,
    Proto,
    Deps,
    Generic,
}

/// Registry of task type matchers, checked in priority order.
const TASK_TYPE_REGISTRY: &[(fn(&str) -> bool, TaskType)] = &[
    (deps::matches_task, TaskType::Deps),
    (test_filter::matches_task, TaskType::Test),
    (detekt::matches_task, TaskType::Detekt),
    (health::matches_task, TaskType::Health),
    (compile::matches_task, TaskType::Compile),
    (proto::matches_task, TaskType::Proto),
];

/// Detect the task type from gradle arguments.
///
/// Scans all args for task name patterns using per-module matchers.
/// If multiple distinct task types are present (batch run), returns `Generic`
/// — the batch filter handles per-task routing.
pub fn detect_task_type(args: &[String]) -> TaskType {
    let mut detected: Vec<TaskType> = Vec::new();

    for arg in args {
        // Skip flags (start with -)
        if arg.starts_with('-') {
            continue;
        }

        // Extract the task name (last segment after :), lowercased for
        // case-insensitive CLI matching (Gradle accepts any casing on CLI).
        let task_name = match arg.rfind(':') {
            Some(pos) => arg[pos + 1..].to_ascii_lowercase(),
            None => arg.to_ascii_lowercase(),
        };

        // Walk registry in priority order, first match wins
        let task_type = TASK_TYPE_REGISTRY
            .iter()
            .find(|(matcher, _)| matcher(&task_name))
            .map(|(_, tt)| tt.clone());

        if let Some(tt) = task_type {
            if !detected.iter().any(|d| d == &tt) {
                detected.push(tt);
            }
        }
    }

    match detected.len() {
        0 => TaskType::Generic,
        1 => detected.into_iter().next().unwrap_or(TaskType::Generic),
        _ => TaskType::Generic, // Multiple distinct task types → batch handles routing
    }
}

/// Refine a Generic task type by scanning raw output for `> Task :...:taskName` lines.
///
/// Handles meta-tasks (like `check`, `build`, `lint`) that delegate to specific tasks.
/// If output reveals a single task type, returns that type; otherwise keeps Generic.
pub fn detect_task_type_from_output(raw: &str) -> TaskType {
    use lazy_static::lazy_static;
    use regex::Regex;

    lazy_static! {
        static ref TASK_LINE: Regex = Regex::new(r"^> Task :(?:[^:]+:)*([^\s]+)").unwrap();
    }

    let mut detected: Vec<TaskType> = Vec::new();

    for line in raw.lines() {
        if let Some(caps) = TASK_LINE.captures(line.trim()) {
            let task_name = caps.get(1).map_or("", |m| m.as_str());

            let task_type = TASK_TYPE_REGISTRY
                .iter()
                .find(|(matcher, _)| matcher(task_name))
                .map(|(_, tt)| tt.clone());

            if let Some(tt) = task_type {
                if !detected.iter().any(|d| d == &tt) {
                    detected.push(tt);
                }
            }
        }
    }

    match detected.len() {
        1 => detected.into_iter().next().unwrap_or(TaskType::Generic),
        _ => TaskType::Generic, // 0 or multiple types → keep Generic
    }
}

/// Returns true if any of the given args refer to an integration/component/instrumented test task.
pub fn is_integration_test(args: &[String]) -> bool {
    args.iter().any(|arg| {
        let task_name = match arg.rfind(':') {
            Some(pos) => arg[pos + 1..].to_ascii_lowercase(),
            None => arg.to_ascii_lowercase(),
        };
        test_filter::is_integration_task_name(&task_name)
    })
}

/// Find the gradle executable: prefer ./gradlew walking up parent dirs, fall back to gradle on PATH.
fn find_gradle_executable() -> String {
    let candidates = [
        "./gradlew",
        "../gradlew",
        "../../gradlew",
        "../../../gradlew",
    ];
    for candidate in &candidates {
        if std::path::Path::new(candidate).exists() {
            return candidate.to_string();
        }
    }
    "gradle".to_string()
}

/// Inject `--console plain` if not already present in args.
fn ensure_console_plain(args: &[String]) -> Vec<String> {
    let has_console = args
        .iter()
        .any(|a| a == "--console" || a.starts_with("--console="));
    if has_console {
        args.to_vec()
    } else {
        let mut result = args.to_vec();
        result.push("--console".to_string());
        result.push("plain".to_string());
        result
    }
}

pub fn run(args: &[String], verbose: u8) -> Result<()> {
    let timer = tracking::TimedExecution::start();

    let gradle = find_gradle_executable();
    let full_args = ensure_console_plain(args);

    if verbose > 0 {
        eprintln!("Running: {} {}", gradle, full_args.join(" "));
    }

    let mut cmd = Command::new(&gradle);
    for arg in &full_args {
        cmd.arg(arg);
    }

    let output = cmd
        .output()
        .context("Failed to run gradle. Is gradle or ./gradlew available?")?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let raw = format!("{}\n{}", stdout, stderr);

    let mut task_type = detect_task_type(args);
    // Fallback: if args didn't reveal a task type, scan output for executed tasks
    if task_type == TaskType::Generic {
        task_type = detect_task_type_from_output(&raw);
    }
    let is_integration = is_integration_test(args);
    let filtered = filter_gradle_output(&raw, &task_type, is_integration);

    let exit_code = output
        .status
        .code()
        .unwrap_or(if output.status.success() { 0 } else { 1 });

    if let Some(hint) = crate::tee::tee_and_hint(&raw, "gradle", exit_code) {
        println!("{}\n{}", filtered, hint);
    } else {
        println!("{}", filtered);
    }

    // Include stderr if it has content not already in stdout, filtered for noise
    if !stderr.trim().is_empty() && !stdout.contains(stderr.trim()) {
        let filtered_stderr = global::apply_global_filters(&stderr);
        if !filtered_stderr.trim().is_empty() {
            eprintln!("{}", filtered_stderr.trim());
        }
    }

    timer.track(
        &format!("{} {}", gradle, args.join(" ")),
        &format!("rtk gradle {}", args.join(" ")),
        &raw,
        &filtered,
    );

    if !output.status.success() {
        std::process::exit(exit_code);
    }

    Ok(())
}

/// Apply task-type-specific filtering to gradle output.
pub fn filter_gradle_output(raw: &str, task_type: &TaskType, is_integration: bool) -> String {
    // For now, pass through with just global filters (will be enhanced in later commits)
    let filtered = global::apply_global_filters(raw);

    match task_type {
        TaskType::Compile => compile::filter_compile(&filtered),
        TaskType::Test => test_filter::filter_test(&filtered, is_integration),
        TaskType::Detekt => detekt::filter_detekt(&filtered),
        TaskType::Health => health::filter_health(&filtered),
        TaskType::Proto => proto::filter_proto(&filtered),
        TaskType::Deps => deps::filter_deps(&filtered),
        TaskType::Generic => filtered,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- detect_task_type tests ---

    #[test]
    fn test_detect_compile_kotlin() {
        let args = vec![":app:billing:compileKotlin".to_string()];
        assert_eq!(detect_task_type(&args), TaskType::Compile);
    }

    #[test]
    fn test_detect_compile_test_kotlin() {
        let args = vec![":app:billing:compileTestKotlin".to_string()];
        assert_eq!(detect_task_type(&args), TaskType::Compile);
    }

    #[test]
    fn test_detect_compile_integration_test_kotlin() {
        let args = vec![":app:billing:compileIntegrationTestKotlin".to_string()];
        assert_eq!(detect_task_type(&args), TaskType::Compile);
    }

    #[test]
    fn test_detect_compile_classes() {
        let args = vec![":app:billing:testClasses".to_string()];
        assert_eq!(detect_task_type(&args), TaskType::Compile);
    }

    #[test]
    fn test_detect_test() {
        let args = vec![":app:billing:test".to_string()];
        assert_eq!(detect_task_type(&args), TaskType::Test);
    }

    #[test]
    fn test_detect_integration_test() {
        let args = vec![":app:billing:integrationTest".to_string()];
        assert_eq!(detect_task_type(&args), TaskType::Test);
    }

    #[test]
    fn test_detect_component_test() {
        let args = vec![":app:billing:componentTest".to_string()];
        assert_eq!(detect_task_type(&args), TaskType::Test);
    }

    #[test]
    fn test_detect_detekt() {
        let args = vec![":app:billing:detekt".to_string()];
        assert_eq!(detect_task_type(&args), TaskType::Detekt);
    }

    #[test]
    fn test_detect_detekt_main() {
        let args = vec![":app:billing:detektMain".to_string()];
        assert_eq!(detect_task_type(&args), TaskType::Detekt);
    }

    #[test]
    fn test_detect_health() {
        let args = vec![":app:billing:projectHealth".to_string()];
        assert_eq!(detect_task_type(&args), TaskType::Health);
    }

    #[test]
    fn test_detect_proto_build() {
        let args = vec![":app:billing-api:buildProtos".to_string()];
        assert_eq!(detect_task_type(&args), TaskType::Proto);
    }

    #[test]
    fn test_detect_proto_generate() {
        let args = vec![":app:billing-api:generateProtos".to_string()];
        assert_eq!(detect_task_type(&args), TaskType::Proto);
    }

    #[test]
    fn test_detect_deps() {
        let args = vec![":app:billing:dependencies".to_string()];
        assert_eq!(detect_task_type(&args), TaskType::Deps);
    }

    #[test]
    fn test_detect_generic_unknown_task() {
        let args = vec![":app:billing:assemble".to_string()];
        assert_eq!(detect_task_type(&args), TaskType::Generic);
    }

    #[test]
    fn test_detect_generic_no_task() {
        let args: Vec<String> = vec!["--help".to_string()];
        assert_eq!(detect_task_type(&args), TaskType::Generic);
    }

    #[test]
    fn test_detect_skips_flags() {
        let args = vec![
            "--continue".to_string(),
            ":app:billing:test".to_string(),
            "--info".to_string(),
        ];
        assert_eq!(detect_task_type(&args), TaskType::Test);
    }

    #[test]
    fn test_detect_multiple_same_type_returns_single() {
        let args = vec![
            ":app:billing:test".to_string(),
            ":app:orders:test".to_string(),
        ];
        assert_eq!(detect_task_type(&args), TaskType::Test);
    }

    #[test]
    fn test_detect_multiple_different_types_returns_generic() {
        let args = vec![
            ":app:billing:test".to_string(),
            ":app:billing:detekt".to_string(),
        ];
        assert_eq!(detect_task_type(&args), TaskType::Generic);
    }

    // --- ensure_console_plain tests ---

    #[test]
    fn test_console_plain_injected_when_missing() {
        let args = vec![":app:test".to_string()];
        let result = ensure_console_plain(&args);
        assert_eq!(result, vec![":app:test", "--console", "plain"]);
    }

    #[test]
    fn test_console_plain_not_duplicated() {
        let args = vec![
            "--console".to_string(),
            "rich".to_string(),
            ":app:test".to_string(),
        ];
        let result = ensure_console_plain(&args);
        assert_eq!(result, args);
    }

    #[test]
    fn test_console_plain_already_plain_not_duplicated() {
        let args = vec![
            "--console".to_string(),
            "plain".to_string(),
            ":app:test".to_string(),
        ];
        let result = ensure_console_plain(&args);
        assert_eq!(result, args);
    }

    #[test]
    fn test_console_equals_form_not_duplicated() {
        let args = vec!["--console=plain".to_string(), ":app:test".to_string()];
        let result = ensure_console_plain(&args);
        assert_eq!(result, args);
    }

    // --- detect_task_type_from_output tests ---

    #[test]
    fn test_output_detection_finds_test() {
        let output = "> Task :app:billing:processResources UP-TO-DATE\n> Task :app:billing:test\n> Task :app:billing:test FAILED";
        assert_eq!(detect_task_type_from_output(output), TaskType::Test);
    }

    #[test]
    fn test_output_detection_finds_detekt() {
        let output = "> Task :app:billing:detektMain\n> Task :app:billing:detektTest";
        assert_eq!(detect_task_type_from_output(output), TaskType::Detekt);
    }

    #[test]
    fn test_output_detection_multiple_types_returns_generic() {
        let output = "> Task :app:billing:test\n> Task :app:billing:detektMain";
        assert_eq!(detect_task_type_from_output(output), TaskType::Generic);
    }

    #[test]
    fn test_output_detection_no_tasks_returns_generic() {
        let output = "BUILD SUCCESSFUL in 5s";
        assert_eq!(detect_task_type_from_output(output), TaskType::Generic);
    }

    #[test]
    fn test_output_detection_ignores_compile_when_test_present() {
        // Compile tasks are common prerequisites — if test tasks also appear,
        // both types are detected → Generic (batch handles routing)
        let output = "> Task :app:compileKotlin\n> Task :app:test";
        // Two distinct types → Generic
        assert_eq!(detect_task_type_from_output(output), TaskType::Generic);
    }

    // --- is_integration_test tests ---

    #[test]
    fn test_is_integration_test_positive() {
        let args = vec![":app:billing:integrationTest".to_string()];
        assert!(is_integration_test(&args));
    }

    #[test]
    fn test_is_integration_test_negative() {
        let args = vec![":app:billing:test".to_string()];
        assert!(!is_integration_test(&args));
    }

    #[test]
    fn test_is_integration_test_mixed_args() {
        let args = vec![
            "--continue".to_string(),
            ":app:billing:integrationTest".to_string(),
            "--info".to_string(),
        ];
        assert!(is_integration_test(&args));
    }

    // --- case-insensitive matching tests ---

    #[test]
    fn test_detect_case_insensitive_test() {
        let args = vec![":app:billing:Test".to_string()];
        assert_eq!(detect_task_type(&args), TaskType::Test);
    }

    #[test]
    fn test_detect_case_insensitive_compile_kotlin() {
        let args = vec![":app:billing:CompileKotlin".to_string()];
        assert_eq!(detect_task_type(&args), TaskType::Compile);
    }

    #[test]
    fn test_detect_case_insensitive_detekt() {
        let args = vec![":app:billing:Detekt".to_string()];
        assert_eq!(detect_task_type(&args), TaskType::Detekt);
    }

    #[test]
    fn test_detect_case_insensitive_project_health() {
        let args = vec![":app:billing:ProjectHealth".to_string()];
        assert_eq!(detect_task_type(&args), TaskType::Health);
    }

    #[test]
    fn test_detect_case_insensitive_dependencies() {
        let args = vec![":app:billing:Dependencies".to_string()];
        assert_eq!(detect_task_type(&args), TaskType::Deps);
    }

    #[test]
    fn test_detect_case_insensitive_build_protos() {
        let args = vec![":app:billing:BuildProtos".to_string()];
        assert_eq!(detect_task_type(&args), TaskType::Proto);
    }

    #[test]
    fn test_is_integration_test_case_insensitive() {
        let args = vec![":app:billing:IntegrationTest".to_string()];
        assert!(is_integration_test(&args));
    }
}
