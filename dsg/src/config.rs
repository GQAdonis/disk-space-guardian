use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    #[serde(default)]
    pub exclude_paths: Vec<String>,

    #[serde(default = "default_min_age_days")]
    pub min_age_days: u64,

    #[serde(default = "default_min_size_mb")]
    pub min_size_mb: u64,

    #[serde(default = "default_dry_run")]
    pub dry_run_default: bool,

    #[serde(default = "default_log_level")]
    pub log_level: String,
}

fn default_min_age_days() -> u64 {
    1
}

fn default_min_size_mb() -> u64 {
    10
}

fn default_dry_run() -> bool {
    true
}

fn default_log_level() -> String {
    "info".to_string()
}

impl Default for Config {
    fn default() -> Self {
        Self {
            exclude_paths: vec![],
            min_age_days: default_min_age_days(),
            min_size_mb: default_min_size_mb(),
            dry_run_default: default_dry_run(),
            log_level: default_log_level(),
        }
    }
}

impl Config {
    /// Load config from the given path, or the default location, or fall back to defaults.
    ///
    /// Resolution order:
    /// 1. `override_path` argument
    /// 2. `$DSG_CONFIG` environment variable
    /// 3. `~/.config/dsg/config.toml`
    ///
    /// If the resolved path does not exist, returns `Config::default()` silently.
    /// If the file exists but is malformed, returns an error.
    pub fn load(override_path: Option<&Path>) -> Result<Self> {
        let path = resolve_config_path(override_path);

        if let Some(p) = &path {
            if p.exists() {
                let raw = std::fs::read_to_string(p)
                    .with_context(|| format!("Failed to read config file: {}", p.display()))?;
                let cfg: Self = toml::from_str(&raw)
                    .with_context(|| format!("Failed to parse config file: {}", p.display()))?;
                validate_config(&cfg)?;
                return Ok(cfg);
            }
        }

        Ok(Self::default())
    }
}

fn resolve_config_path(override_path: Option<&Path>) -> Option<PathBuf> {
    if let Some(p) = override_path {
        return Some(p.to_path_buf());
    }
    if let Ok(env_path) = std::env::var("DSG_CONFIG") {
        return Some(PathBuf::from(env_path));
    }
    dirs::home_dir().map(|h| h.join(".config").join("dsg").join("config.toml"))
}

fn validate_config(cfg: &Config) -> Result<()> {
    const VALID_LOG_LEVELS: &[&str] = &["error", "warn", "info", "debug", "trace"];
    if !VALID_LOG_LEVELS.contains(&cfg.log_level.as_str()) {
        anyhow::bail!(
            "Invalid log_level {:?}. Valid values: {}",
            cfg.log_level,
            VALID_LOG_LEVELS.join(", ")
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn defaults_when_no_file() {
        let cfg = Config::load(Some(Path::new("/nonexistent/path/config.toml"))).unwrap();
        assert_eq!(cfg.min_age_days, 1);
        assert_eq!(cfg.min_size_mb, 10);
        assert!(cfg.dry_run_default);
        assert_eq!(cfg.log_level, "info");
        assert!(cfg.exclude_paths.is_empty());
    }

    #[test]
    fn parses_valid_toml() {
        let mut f = NamedTempFile::new().unwrap();
        writeln!(
            f,
            r#"min_age_days = 7
min_size_mb = 50
dry_run_default = false
log_level = "debug"
exclude_paths = ["/tmp/keep/**"]
"#
        )
        .unwrap();

        let cfg = Config::load(Some(f.path())).unwrap();
        assert_eq!(cfg.min_age_days, 7);
        assert_eq!(cfg.min_size_mb, 50);
        assert!(!cfg.dry_run_default);
        assert_eq!(cfg.log_level, "debug");
        assert_eq!(cfg.exclude_paths, vec!["/tmp/keep/**"]);
    }

    #[test]
    fn rejects_invalid_log_level() {
        let mut f = NamedTempFile::new().unwrap();
        writeln!(f, r#"log_level = "verbose""#).unwrap();
        let err = Config::load(Some(f.path())).unwrap_err();
        assert!(err.to_string().contains("log_level"));
    }

    #[test]
    fn partial_config_uses_defaults_for_missing_fields() {
        let mut f = NamedTempFile::new().unwrap();
        writeln!(f, r#"min_age_days = 30"#).unwrap();
        let cfg = Config::load(Some(f.path())).unwrap();
        assert_eq!(cfg.min_age_days, 30);
        assert_eq!(cfg.min_size_mb, 10); // default
        assert!(cfg.dry_run_default); // default
    }
}
