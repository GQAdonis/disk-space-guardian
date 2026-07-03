# Capability Spec: Scanner Algorithm

**Spec ID:** dsg-spec-scanner  
**Phase:** Phase 1 (MVP)  
**Status:** approved  

---

## Overview

The scanner is responsible for parallel filesystem traversal, size calculation, staleness scoring, and ecosystem detection. It produces `ScanResult` entries that are then passed to the safety pipeline before any cleanup.

---

## Directory Walkers

**Primary walker:** `jwalk` crate — parallel directory walk using a thread pool.  
**Fallback walker:** `walkdir` crate — single-threaded, safe, used for:
- Verification passes (confirming a candidate still exists before deletion)
- Environments where parallel I/O causes issues (detected via config flag in Phase 2)

**Walker selection rule:** use `jwalk` for the initial scan; use `walkdir` for any verification pass immediately before deletion.

---

## Core Data Types

### `ScanResult`

Represents a single filesystem entry identified as a cleanup candidate.

```rust
pub struct ScanResult {
    /// Absolute path to the entry.
    pub path: PathBuf,

    /// Total size in bytes (recursive for directories).
    pub size_bytes: u64,

    /// Last modification time (mtime) of the entry itself.
    /// For directories: mtime of the directory inode (not recursive max).
    pub last_modified: SystemTime,

    /// Whether this entry is a file, directory, or symlink.
    pub entry_type: EntryType,

    /// The detected ecosystem, if any.
    pub ecosystem: Option<Ecosystem>,

    /// Human-readable description of what this entry is (from the detector).
    pub description: String,
}

pub enum EntryType {
    File,
    Dir,
    Symlink,
}

pub enum Ecosystem {
    Rust,
    Node,
    Python,
    Go,
    Docker,
    Xcode,
    Homebrew,
}
```

---

## Sorting

Scan results are sorted by `size_bytes` descending by default. This surfaces the largest candidates at the top of the output, maximizing reclamation value per user decision.

No secondary sort key is defined for Phase 1. Phase 2 may add staleness-weighted scoring.

---

## Output Formats

### Human table (default)

```
Path                              Size     Age      Ecosystem
────────────────────────────────────────────────────────────────
./my-app/target                   2.4 GB   12d      rust
./my-app/node_modules             340 MB   5d       node
./old-proj/target                 1.1 GB   87d      rust
────────────────────────────────────────────────────────────────
Total reclaimable: 3.8 GB (3 entries)
```

**Column definitions:**
- `Path`: relative to scan root, or absolute if `--deep`
- `Size`: human-readable (GiB, MiB, KiB) using `humansize` crate
- `Age`: days since `last_modified` (e.g., `12d`, `87d`, `<1d`)
- `Ecosystem`: lowercase ecosystem name or `-` if undetected

### JSON output (`--json`)

```json
{
  "scan_root": "/absolute/path",
  "scanned_at": "2026-07-03T10:00:00Z",
  "total_bytes": 4077715456,
  "entry_count": 3,
  "entries": [
    {
      "path": "/absolute/path/my-app/target",
      "size_bytes": 2578849792,
      "last_modified_epoch_secs": 1751500000,
      "age_days": 12,
      "entry_type": "Dir",
      "ecosystem": "rust",
      "description": "Rust build artifacts (cargo target directory)"
    }
  ]
}
```

---

## Performance Target

**Scan a 10 GB directory tree in < 30 seconds on typical developer hardware.**

"Typical developer hardware" = Apple M-series or AMD Ryzen 7+ with NVMe SSD, 16 GB RAM.

This target applies to `dsg scan` (CWD scan). `dsg scan --deep` (full system scan) is exempt from this target for Phase 1 — it runs as fast as `jwalk` allows.

**Size calculation for directories:** use parallel sum of all file sizes via `jwalk`'s parallel callback. Do not `du` shell out.

---

## `EcosystemDetector` Trait

