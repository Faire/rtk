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
    IntegrationTest,
    Detekt,
    Health,
    Proto,
    Deps,
    Generic,
}

/// Detect the task type from gradle arguments.
///
/// Scans all args for task name patterns. If multiple tasks are present
/// (batch run), returns `Generic` — the batch filter handles per-task routing.
pub fn detect_task_type(args: &[String]) -> TaskType {
    let mut detected: Vec<TaskType> = Vec::new();

    for arg in args {
        // Skip flags (start with -)
        if arg.starts_with('-') {
            continue;
        }

        // Extract the task name (last segment after :)
        let task_name = arg.rsplit(':').next().unwrap_or(arg);

        let task_type = if task_name == "dependencies" {
            Some(TaskType::Deps)
        } else if task_name == "integrationTest" || task_name == "componentTest" {
            Some(TaskType::IntegrationTest)
        } else if task_name == "test" {
            Some(TaskType::Test)
        } else if task_name.starts_with("detekt") {
            Some(TaskType::Detekt)
        } else if task_name.starts_with("projectHealth") {
            Some(TaskType::Health)
        } else if task_name == "compileKotlin"
            || task_name == "compileTestKotlin"
            || task_name == "compileJava"
            || task_name == "compileTestJava"
            || task_name.ends_with("Classes")
        {
            Some(TaskType::Compile)
        } else if task_name == "buildProtos"
            || task_name == "generateProtos"
            || task_name.contains("Proto")
        {
            Some(TaskType::Proto)
        } else {
            None
        };

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

    let task_type = detect_task_type(args);
    let filtered = filter_gradle_output(&raw, &task_type);

    let exit_code = output
        .status
        .code()
        .unwrap_or(if output.status.success() { 0 } else { 1 });

    if let Some(hint) = crate::tee::tee_and_hint(&raw, "gradle", exit_code) {
        println!("{}\n{}", filtered, hint);
    } else {
        println!("{}", filtered);
    }

    // Include stderr if it has content not already in stdout
    if !stderr.trim().is_empty() && !stdout.contains(stderr.trim()) {
        eprintln!("{}", stderr.trim());
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
pub fn filter_gradle_output(raw: &str, task_type: &TaskType) -> String {
    // For now, pass through with just global filters (will be enhanced in later commits)
    let filtered = global::apply_global_filters(raw);

    match task_type {
        TaskType::Compile => compile::filter_compile(&filtered),
        TaskType::Test | TaskType::IntegrationTest => {
            test_filter::filter_test(&filtered, task_type == &TaskType::IntegrationTest)
        }
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
        let args = vec![":backend:backend-payments:compileKotlin".to_string()];
        assert_eq!(detect_task_type(&args), TaskType::Compile);
    }

    #[test]
    fn test_detect_compile_test_kotlin() {
        let args = vec![":backend:backend-payments:compileTestKotlin".to_string()];
        assert_eq!(detect_task_type(&args), TaskType::Compile);
    }

    #[test]
    fn test_detect_compile_classes() {
        let args = vec![":backend:backend-payments:testClasses".to_string()];
        assert_eq!(detect_task_type(&args), TaskType::Compile);
    }

    #[test]
    fn test_detect_test() {
        let args = vec![":backend:backend-payments:test".to_string()];
        assert_eq!(detect_task_type(&args), TaskType::Test);
    }

    #[test]
    fn test_detect_integration_test() {
        let args = vec![":backend:backend-payments:integrationTest".to_string()];
        assert_eq!(detect_task_type(&args), TaskType::IntegrationTest);
    }

    #[test]
    fn test_detect_component_test() {
        let args = vec![":backend:backend-payments:componentTest".to_string()];
        assert_eq!(detect_task_type(&args), TaskType::IntegrationTest);
    }

    #[test]
    fn test_detect_detekt() {
        let args = vec![":backend:backend-payments:detekt".to_string()];
        assert_eq!(detect_task_type(&args), TaskType::Detekt);
    }

    #[test]
    fn test_detect_detekt_main() {
        let args = vec![":backend:backend-payments:detektMain".to_string()];
        assert_eq!(detect_task_type(&args), TaskType::Detekt);
    }

    #[test]
    fn test_detect_health() {
        let args = vec![":backend:backend-payments:projectHealth".to_string()];
        assert_eq!(detect_task_type(&args), TaskType::Health);
    }

    #[test]
    fn test_detect_proto_build() {
        let args = vec![":backend:backend-payments-api:buildProtos".to_string()];
        assert_eq!(detect_task_type(&args), TaskType::Proto);
    }

    #[test]
    fn test_detect_proto_generate() {
        let args = vec![":backend:backend-payments-api:generateProtos".to_string()];
        assert_eq!(detect_task_type(&args), TaskType::Proto);
    }

    #[test]
    fn test_detect_deps() {
        let args = vec![":backend:backend-payments:dependencies".to_string()];
        assert_eq!(detect_task_type(&args), TaskType::Deps);
    }

    #[test]
    fn test_detect_generic_unknown_task() {
        let args = vec![":backend:backend-payments:assemble".to_string()];
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
            ":backend:backend-payments:test".to_string(),
            "--info".to_string(),
        ];
        assert_eq!(detect_task_type(&args), TaskType::Test);
    }

    #[test]
    fn test_detect_multiple_same_type_returns_single() {
        let args = vec![
            ":backend:backend-payments:test".to_string(),
            ":backend:backend-orders:test".to_string(),
        ];
        assert_eq!(detect_task_type(&args), TaskType::Test);
    }

    #[test]
    fn test_detect_multiple_different_types_returns_generic() {
        let args = vec![
            ":backend:backend-payments:test".to_string(),
            ":backend:backend-payments:detekt".to_string(),
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
}
