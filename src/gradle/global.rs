use crate::utils::strip_ansi;
use lazy_static::lazy_static;
use regex::{Regex, RegexSet};

lazy_static! {
    /// Built-in noise patterns compiled into a single RegexSet for single-pass matching.
    static ref NOISE_SET: RegexSet = RegexSet::new([
        // Task status lines (UP-TO-DATE, SKIPPED, NO-SOURCE, FROM-CACHE)
        r"^> Task \S+ (UP-TO-DATE|SKIPPED|NO-SOURCE|FROM-CACHE)$",
        // Bare executed task lines (no suffix) — replaced by ✓ summary
        r"^> Task \S+\s*$",
        // Configure lines
        r"^> Configure project ",
        // Daemon startup
        r"^(Starting a? ?Gradle Daemon|Gradle Daemon started|Daemon initialized|Worker lease)",
        // JVM warnings
        r"^(OpenJDK 64-Bit Server VM warning:|Initialized native services|Initialized jansi)",
        // Incubating (including Problems report)
        r"\[Incubating\]|Configuration on demand is an incubating feature|Parallel Configuration Cache is an incubating feature",
        // Config cache
        r"^(Reusing configuration cache|Calculating task graph|Configuration cache entry|Storing configuration cache|Loading configuration cache)",
        // Deprecation
        r"^(Deprecated Gradle features were used|For more on this, please refer to|You can use '--warning-mode all')",
        // Downloads + progress bars
        r"^(Download |Downloading )",
        r"^\s*\[[\s<=\->]+\]\s+\d+%",
        // Build scan + develocity URLs (both private develocity.* and public scans.gradle.com)
        r"^(Publishing build scan|https://(develocity\.|scans\.gradle\.com)|Upload .* build scan|Waiting for build scan)",
        // VFS (all VFS> lines and Virtual file system lines)
        r"^(VFS>|Virtual file system )",
        // Evaluation
        r"^(Evaluating root project|All projects evaluated|Settings evaluated)",
        // Classpath
        r"^(Classpath snapshot |Snapshotting classpath)",
        // Kotlin daemon
        r"^(Kotlin compile daemon|Connected to the daemon)",
        // Reflection warnings
        r"(?i)^WARNING:.*illegal reflective|(?i)^WARNING:.*reflect",
        // File system events
        r"^Received \d+ file system events",
        // Javac/kapt notes (not actionable)
        r"^Note: ",
    ]).unwrap();
}

/// Apply global noise filters to gradle output.
///
/// Drops noise lines, removes `* Try:` blocks, and trims blank lines.
pub fn apply_global_filters(input: &str) -> String {
    let config = load_extra_patterns();
    apply_global_filters_with_extras(input, &config)
}

/// Load extra drop patterns from config.toml [gradle] section.
fn load_extra_patterns() -> Option<RegexSet> {
    match crate::config::Config::load() {
        Ok(config) => compile_extra_patterns(&config.gradle.extra_drop_patterns),
        Err(_) => None,
    }
}

/// Compile user-supplied regex patterns into a RegexSet, skipping invalid ones with stderr warning.
pub fn compile_extra_patterns(patterns: &[String]) -> Option<RegexSet> {
    let valid: Vec<&str> = patterns
        .iter()
        .filter(|p| match Regex::new(p) {
            Ok(_) => true,
            Err(e) => {
                eprintln!("rtk: invalid extra_drop_pattern '{}': {}", p, e);
                false
            }
        })
        .map(|s| s.as_str())
        .collect();
    if valid.is_empty() {
        None
    } else {
        RegexSet::new(&valid).ok()
    }
}

