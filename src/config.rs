use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub tracking: TrackingConfig,
    #[serde(default)]
    pub display: DisplayConfig,
    #[serde(default)]
    pub filters: FilterConfig,
    #[serde(default)]
    pub tee: crate::tee::TeeConfig,
    #[serde(default)]
    pub telemetry: TelemetryConfig,
    #[serde(default)]
    pub hooks: HooksConfig,
    #[serde(default)]
    pub limits: LimitsConfig,
    #[serde(default)]
    pub gradle: GradleConfig,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct HooksConfig {
    /// Commands to exclude from auto-rewrite (e.g. ["curl", "playwright"]).
    /// Survives `rtk init -g` re-runs since config.toml is user-owned.
    #[serde(default)]
    pub exclude_commands: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TrackingConfig {
    pub enabled: bool,
    pub history_days: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub database_path: Option<PathBuf>,
}

impl Default for TrackingConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            history_days: 90,
            database_path: None,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DisplayConfig {
    pub colors: bool,
    pub emoji: bool,
    pub max_width: usize,
}

impl Default for DisplayConfig {
    fn default() -> Self {
        Self {
            colors: true,
            emoji: true,
            max_width: 120,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FilterConfig {
    pub ignore_dirs: Vec<String>,
    pub ignore_files: Vec<String>,
}

impl Default for FilterConfig {
    fn default() -> Self {
        Self {
            ignore_dirs: vec![
                ".git".into(),
                "node_modules".into(),
                "target".into(),
                "__pycache__".into(),
                ".venv".into(),
                "vendor".into(),
            ],
            ignore_files: vec!["*.lock".into(), "*.min.js".into(), "*.min.css".into()],
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TelemetryConfig {
    pub enabled: bool,
}

impl Default for TelemetryConfig {
    fn default() -> Self {
        Self { enabled: true }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LimitsConfig {
    /// Max total grep results to show (default: 200)
    pub grep_max_results: usize,
    /// Max matches per file in grep output (default: 25)
    pub grep_max_per_file: usize,
    /// Max staged/modified files shown in git status (default: 15)
    pub status_max_files: usize,
    /// Max untracked files shown in git status (default: 10)
    pub status_max_untracked: usize,
    /// Max chars for parser passthrough fallback (default: 2000)
    pub passthrough_max_chars: usize,
}

impl Default for LimitsConfig {
    fn default() -> Self {
        Self {
            grep_max_results: 200,
            grep_max_per_file: 25,
            status_max_files: 15,
            status_max_untracked: 10,
            passthrough_max_chars: 2000,
        }
    }
}

/// Get limits config. Falls back to defaults if config can't be loaded.
pub fn limits() -> LimitsConfig {
    Config::load().map(|c| c.limits).unwrap_or_default()
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct GradleConfig {
    /// Package prefixes treated as "user code" in stack traces (kept, not dropped).
    /// Default: [] (empty — all frames treated as framework noise).
    #[serde(default)]
    pub user_packages: Vec<String>,
    /// Additional regex patterns for lines to drop (applied after built-in global filters).
    #[serde(default)]
    pub extra_drop_patterns: Vec<String>,
    /// Additional package prefixes to treat as framework noise in stack traces.
    /// Built-in: java.*, kotlin.*, sun.*, javax.*, jdk.*, jakarta.*, *.internal.*
    /// Default extras: org.gradle, org.junit.platform, org.junit.jupiter.engine,
    ///   org.springframework, com.google.inject, io.grpc, org.mockito, io.mockk,
    ///   org.eclipse.jetty
    #[serde(default = "default_drop_frame_packages")]
    pub drop_frame_packages: Vec<String>,
}

pub fn default_drop_frame_packages() -> Vec<String> {
    vec![
        "org.gradle".to_string(),
        "org.junit.platform".to_string(),
        "org.junit.jupiter.engine".to_string(),
        "org.springframework".to_string(),
        "com.google.inject".to_string(),
        "io.grpc".to_string(),
        "org.mockito".to_string(),
        "io.mockk".to_string(),
        "org.eclipse.jetty".to_string(),
        "org.assertj.core.internal".to_string(),
    ]
}

/// Repo-level config — only fields that make sense at repo scope.
/// Absent sections deserialize as None (not overridden).
#[derive(Debug, Deserialize, Default)]
struct RepoConfig {
    #[serde(default)]
    gradle: Option<GradleConfig>,
    #[serde(default)]
    hooks: Option<HooksConfig>,
    #[serde(default)]
    filters: Option<FilterConfig>,
}

/// Walk up from `start` to find `.rtk.toml`, stopping at `.git` boundary.
fn find_repo_config_from(start: &std::path::Path) -> Option<PathBuf> {
    let mut dir = start.to_path_buf();
    loop {
        let candidate = dir.join(".rtk.toml");
        if candidate.exists() {
            return Some(candidate);
        }
        // Stop at .git boundary (repo root)
        if dir.join(".git").exists() {
            return None;
        }
        if !dir.pop() {
            return None;
        }
    }
}

/// Check if telemetry is enabled in config. Returns None if config can't be loaded.
pub fn telemetry_enabled() -> Option<bool> {
    Config::load().ok().map(|c| c.telemetry.enabled)
}

impl Config {
    pub fn load() -> Result<Self> {
        let cwd = std::env::current_dir().unwrap_or_default();
        Self::load_from_dir(&cwd)
    }

    /// Load config: user-level first, then merge repo-level `.rtk.toml` if found.
    pub fn load_from_dir(cwd: &std::path::Path) -> Result<Self> {
        let user_path = get_config_path()?;
        let mut config = if user_path.exists() {
            let content = std::fs::read_to_string(&user_path)?;
            toml::from_str(&content)?
        } else {
            Config::default()
        };

        if let Some(repo_path) = find_repo_config_from(cwd) {
            if let Ok(repo_toml) = std::fs::read_to_string(&repo_path) {
                config.merge_repo(&repo_toml);
            }
        }

        Ok(config)
    }

    /// Merge repo-level config on top of self. Repo wins for present fields.
    /// Only `[gradle]`, `[hooks]`, and `[filters]` are repo-scoped.
    fn merge_repo(&mut self, repo_toml: &str) {
        let repo: RepoConfig = match toml::from_str(repo_toml) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("rtk: invalid .rtk.toml: {}", e);
                return;
            }
        };

        if let Some(gradle) = repo.gradle {
            self.gradle = gradle;
        }
        if let Some(hooks) = repo.hooks {
            self.hooks = hooks;
        }
        if let Some(filters) = repo.filters {
            self.filters = filters;
        }
    }

    pub fn save(&self) -> Result<()> {
        let path = get_config_path()?;

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let content = toml::to_string_pretty(self)?;
        std::fs::write(&path, content)?;
        Ok(())
    }

    pub fn create_default() -> Result<PathBuf> {
        let config = Config::default();
        config.save()?;
        get_config_path()
    }
}

fn get_config_path() -> Result<PathBuf> {
    let config_dir = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
    Ok(config_dir.join("rtk").join("config.toml"))
}

pub fn show_config() -> Result<()> {
    let user_path = get_config_path()?;
    println!("User config: {}", user_path.display());
    if user_path.exists() {
        println!("  (found)");
    } else {
        println!("  (not found, using defaults)");
    }

    let cwd = std::env::current_dir().unwrap_or_default();
    if let Some(repo_path) = find_repo_config_from(&cwd) {
        println!("Repo config: {}", repo_path.display());
    } else {
        println!("Repo config: (none)");
    }

    println!();
    println!("Effective config (merged):");
    println!();
    let config = Config::load()?;
    println!("{}", toml::to_string_pretty(&config)?);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hooks_config_deserialize() {
        let toml = r#"
[hooks]
exclude_commands = ["curl", "gh"]
"#;
        let config: Config = toml::from_str(toml).expect("valid toml");
        assert_eq!(config.hooks.exclude_commands, vec!["curl", "gh"]);
    }

    #[test]
    fn test_hooks_config_default_empty() {
        let config = Config::default();
        assert!(config.hooks.exclude_commands.is_empty());
    }

    #[test]
    fn test_config_without_hooks_section_is_valid() {
        let toml = r#"
[tracking]
enabled = true
history_days = 90
"#;
        let config: Config = toml::from_str(toml).expect("valid toml");
        assert!(config.hooks.exclude_commands.is_empty());
    }

    // --- find_repo_config_from tests ---

    #[test]
    fn test_find_repo_config_returns_none_when_no_rtk_toml() {
        let tmp = std::env::temp_dir().join("rtk_test_no_config");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        let result = find_repo_config_from(&tmp);
        assert!(result.is_none());
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_find_repo_config_finds_rtk_toml_in_current_dir() {
        let tmp = std::env::temp_dir().join("rtk_test_find_config");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        std::fs::create_dir_all(tmp.join(".git")).unwrap();
        std::fs::write(
            tmp.join(".rtk.toml"),
            "[gradle]\nuser_packages = [\"com.faire\"]\n",
        )
        .unwrap();
        let result = find_repo_config_from(&tmp);
        assert_eq!(result, Some(tmp.join(".rtk.toml")));
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_find_repo_config_walks_up_to_git_root() {
        let tmp = std::env::temp_dir().join("rtk_test_walk_up");
        let _ = std::fs::remove_dir_all(&tmp);
        let subdir = tmp.join("app").join("billing");
        std::fs::create_dir_all(&subdir).unwrap();
        std::fs::create_dir_all(tmp.join(".git")).unwrap();
        std::fs::write(
            tmp.join(".rtk.toml"),
            "[gradle]\nuser_packages = [\"com.faire\"]\n",
        )
        .unwrap();
        let result = find_repo_config_from(&subdir);
        assert_eq!(result, Some(tmp.join(".rtk.toml")));
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_find_repo_config_stops_at_git_boundary() {
        let tmp = std::env::temp_dir().join("rtk_test_git_boundary");
        let _ = std::fs::remove_dir_all(&tmp);
        let inner = tmp.join("inner_repo");
        std::fs::create_dir_all(inner.join(".git")).unwrap();
        std::fs::write(
            tmp.join(".rtk.toml"),
            "[gradle]\nuser_packages = [\"com.faire\"]\n",
        )
        .unwrap();
        let result = find_repo_config_from(&inner);
        assert!(
            result.is_none(),
            ".rtk.toml above .git boundary should not be found"
        );
        let _ = std::fs::remove_dir_all(&tmp);
    }

    // --- merge_repo tests ---

    #[test]
    fn test_merge_repo_gradle_overrides_user() {
        let mut user = Config::default();
        user.gradle.user_packages = vec!["com.user".to_string()];

        let repo_toml = r#"
[gradle]
user_packages = ["com.faire"]
"#;
        user.merge_repo(repo_toml);
        assert_eq!(user.gradle.user_packages, vec!["com.faire"]);
    }

    #[test]
    fn test_merge_repo_absent_section_keeps_user() {
        let mut user = Config::default();
        user.gradle.user_packages = vec!["com.user".to_string()];
        user.tracking.history_days = 30;

        let repo_toml = r#"
[hooks]
exclude_commands = ["curl"]
"#;
        user.merge_repo(repo_toml);
        assert_eq!(user.gradle.user_packages, vec!["com.user"]);
        assert_eq!(user.hooks.exclude_commands, vec!["curl"]);
        assert_eq!(user.tracking.history_days, 30);
    }

    #[test]
    fn test_merge_repo_ignores_user_only_sections() {
        let mut user = Config::default();
        user.tracking.history_days = 30;
        user.display.max_width = 80;

        let repo_toml = r#"
[tracking]
history_days = 999

[display]
max_width = 200

[gradle]
user_packages = ["com.faire"]
"#;
        user.merge_repo(repo_toml);
        assert_eq!(user.tracking.history_days, 30, "tracking is user-only");
        assert_eq!(user.display.max_width, 80, "display is user-only");
        assert_eq!(
            user.gradle.user_packages,
            vec!["com.faire"],
            "gradle is repo-scoped"
        );
    }

    // --- load_from_dir integration test ---

    #[test]
    fn test_load_with_repo_config_integration() {
        let tmp = std::env::temp_dir().join("rtk_test_load_integration");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(tmp.join(".git")).unwrap();
        std::fs::write(
            tmp.join(".rtk.toml"),
            "[gradle]\nuser_packages = [\"com.faire\"]\n",
        )
        .unwrap();

        let config = Config::load_from_dir(&tmp).unwrap();
        assert_eq!(config.gradle.user_packages, vec!["com.faire"]);
        let _ = std::fs::remove_dir_all(&tmp);
    }
}
