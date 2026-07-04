use std::path::{Path, PathBuf};

use crate::scanner::EcosystemDetector;

// ── Rust ─────────────────────────────────────────────────────────────────────

pub struct RustDetector;

impl EcosystemDetector for RustDetector {
    fn name(&self) -> &str {
        "rust"
    }

    fn detect_roots(&self, _deep: bool) -> Vec<PathBuf> {
        let mut roots = Vec::new();
        if let Some(home) = dirs::home_dir() {
            roots.push(home.join(".cargo/registry"));
            roots.push(home.join(".cargo/git"));
        }
        roots
    }

    fn matches(&self, path: &Path) -> bool {
        let s = path.to_string_lossy();
        // Cargo registry / git cache
        if s.contains("/.cargo/registry") || s.contains("/.cargo/git") {
            return true;
        }
        // target/ directories produced by cargo build
        if let Some(name) = path.file_name() {
            if name == "target" {
                return path.join("debug").exists() || path.join("release").exists();
            }
        }
        false
    }
}

// ── Node ──────────────────────────────────────────────────────────────────────

pub struct NodeDetector;

impl EcosystemDetector for NodeDetector {
    fn name(&self) -> &str {
        "node"
    }

    fn detect_roots(&self, _deep: bool) -> Vec<PathBuf> {
        let mut roots = Vec::new();
        if let Some(home) = dirs::home_dir() {
            roots.push(home.join(".npm/_cacache"));
            // pnpm global store (common locations)
            roots.push(home.join(".local/share/pnpm/store"));
            roots.push(home.join("Library/pnpm/store"));
        }
        roots
    }

    fn matches(&self, path: &Path) -> bool {
        let s = path.to_string_lossy();
        if s.contains("/.npm/_cacache") || s.contains("/pnpm/store") {
            return true;
        }
        // node_modules directories
        if let Some(name) = path.file_name() {
            if name == "node_modules" {
                return true;
            }
        }
        false
    }
}

// ── Python ────────────────────────────────────────────────────────────────────

pub struct PythonDetector;

impl EcosystemDetector for PythonDetector {
    fn name(&self) -> &str {
        "python"
    }

    fn detect_roots(&self, _deep: bool) -> Vec<PathBuf> {
        let mut roots = Vec::new();
        if let Some(home) = dirs::home_dir() {
            roots.push(home.join(".cache/pip"));
            roots.push(home.join(".cache/pypoetry"));
            roots.push(home.join("Library/Caches/pip"));
        }
        roots
    }

    fn matches(&self, path: &Path) -> bool {
        let s = path.to_string_lossy();
        if s.contains("/.cache/pip") || s.contains("/Library/Caches/pip") {
            return true;
        }
        if let Some(name) = path.file_name() {
            let n = name.to_string_lossy();
            // __pycache__ and compiled bytecode
            if n == "__pycache__" {
                return true;
            }
            if n.ends_with(".pyc") || n.ends_with(".pyo") {
                return true;
            }
            // virtual environments (common naming conventions)
            if n == ".venv" || n == "venv" || n == "env" {
                return true;
            }
        }
        false
    }
}

// ── Go ────────────────────────────────────────────────────────────────────────

pub struct GoDetector;

impl EcosystemDetector for GoDetector {
    fn name(&self) -> &str {
        "go"
    }

    fn detect_roots(&self, _deep: bool) -> Vec<PathBuf> {
        let mut roots = Vec::new();
        if let Some(home) = dirs::home_dir() {
            roots.push(home.join("go/pkg/mod/cache"));
        }
        // GOPATH env override
        if let Ok(gopath) = std::env::var("GOPATH") {
            roots.push(PathBuf::from(gopath).join("pkg/mod/cache"));
        }
        roots
    }

    fn matches(&self, path: &Path) -> bool {
        let s = path.to_string_lossy();
        s.contains("/go/pkg/mod/cache") || s.contains("/go/pkg/mod/download")
    }
}

// ── Docker ────────────────────────────────────────────────────────────────────

pub struct DockerDetector;

impl EcosystemDetector for DockerDetector {
    fn name(&self) -> &str {
        "docker"
    }

    fn detect_roots(&self, _deep: bool) -> Vec<PathBuf> {
        // Docker stores its data in system-level paths
        vec![
            PathBuf::from("/var/lib/docker/overlay2"),
            PathBuf::from("/var/lib/docker/volumes"),
        ]
    }

    fn matches(&self, path: &Path) -> bool {
        let s = path.to_string_lossy();
        s.starts_with("/var/lib/docker/") || s.contains("/.docker/")
    }
}

// ── Xcode ─────────────────────────────────────────────────────────────────────

pub struct XcodeDetector;

impl EcosystemDetector for XcodeDetector {
    fn name(&self) -> &str {
        "xcode"
    }

    fn detect_roots(&self, _deep: bool) -> Vec<PathBuf> {
        let mut roots = Vec::new();
        if let Some(home) = dirs::home_dir() {
            roots.push(home.join("Library/Developer/Xcode/DerivedData"));
            roots.push(home.join("Library/Developer/Xcode/Archives"));
            roots.push(home.join("Library/Caches/com.apple.dt.Xcode"));
        }
        roots
    }

