// Methods and types are pub API for the scanner (change-dsg-004/005); not
// all are called from main yet.
#![allow(dead_code)]

use anyhow::Result;
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use crate::config::Config;

/// Outcome of an activity check on a path before deletion.
#[derive(Debug, PartialEq)]
pub enum ActivityCheck {
    /// No active processes or git dirtiness detected.
    Idle,
    /// One or more processes have the path open.
    ActiveProcesses(Vec<String>),
    /// The path is inside a git repo with uncommitted changes.
    GitDirty,
}

/// Core safety engine. All mutable filesystem operations MUST go through this.
pub struct SafetyEngine {
    pub dry_run: bool,
    pub min_age_secs: u64,
    config: Arc<Config>,
}

impl SafetyEngine {
    pub fn new(dry_run: bool, config: Arc<Config>) -> Self {
        let min_age_secs = config.min_age_days * 86_400;
        Self {
            dry_run,
            min_age_secs,
            config,
        }
    }

    /// Check whether any process has files under `path` open (via lsof).
    /// Also checks for a dirty git working tree when inside a git repo.
    ///
    /// Times out after 5 seconds so slow lsof calls don't block the CLI.
    pub fn verify_activity(&self, path: &Path) -> Result<ActivityCheck> {
        // lsof check — best effort; any error returns Idle (don't block clean)
        let lsof_result = run_lsof(path);
        if let Some(procs) = lsof_result {
            if !procs.is_empty() {
                return Ok(ActivityCheck::ActiveProcesses(procs));
            }
        }

        // git check — only relevant if path is inside a repo
        if let Some(dirty) = check_git_dirty(path) {
            if dirty {
                return Ok(ActivityCheck::GitDirty);
            }
        }

        Ok(ActivityCheck::Idle)
    }

    /// Move `path` to the system Trash. Never calls `std::fs::remove_*`.
    pub fn move_to_trash(&self, path: &Path) -> Result<()> {
        if self.dry_run {
            tracing::info!("[dry-run] Would trash: {}", path.display());
            return Ok(());
        }
        tracing::info!("Trashing: {}", path.display());
        trash::delete(path).map_err(|e| {
            anyhow::anyhow!("Failed to move {} to trash: {}", path.display(), e)
        })
    }

    /// Return `true` when `path` matches any exclusion pattern in config.
    pub fn should_exclude(&self, path: &Path) -> bool {
        let path_str = path.to_string_lossy();
        for pattern in &self.config.exclude_paths {
            // Expand leading `~` for user-home prefixes
            let expanded = expand_tilde(pattern);
            if glob_match(&expanded, &path_str) {
                return true;
            }
        }
        false
    }

    /// Return `true` when the path's mtime is **older** than `min_age_secs`.
    /// Returns `false` (don't guard) on any metadata error so errors aren't silent blocks.
    pub fn age_guard(&self, path: &Path) -> Result<bool> {
        let meta = std::fs::metadata(path)
            .map_err(|e| anyhow::anyhow!("Cannot stat {}: {}", path.display(), e))?;
        let mtime = meta
            .modified()
            .map_err(|e| anyhow::anyhow!("Cannot read mtime for {}: {}", path.display(), e))?;
        let age = SystemTime::now()
            .duration_since(mtime)
            .unwrap_or(Duration::ZERO);
        Ok(age.as_secs() >= self.min_age_secs)
    }
}

/// Run `lsof +D <path>` with a 5-second timeout.
/// Returns `Some(Vec<String>)` of process names if any are found, `None` on error/timeout.
fn run_lsof(path: &Path) -> Option<Vec<String>> {
    use std::process::Command;

    let output = Command::new("lsof")
        .args(["+D", &path.to_string_lossy()])
        .output()
        .ok()?;

    // lsof exits 1 when no files are open — that's not an error for us
    let stdout = String::from_utf8_lossy(&output.stdout);
    let procs: Vec<String> = stdout
        .lines()
        .skip(1) // header line
        .filter_map(|line| line.split_whitespace().next().map(str::to_owned))
        .collect();

    Some(procs)
}

