use anyhow::Result;
use humansize::{format_size, BINARY};
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// The kind of filesystem entry a ScanResult represents.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum EntryType {
    File,
    Directory,
    Symlink,
}

/// One reclaimable item surfaced by the scanner.
#[derive(Debug, Clone, Serialize)]
pub struct ScanResult {
    pub path: PathBuf,
    pub size_bytes: u64,
    #[serde(with = "system_time_secs")]
    pub last_modified: SystemTime,
    pub entry_type: EntryType,
    pub ecosystem: Option<String>,
}

/// Options that control the scan behaviour.
#[derive(Debug, Clone, Default)]
pub struct ScanOptions {
    /// Walk from home dir + known cache roots (true) or CWD only (false).
    /// Consumed by ecosystem detectors.
    #[allow(dead_code)]
    pub deep: bool,
    /// When set, only include results that match this ecosystem name.
    pub ecosystem_filter: Option<String>,
    /// When set, only include entries with mtime older than this many seconds.
    pub stale_secs: Option<u64>,
    /// Skip entries smaller than this (bytes). Avoids noisy tiny-file listings.
    pub min_size_bytes: u64,
}

/// Trait implemented by per-ecosystem detector plugins (change-dsg-005 fills these in).
/// The scanner calls `detect_roots` to find directories to walk and `matches` to tag
/// entries with an ecosystem name.
pub trait EcosystemDetector: Send + Sync {
    fn name(&self) -> &str;
    /// Return root directories for this ecosystem. Called by change-dsg-005.
    #[allow(dead_code)]
    fn detect_roots(&self, deep: bool) -> Vec<PathBuf>;
    fn matches(&self, path: &Path) -> bool;
}

/// Walk `root` in parallel using jwalk and collect reclaimable entries.
///
/// The scan is additive: when a path matches multiple detectors the first matching
/// detector's name wins (ordered by the `detectors` slice).
pub fn scan_directory(
    root: &Path,
    options: &ScanOptions,
    detectors: &[Box<dyn EcosystemDetector>],
) -> Vec<ScanResult> {
    let now = SystemTime::now();
    let mut results = Vec::new();

    let walk = jwalk::WalkDir::new(root)
        .follow_links(false)
        .parallelism(jwalk::Parallelism::RayonDefaultPool { busy_timeout: std::time::Duration::from_secs(30) });

    for entry in walk {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };

        let path = entry.path();
        let metadata = match entry.metadata() {
            Ok(m) => m,
            Err(_) => continue,
        };

        let entry_type = if metadata.is_symlink() || entry.file_type().is_symlink() {
            EntryType::Symlink
        } else if metadata.is_dir() {
            EntryType::Directory
        } else {
            EntryType::File
        };

        let size_bytes = if entry_type == EntryType::File {
            metadata.len()
        } else {
            0
        };

        if size_bytes < options.min_size_bytes {
            continue;
        }

        let last_modified = metadata.modified().unwrap_or(now);

        // Stale filter: skip if mtime is newer than the stale threshold
        if let Some(stale_secs) = options.stale_secs {
            let age = now.duration_since(last_modified).unwrap_or_default();
            if age.as_secs() < stale_secs {
                continue;
            }
        }

        // Ecosystem tagging
        let ecosystem = detectors
            .iter()
            .find(|d| d.matches(&path))
            .map(|d| d.name().to_string());

        // Ecosystem filter: skip if filter set and no match
        if let Some(ref filter) = options.ecosystem_filter {
            if ecosystem.as_deref() != Some(filter.as_str()) {
                continue;
            }
        }

        results.push(ScanResult {
            path,
            size_bytes,
            last_modified,
            entry_type,
            ecosystem,
        });
    }

    results
}

/// Collect the union of all detector roots for deep scanning.
///
/// When `opts.ecosystem_filter` is set only that detector's roots are included.
/// Deduplicates paths so overlapping detector roots don't cause double-walks.
pub fn collect_scan_roots(
    opts: &ScanOptions,
    detectors: &[Box<dyn EcosystemDetector>],
) -> Vec<PathBuf> {
    let mut seen = std::collections::HashSet::new();
    let mut roots = Vec::new();

    for d in detectors {
        if let Some(ref filter) = opts.ecosystem_filter {
            if d.name() != filter.as_str() {
                continue;
            }
        }
        for root in d.detect_roots(opts.deep) {
            if root.exists() && seen.insert(root.clone()) {
                roots.push(root);
            }
        }
    }

    roots
}