/// Core filter logic, testable with explicit extra patterns.
pub fn apply_global_filters_with_extras(input: &str, extra_patterns: &Option<RegexSet>) -> String {
    let mut result = Vec::new();
    let mut in_try_block = false;

    for line in input.lines() {
        let trimmed = line.trim();
        // Strip ANSI escape codes for pattern matching (but keep original line for output)
        let clean = strip_ansi(trimmed);
        let clean_trimmed = clean.trim();

        // Try block removal: "* Try:" through next "* " header or end of block
        // Must be checked before blank line handling so blank lines inside Try blocks are consumed
        if clean_trimmed.starts_with("* Try:") {
            in_try_block = true;
            continue;
        }
        if in_try_block {
            if clean_trimmed.is_empty() {
                // Blank lines inside Try block — consume
                continue;
            } else if clean_trimmed.starts_with("* ") {
                // Next * header ends the Try block
                in_try_block = false;
                // Fall through to process this line normally
            } else if clean_trimmed.starts_with("> ")
                || clean_trimmed.starts_with("Get more help at")
            {
                // Indented content within Try block
                continue;
            } else {
                // Non-Try-block content — end the block
                in_try_block = false;
                // Fall through to process this line normally
            }
        }

        // Skip empty lines (blank line collapsing)
        if clean_trimmed.is_empty() {
            // Only add blank if last line wasn't blank
            if result
                .last()
                .map_or(true, |l: &String| !l.trim().is_empty())
            {
                result.push(String::new());
            }
            continue;
        }

        // Check against built-in noise patterns (single-pass RegexSet match)
        if NOISE_SET.is_match(clean_trimmed) {
            continue;
        }

        // Drop lines that are only ANSI escape codes (no visible content after stripping)
        if clean_trimmed.is_empty() && !trimmed.is_empty() {
            continue;
        }

        // Check against extra user-supplied patterns
        if extra_patterns
            .as_ref()
            .map_or(false, |set| set.is_match(clean_trimmed))
        {
            continue;
        }

        // Lines always kept (BUILD SUCCESSFUL/FAILED, FAILURE header, What went wrong)
        // These pass through naturally since they don't match noise patterns

        result.push(line.to_string());
    }

    // Trim leading/trailing blank lines
    while result.first().map_or(false, |l| l.trim().is_empty()) {
        result.remove(0);
    }
    while result.last().map_or(false, |l| l.trim().is_empty()) {
        result.pop();
    }

    result.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use insta::assert_snapshot;

    fn count_tokens(text: &str) -> usize {
        text.split_whitespace().count()
    }

    #[test]
    fn test_compile_success_snapshot() {
        let input = include_str!("../../tests/fixtures/gradle/compile_success_raw.txt");
        let output = apply_global_filters(input);
        assert_snapshot!(output);
    }

    #[test]
    fn test_compile_success_token_savings() {
        let input = include_str!("../../tests/fixtures/gradle/compile_success_raw.txt");
        let output = apply_global_filters(input);
        let input_tokens = count_tokens(input);
        let output_tokens = count_tokens(&output);
        let savings = 100.0 - (output_tokens as f64 / input_tokens as f64 * 100.0);
        assert!(
            savings >= 90.0,
            "Expected >=90% savings on compile success, got {:.1}% (input={}, output={})",
            savings,
            input_tokens,
            output_tokens
        );
    }

    #[test]
    fn test_generic_noise_snapshot() {
        let input = include_str!("../../tests/fixtures/gradle/generic_noise_raw.txt");
        let output = apply_global_filters(input);
        assert_snapshot!(output);
    }

    #[test]
    fn test_generic_noise_token_savings() {
        let input = include_str!("../../tests/fixtures/gradle/generic_noise_raw.txt");
        let output = apply_global_filters(input);
        let input_tokens = count_tokens(input);
        let output_tokens = count_tokens(&output);
        let savings = 100.0 - (output_tokens as f64 / input_tokens as f64 * 100.0);
        assert!(
            savings >= 90.0,
            "Expected >=90% savings on generic noise, got {:.1}% (input={}, output={})",
            savings,
            input_tokens,
            output_tokens
        );
    }

    #[test]
    fn test_try_block_removal() {
        let input = "Some content\n\n* Try:\n> Run with --stacktrace option.\n> Run with --info option.\n> Run with --scan.\n> Get more help at https://help.gradle.org.\n\n* What went wrong:\nSomething failed";
        let output = apply_global_filters_with_extras(input, &None);
        assert!(!output.contains("* Try:"), "Try block should be removed");
        assert!(
            !output.contains("--stacktrace"),
            "Try block content should be removed"
        );
        assert!(
            output.contains("* What went wrong:"),
            "What went wrong should be kept"
        );
    }

    #[test]
    fn test_note_lines_dropped() {
        let input = "Note: Some input files use unchecked or unsafe operations.\nNote: Recompile with -Xlint:unchecked for details.\nBUILD SUCCESSFUL in 1s";
        let output = apply_global_filters_with_extras(input, &None);
        assert!(!output.contains("Note:"), "Note: lines should be dropped");
        assert!(output.contains("BUILD SUCCESSFUL"));
    }

    #[test]
    fn test_build_result_always_kept() {
        let input = "Starting Gradle Daemon...\nBUILD SUCCESSFUL in 12s\n8 actionable tasks: 1 executed, 7 up-to-date";
        let output = apply_global_filters_with_extras(input, &None);
        assert!(output.contains("BUILD SUCCESSFUL"));
    }

    #[test]
    fn test_failure_header_kept() {
        let input = "FAILURE: Build failed with an exception\n\n* What went wrong:\nCompilation failed\n\nBUILD FAILED in 5s";
        let output = apply_global_filters_with_extras(input, &None);
        assert!(output.contains("FAILURE: Build failed with an exception"));
        assert!(output.contains("* What went wrong:"));
        assert!(output.contains("BUILD FAILED"));
    }

    #[test]
    fn test_extra_drop_patterns() {
        let input = "Normal line\nCustomOrgBuildPlugin: initializing\nAnother normal line";
        let extras = compile_extra_patterns(&["^CustomOrgBuildPlugin:".to_string()]);
        let output = apply_global_filters_with_extras(input, &extras);
        assert!(!output.contains("CustomOrgBuildPlugin"));
        assert!(output.contains("Normal line"));
        assert!(output.contains("Another normal line"));
    }

    #[test]
    fn test_invalid_extra_pattern_skipped() {
        let patterns = vec!["[invalid".to_string(), "^valid$".to_string()];
        let compiled = compile_extra_patterns(&patterns);
        assert!(
            compiled.is_some(),
            "Should produce a RegexSet with the valid pattern"
        );
        assert!(
            compiled.as_ref().unwrap().is_match("valid"),
            "Valid pattern should match"
        );
        assert!(
            !compiled.as_ref().unwrap().is_match("no match"),
            "Should not match arbitrary text"
        );
    }

    #[test]
    fn test_blank_line_trimming() {
        let input = "\n\n\nBUILD SUCCESSFUL in 1s\n\n\n";
        let output = apply_global_filters_with_extras(input, &None);
        assert!(!output.starts_with('\n'));
        assert!(!output.ends_with('\n'));
        assert!(output.contains("BUILD SUCCESSFUL"));
    }
}