Each supported ecosystem has an implementation of this trait. The scanner calls `detect()` on each registered detector for every directory it visits.

```rust
pub trait EcosystemDetector: Send + Sync {
    /// Machine name of the ecosystem (e.g., "rust", "node").
    fn name(&self) -> &str;

    /// Given a directory path, return the list of candidate sub-paths
    /// within that directory that are cleanup candidates for this ecosystem.
    ///
    /// Returns an empty Vec if this directory has no candidates.
    /// This method must be cheap — it should stat at most a handful of files.
    fn detect(&self, root: &Path) -> Vec<PathBuf>;

    /// Human-readable description of what a detected path is.
    /// Used in scan output and audit logs.
    ///
    /// Example: "Rust build artifacts (cargo target directory)"
    fn describe(&self, path: &Path) -> String;
}
```

---

## Phase 1 Ecosystem Detectors

### Rust

**Trigger:** directory named `target/` AND sibling `Cargo.toml` exists.  
**Candidates:** the `target/` directory itself.  
**Description:** `"Rust build artifacts (cargo target directory)"`

**Special case:** `target/` under `~/.cargo/` (the global registry build cache) is handled separately by the `RustCacheDetector`, not the project `target/` detector.

---

### Node.js

**Trigger:** directory named `node_modules/` AND sibling `package.json` exists.  
**Candidates:** the `node_modules/` directory.  
**Description:** `"Node.js dependencies (node_modules)"`

**pnpm hardlink guard:** if `node_modules/.pnpm/` exists, the detector marks the entry with a warning that pnpm hardlinks may be present. The safety module will log this; cleanup proceeds normally (pnpm hardlinks are stored in the global pnpm store, not in this directory).

---

### Python

**Trigger:** directory named `.venv/` or `venv/` or `env/` AND (sibling `pyproject.toml` OR sibling `requirements.txt` OR sibling `setup.py`) exists.  
**Candidates:** the virtual environment directory.  
**Description:** `"Python virtual environment (.venv)"`

---

### Go

**Trigger:** not based on project directories. Detects `~/go/pkg/mod` (global module cache) and `$GOCACHE` (build cache).  
**Candidates:** sub-directories older than `min_age_days` within these paths.  
**Description:** `"Go module cache"` or `"Go build cache"`

Note: Go detectors operate on global cache paths, not project directories. They are invoked once per scan, not per-directory.

---

### Homebrew

**Trigger:** not per-project. Detects `~/Library/Caches/Homebrew` (macOS) or `/home/linuxbrew/.linuxbrew/var/cache` (Linux).  
**Candidates:** the entire cache directory (treated as a single unit).  
**Description:** `"Homebrew download cache"`

---

## Symlink Handling

**Design decision binding (D-02):** see `docs/decisions.md` §D-02.

- **Size calculation:** follow symlinks to compute the size of the target.
- **`entry_type`:** report as `Symlink` (not `File` or `Dir`).
- **Deletion:** when a symlink is a cleanup candidate, delete the symlink itself (not the target). The safety module enforces this — `trash()` is called on the symlink path.
- **Cycle detection:** `jwalk` handles cycle detection internally. Do not follow symlinks that create cycles.

---

## Scan Depth Limits

**`dsg scan` (CWD):** walk up to 8 directory levels deep.  
**`dsg scan --deep`:** walk up to 20 directory levels deep.  
**`dsg caches`:** fixed paths only (no recursive descent beyond the cache roots).

These limits prevent runaway scans on pathological directory trees (e.g., recursive symlinks not caught by `jwalk`'s cycle detection).

---

## Error Handling During Scan

Scan errors on individual entries are **non-fatal**:
- Permission denied: log at `debug` level, skip entry.
- I/O error on stat: log at `warn` level, skip entry.
- Detector panic: catch at the detector boundary, log at `error` level, skip the directory, continue.

A scan that encounters any errors still exits with code `0` as long as at least one entry was successfully scanned. A scan where all entries fail exits with code `1`.