/// Collect total size for each root path returned by detectors.
/// Used by `dsg status` for a quick disk usage summary.
pub fn measure_roots(roots: &[(String, PathBuf)]) -> Vec<(String, PathBuf, u64)> {
    roots
        .iter()
        .map(|(label, root)| {
            let size = dir_size(root);
            (label.clone(), root.clone(), size)
        })
        .collect()
}

/// Recursively sum file sizes under `dir`, skipping errors silently.
pub fn dir_size(dir: &Path) -> u64 {
    if !dir.exists() {
        return 0;
    }
    jwalk::WalkDir::new(dir)
        .follow_links(false)
        .parallelism(jwalk::Parallelism::RayonDefaultPool { busy_timeout: std::time::Duration::from_secs(10) })
        .into_iter()
        .filter_map(|e| e.ok())
        .filter_map(|e| e.metadata().ok())
        .filter(|m| m.is_file())
        .map(|m| m.len())
        .sum()
}

/// Print a human-readable aligned table of scan results to stdout.
pub fn report_human(results: &[ScanResult]) {
    if results.is_empty() {
        println!("No reclaimable entries found.");
        return;
    }

    let total: u64 = results.iter().map(|r| r.size_bytes).sum();
    println!("{:<8}  {:<12}  PATH", "TYPE", "SIZE");
    println!("{}", "-".repeat(72));

    for r in results {
        let type_tag = match r.entry_type {
            EntryType::File => "file",
            EntryType::Directory => "dir",
            EntryType::Symlink => "symlink",
        };
        let eco_suffix = r
            .ecosystem
            .as_deref()
            .map(|e| format!("  [{e}]"))
            .unwrap_or_default();
        println!(
            "{:<8}  {:<12}  {}{}",
            type_tag,
            format_size(r.size_bytes, BINARY),
            r.path.display(),
            eco_suffix
        );
    }

    println!("{}", "-".repeat(72));
    println!("Total: {}  ({} entries)", format_size(total, BINARY), results.len());
}

/// Serialize results as a JSON array to stdout.
pub fn report_json(results: &[ScanResult]) -> Result<()> {
    let json = serde_json::to_string_pretty(results)?;
    println!("{json}");
    Ok(())
}

/// Print a status summary (disk usage per well-known root) to stdout.
pub fn report_status_human(measurements: &[(String, PathBuf, u64)]) {
    if measurements.is_empty() {
        println!("No known cache roots found.");
        return;
    }
    println!("{:<30}  {:<12}  PATH", "LOCATION", "SIZE");
    println!("{}", "-".repeat(80));
    for (label, path, size) in measurements {
        println!(
            "{:<30}  {:<12}  {}",
            label,
            format_size(*size, BINARY),
            path.display()
        );
    }
    let total: u64 = measurements.iter().map(|(_, _, s)| s).sum();
    println!("{}", "-".repeat(80));
    println!("Total: {}", format_size(total, BINARY));
}

/// Serialize status measurements as JSON to stdout.
pub fn report_status_json(measurements: &[(String, PathBuf, u64)]) -> Result<()> {
    let items: Vec<serde_json::Value> = measurements
        .iter()
        .map(|(label, path, size)| {
            serde_json::json!({
                "location": label,
                "path": path.to_string_lossy(),
                "size_bytes": size,
                "size_human": format_size(*size, BINARY),
            })
        })
        .collect();
    let total: u64 = measurements.iter().map(|(_, _, s)| s).sum();
    let output = serde_json::json!({
        "roots": items,
        "total_bytes": total,
        "total_human": format_size(total, BINARY),
    });
    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}

// ── serde helper: serialize SystemTime as Unix epoch seconds ─────────────────

mod system_time_secs {
    use serde::Serializer;
    use std::time::{SystemTime, UNIX_EPOCH};