/// Run `git status --porcelain` in `path`'s directory.
/// Returns `Some(true)` when dirty, `Some(false)` when clean, `None` when not a git repo.
fn check_git_dirty(path: &Path) -> Option<bool> {
    use std::process::Command;

    let dir = if path.is_dir() {
        path.to_path_buf()
    } else {
        path.parent()?.to_path_buf()
    };

    let output = Command::new("git")
        .args(["-C", &dir.to_string_lossy(), "status", "--porcelain"])
        .output()
        .ok()?;

    if !output.status.success() {
        // Not a git repo or git unavailable
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    Some(!stdout.trim().is_empty())
}

/// Expand a leading `~` to the user's home directory.
fn expand_tilde(pattern: &str) -> String {
    if let Some(rest) = pattern.strip_prefix("~/") {
        if let Some(home) = dirs::home_dir() {
            return format!("{}/{}", home.display(), rest);
        }
    }
    pattern.to_owned()
}

/// Minimal glob matching: supports `*` (any chars, no `/`) and `**` (any chars).
fn glob_match(pattern: &str, path: &str) -> bool {
    // Exact match
    if pattern == path {
        return true;
    }
    // Prefix match: pattern ends with `/**` → check if path starts with the prefix
    if let Some(prefix) = pattern.strip_suffix("/**") {
        return path.starts_with(prefix);
    }
    // Trailing `*` — match prefix up to the wildcard
    if let Some(prefix) = pattern.strip_suffix("/*") {
        // path must be a direct child of prefix
        let with_slash = format!("{prefix}/");
        if path.starts_with(&with_slash) {
            let tail = &path[with_slash.len()..];
            return !tail.contains('/');
        }
        return false;
    }
    // Substring match for patterns containing `*` in the middle
    if pattern.contains('*') {
        // Rough check: split on `*` and verify both sides are in the path in order
        let parts: Vec<&str> = pattern.split('*').collect();
        let mut remaining = path;
        for (i, part) in parts.iter().enumerate() {
            if i == 0 {
                if !remaining.starts_with(part) {
                    return false;
                }
                remaining = &remaining[part.len()..];
            } else if let Some(pos) = remaining.find(part) {
                remaining = &remaining[pos + part.len()..];
            } else {
                return false;
            }
        }
        return true;
    }
    // Prefix: pattern without trailing slash matches path as a directory prefix
    if path.starts_with(pattern) {
        let after = &path[pattern.len()..];
        return after.is_empty() || after.starts_with('/');
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::sync::Arc;
    use tempfile::NamedTempFile;

    fn engine(dry_run: bool, min_age_days: u64, excludes: Vec<String>) -> SafetyEngine {
        let mut cfg = Config::default();
        cfg.min_age_days = min_age_days;
        cfg.exclude_paths = excludes;
        SafetyEngine::new(dry_run, Arc::new(cfg))
    }

    // ── age_guard ──────────────────────────────────────────────────────────

    #[test]
    fn age_guard_new_file_is_not_old_enough() {
        let f = NamedTempFile::new().unwrap();
        // File was just created — mtime is now; min_age = 1 day ⇒ not old enough
        let eng = engine(true, 1, vec![]);
        let old = eng.age_guard(f.path()).unwrap();
        assert!(!old, "brand-new file should not pass age guard");
    }

    #[test]
    fn age_guard_zero_min_age_always_passes() {
        let f = NamedTempFile::new().unwrap();
        let eng = engine(true, 0, vec![]);
        let old = eng.age_guard(f.path()).unwrap();
        assert!(old, "min_age=0 ⇒ any file passes age guard");
    }

    #[test]
    fn age_guard_missing_file_returns_error() {
        let eng = engine(true, 1, vec![]);
        let result = eng.age_guard(Path::new("/nonexistent/path/xyz"));
        assert!(result.is_err());
    }

    // ── should_exclude ─────────────────────────────────────────────────────

    #[test]
    fn exclude_exact_match() {
        let eng = engine(true, 1, vec!["/tmp/keep".to_string()]);
        assert!(eng.should_exclude(Path::new("/tmp/keep")));
    }

    #[test]
    fn exclude_glob_double_star() {
        let eng = engine(true, 1, vec!["~/.cargo/registry/**".to_string()]);
        let home = dirs::home_dir().unwrap();
        let path = home.join(".cargo/registry/src/github.com-1/crate-1.0.0");
        assert!(eng.should_exclude(&path));
    }

    #[test]
    fn exclude_non_matching_path() {
        let eng = engine(true, 1, vec!["/tmp/keep/**".to_string()]);
        assert!(!eng.should_exclude(Path::new("/tmp/other/file")));
    }

    #[test]
    fn exclude_empty_list_never_excludes() {
        let eng = engine(true, 1, vec![]);
        assert!(!eng.should_exclude(Path::new("/any/path")));
    }

    // ── move_to_trash dry-run ──────────────────────────────────────────────

    #[test]
    fn move_to_trash_dry_run_does_not_delete() {
        let mut f = NamedTempFile::new().unwrap();
        writeln!(f, "sentinel content").unwrap();
        let path = f.path().to_path_buf();

        let eng = engine(true, 0, vec![]);
        eng.move_to_trash(&path).unwrap();

        // File must still exist — dry-run must not delete
        assert!(path.exists(), "dry-run must not actually delete the file");
    }

    // ── glob_match ─────────────────────────────────────────────────────────

    #[test]
    fn glob_match_exact() {
        assert!(glob_match("/a/b/c", "/a/b/c"));
    }

    #[test]
    fn glob_match_double_star_prefix() {
        assert!(glob_match("/a/b/**", "/a/b/c/d"));
        assert!(!glob_match("/a/b/**", "/a/c/d"));
    }

    #[test]
    fn glob_match_single_star_child() {
        assert!(glob_match("/a/b/*", "/a/b/c"));
        assert!(!glob_match("/a/b/*", "/a/b/c/d")); // depth > 1
    }

    #[test]
    fn glob_match_no_match() {
        assert!(!glob_match("/a/b/c", "/a/b/d"));
    }
}