    fn matches(&self, path: &Path) -> bool {
        let s = path.to_string_lossy();
        s.contains("/Library/Developer/Xcode/DerivedData")
            || s.contains("/Library/Developer/Xcode/Archives")
            || s.contains("/Library/Caches/com.apple.dt.Xcode")
    }
}

// ── Homebrew ──────────────────────────────────────────────────────────────────

pub struct HomebrewDetector;

impl EcosystemDetector for HomebrewDetector {
    fn name(&self) -> &str {
        "homebrew"
    }

    fn detect_roots(&self, _deep: bool) -> Vec<PathBuf> {
        let mut roots = Vec::new();
        if let Some(home) = dirs::home_dir() {
            roots.push(home.join("Library/Caches/Homebrew"));
        }
        // Apple Silicon default prefix
        roots.push(PathBuf::from("/opt/homebrew/var/homebrew/locks"));
        roots.push(PathBuf::from("/usr/local/var/homebrew/locks"));
        roots
    }

    fn matches(&self, path: &Path) -> bool {
        let s = path.to_string_lossy();
        s.contains("/Library/Caches/Homebrew")
            || s.contains("/Homebrew/downloads")
            || s.contains("/homebrew/cache")
    }
}

// ── Factory ───────────────────────────────────────────────────────────────────

/// Return all built-in ecosystem detectors in priority order.
pub fn all_detectors() -> Vec<Box<dyn EcosystemDetector>> {
    vec![
        Box::new(RustDetector),
        Box::new(NodeDetector),
        Box::new(PythonDetector),
        Box::new(GoDetector),
        Box::new(DockerDetector),
        Box::new(XcodeDetector),
        Box::new(HomebrewDetector),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    // ── RustDetector ──────────────────────────────────────────────────────────

    #[test]
    fn rust_matches_cargo_registry() {
        let d = RustDetector;
        let home = dirs::home_dir().unwrap();
        assert!(d.matches(&home.join(".cargo/registry/src/github.com-1/serde-1.0.0")));
    }

    #[test]
    fn rust_matches_cargo_git() {
        let d = RustDetector;
        let home = dirs::home_dir().unwrap();
        assert!(d.matches(&home.join(".cargo/git/checkouts/anyhow-abc")));
    }

    #[test]
    fn rust_does_not_match_unrelated() {
        let d = RustDetector;
        assert!(!d.matches(Path::new("/home/user/projects/my-project")));
    }

    // ── NodeDetector ──────────────────────────────────────────────────────────

    #[test]
    fn node_matches_npm_cache() {
        let d = NodeDetector;
        let home = dirs::home_dir().unwrap();
        assert!(d.matches(&home.join(".npm/_cacache/content-v2/sha512/ab")));
    }

    #[test]
    fn node_matches_node_modules() {
        let d = NodeDetector;
        assert!(d.matches(Path::new("/project/node_modules")));
    }

    #[test]
    fn node_does_not_match_unrelated() {
        let d = NodeDetector;
        assert!(!d.matches(Path::new("/project/src/index.ts")));
    }

    // ── PythonDetector ────────────────────────────────────────────────────────

    #[test]
    fn python_matches_pycache() {
        let d = PythonDetector;
        assert!(d.matches(Path::new("/project/src/__pycache__")));
    }

    #[test]
    fn python_matches_pyc_file() {
        let d = PythonDetector;
        assert!(d.matches(Path::new("/project/src/__pycache__/foo.cpython-311.pyc")));
    }

    #[test]
    fn python_matches_venv() {
        let d = PythonDetector;
        assert!(d.matches(Path::new("/project/.venv")));
        assert!(d.matches(Path::new("/project/venv")));
    }

    #[test]
    fn python_does_not_match_source() {
        let d = PythonDetector;
        assert!(!d.matches(Path::new("/project/src/main.py")));
    }

    // ── GoDetector ────────────────────────────────────────────────────────────

    #[test]
    fn go_matches_mod_cache() {
        let d = GoDetector;
        let home = dirs::home_dir().unwrap();
        assert!(d.matches(&home.join("go/pkg/mod/cache/download")));
    }

    #[test]
    fn go_does_not_match_go_src() {
        let d = GoDetector;
        let home = dirs::home_dir().unwrap();
        assert!(!d.matches(&home.join("go/src/github.com/user/repo")));
    }

    // ── XcodeDetector ─────────────────────────────────────────────────────────

    #[test]
    fn xcode_matches_derived_data() {
        let d = XcodeDetector;
        let home = dirs::home_dir().unwrap();
        assert!(d.matches(&home.join("Library/Developer/Xcode/DerivedData/MyApp-abc")));
    }

    // ── HomebrewDetector ──────────────────────────────────────────────────────

    #[test]
    fn homebrew_matches_caches() {
        let d = HomebrewDetector;
        let home = dirs::home_dir().unwrap();
        assert!(d.matches(&home.join("Library/Caches/Homebrew/downloads")));
    }

    // ── all_detectors ─────────────────────────────────────────────────────────

    #[test]
    fn all_detectors_returns_seven() {
        assert_eq!(all_detectors().len(), 7);
    }
}