    pub fn serialize<S: Serializer>(t: &SystemTime, s: S) -> Result<S::Ok, S::Error> {
        let secs = t
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        s.serialize_u64(secs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    struct AlwaysDetector;
    impl EcosystemDetector for AlwaysDetector {
        fn name(&self) -> &str {
            "test-eco"
        }
        fn detect_roots(&self, _deep: bool) -> Vec<PathBuf> {
            vec![]
        }
        fn matches(&self, _path: &Path) -> bool {
            true
        }
    }

    struct NeverDetector;
    impl EcosystemDetector for NeverDetector {
        fn name(&self) -> &str {
            "never"
        }
        fn detect_roots(&self, _deep: bool) -> Vec<PathBuf> {
            vec![]
        }
        fn matches(&self, _path: &Path) -> bool {
            false
        }
    }

    fn make_tree(dir: &TempDir) -> PathBuf {
        let root = dir.path().to_path_buf();
        fs::write(root.join("file_a.txt"), "hello world").unwrap();
        fs::write(root.join("file_b.bin"), vec![0u8; 1024]).unwrap();
        let sub = root.join("subdir");
        fs::create_dir_all(&sub).unwrap();
        fs::write(sub.join("nested.rs"), "fn main() {}").unwrap();
        root
    }

    #[test]
    fn scan_finds_files_in_temp_dir() {
        let dir = TempDir::new().unwrap();
        let root = make_tree(&dir);
        let opts = ScanOptions::default();
        let results = scan_directory(&root, &opts, &[]);
        // At minimum, the files we wrote should appear
        let files: Vec<_> = results
            .iter()
            .filter(|r| r.entry_type == EntryType::File)
            .collect();
        assert!(files.len() >= 3, "expected at least 3 files, got {}", files.len());
    }

    #[test]
    fn scan_tags_ecosystem_when_detector_matches() {
        let dir = TempDir::new().unwrap();
        let root = make_tree(&dir);
        let opts = ScanOptions::default();
        let detectors: Vec<Box<dyn EcosystemDetector>> = vec![Box::new(AlwaysDetector)];
        let results = scan_directory(&root, &opts, &detectors);
        let tagged: Vec<_> = results
            .iter()
            .filter(|r| r.ecosystem.as_deref() == Some("test-eco"))
            .collect();
        assert!(!tagged.is_empty(), "AlwaysDetector should tag every entry");
    }

    #[test]
    fn scan_filters_by_ecosystem() {
        let dir = TempDir::new().unwrap();
        let root = make_tree(&dir);
        let opts = ScanOptions {
            ecosystem_filter: Some("never".to_string()),
            ..Default::default()
        };
        let detectors: Vec<Box<dyn EcosystemDetector>> = vec![Box::new(NeverDetector)];
        let results = scan_directory(&root, &opts, &detectors);
        // NeverDetector never matches → nothing passes the ecosystem filter
        assert!(results.is_empty(), "NeverDetector filter should yield 0 results");
    }

    #[test]
    fn scan_respects_min_size() {
        let dir = TempDir::new().unwrap();
        let root = dir.path().to_path_buf();
        // Write a 5-byte file
        fs::write(root.join("tiny.txt"), "hello").unwrap();
        // Write a 2048-byte file
        fs::write(root.join("big.bin"), vec![0u8; 2048]).unwrap();

        let opts = ScanOptions {
            min_size_bytes: 1024,
            ..Default::default()
        };
        let results = scan_directory(&root, &opts, &[]);
        let files: Vec<_> = results
            .iter()
            .filter(|r| r.entry_type == EntryType::File)
            .collect();
        assert_eq!(files.len(), 1, "Only the 2048-byte file should pass min_size filter");
        assert!(files[0].path.ends_with("big.bin"));
    }

    #[test]
    fn dir_size_empty_dir_is_zero() {
        let dir = TempDir::new().unwrap();
        assert_eq!(dir_size(dir.path()), 0);
    }

    #[test]
    fn dir_size_sums_files() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();
        fs::write(root.join("a"), vec![0u8; 100]).unwrap();
        fs::write(root.join("b"), vec![0u8; 200]).unwrap();
        let size = dir_size(root);
        assert_eq!(size, 300);
    }

    #[test]
    fn report_json_emits_valid_json() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("x.txt"), "data").unwrap();
        let opts = ScanOptions::default();
        let results = scan_directory(dir.path(), &opts, &[]);
        // Just check it doesn't error — actual JSON parsing checked implicitly by serde_json
        report_json(&results).unwrap();
    }
}
