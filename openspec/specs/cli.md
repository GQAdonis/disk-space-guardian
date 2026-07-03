# Capability Spec: CLI Command Surface

**Spec ID:** dsg-spec-cli  
**Phase:** Phase 1 (MVP)  
**Status:** approved  
**Binary name:** `dsg`

---

## Overview

The `dsg` CLI is the primary execution engine for Disk Space Guardian. All destructive operations default to dry-run (preview-only). Execution requires either `--force` or user confirmation in TUI mode.

---

## Phase 1 Commands

### `dsg scan`

Scan the current working directory for reclaimable space. Reports detected caches, their sizes, and staleness.

```
dsg scan
dsg scan --deep
dsg scan --ecosystem <name>
dsg scan --stale <duration>
dsg scan --json
```

**Flags:**

| Flag | Type | Description |
|------|------|-------------|
| `--deep` | bool | Scan entire system (home dir + common cache paths) instead of CWD only |
| `--ecosystem <name>` | string | Limit scan to one ecosystem: `rust`, `node`, `python`, `go`, `docker`, `xcode`, `homebrew` |
| `--stale <duration>` | string | Report only entries not modified in `<duration>`. Accepts `7d`, `30d`, `90d`. Uses mtime. |
| `--json` | bool | Emit machine-readable JSON to stdout instead of human table |

**Human-readable output format (default):**

```
Scan Results — /Users/user/projects
─────────────────────────────────────────────────────────────
  Path                              Size     Age     Ecosystem
  ─────────────────────────────────────────────────────────
  ./my-app/target                   2.4 GB   12d     rust
  ./my-app/node_modules             340 MB   5d      node
  ./old-proj/target                 1.1 GB   87d     rust
─────────────────────────────────────────────────────────────
  Total reclaimable:                3.8 GB   (3 entries)

Run `dsg clean` to preview cleanup interactively.
```

**JSON output format (`--json`):**

```json
{
  "scan_root": "/Users/user/projects",
  "total_bytes": 4077715456,
  "entries": [
    {
      "path": "/Users/user/projects/my-app/target",
      "size_bytes": 2578849792,
      "last_modified_secs": 1751500000,
      "age_days": 12,
      "ecosystem": "rust",
      "entry_type": "Dir"
    }
  ]
}
```

---

### `dsg clean`

Interactive cleanup. Defaults to dry-run: shows what would be deleted, does not delete. Requires `--force` to execute deletions.

```
dsg clean
dsg clean --dry-run
dsg clean --force
dsg clean --target <path>
dsg clean --ecosystem <name>
```

**Flags:**

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `--dry-run` | bool | `true` | Preview only; print what would be deleted without touching the filesystem |
| `--force` | bool | `false` | Execute deletions (moves to Trash via `trash` crate). Implies preview printed first; user must confirm. |
| `--target <path>` | path | CWD | Restrict cleanup to this directory subtree |
| `--ecosystem <name>` | string | all | Clean only entries matching this ecosystem |

**Dry-run output format:**

```
Dry-run Preview — nothing will be deleted
─────────────────────────────────────────────────────────────
  [WOULD DELETE] ./my-app/target           2.4 GB   rust
  [WOULD DELETE] ./old-proj/target         1.1 GB   rust
─────────────────────────────────────────────────────────────
  Total: 2 items, 3.5 GB would be moved to Trash.

To execute: dsg clean --force
```

**Force output format (after confirmation prompt):**

```
Executing cleanup — items moved to Trash
─────────────────────────────────────────────────────────────
  [TRASHED]  ./my-app/target              2.4 GB
  [SKIPPED]  ./active-proj/target         (uncommitted changes — git status)
─────────────────────────────────────────────────────────────
  Done: 1 item moved to Trash (2.4 GB recovered). 1 item skipped.
```

---

### `dsg caches`

Manage global developer ecosystem caches (locations shared across projects, e.g., `~/.cargo/registry`, `~/.npm`, `~/.cache/pip`).

```
dsg caches
dsg caches --list
dsg caches --clean <ecosystem>
```

**Flags:**

| Flag | Type | Description |
|------|------|-------------|
| `--list` | bool | List all detected global caches with their sizes |
| `--clean <ecosystem>` | string | Clean the global cache for the named ecosystem |

**`--list` output:**

```
Global Developer Caches
─────────────────────────────────────────────────────────────
  Ecosystem   Location                    Size
  ─────────────────────────────────────────────────────────
  rust        ~/.cargo/registry           1.8 GB
  rust        ~/.cargo/git                240 MB
  node        ~/.npm                      580 MB
  node        ~/.pnpm-store               2.1 GB
  python      ~/.cache/pip                420 MB
  python      ~/.cache/uv                 310 MB
─────────────────────────────────────────────────────────────
  Total:                                  5.5 GB

Run `dsg caches --clean <ecosystem>` to preview cleanup.
```

---

### `dsg --version`

Print the binary version and exit.

```
dsg --version
dsg 0.1.0
```

---

### `dsg --help`

Print top-level help and exit.

---

## Exit Codes

| Code | Meaning |
|------|---------|
| `0` | Success — operation completed normally |
| `1` | Error — scan failed, I/O error, or config parse error; error message on stderr |
| `2` | Dry-run completed — printed what would be deleted; no filesystem changes made |

**Exit code 2 semantics for `--dry-run`:**

```
$ dsg clean --dry-run; echo $?
[Dry-run output]
2
```

This allows scripts to distinguish "nothing to do" from "would have done something".

If `--dry-run` reports zero candidates, exit code is `0` (nothing to report is a success).

---

## Global Flags

These flags are accepted by all subcommands:

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `--config <path>` | path | `~/.config/dsg/config.toml` | Override config file location |
| `--log-level <level>` | string | `info` | Logging verbosity: `error`, `warn`, `info`, `debug`, `trace` |
| `--no-color` | bool | `false` | Disable ANSI color codes in output |
| `--quiet` | bool | `false` | Suppress informational output; only print results and errors |

---

## Design Notes

- All output goes to **stdout**; log messages and warnings go to **stderr**. This ensures `--json` piping works correctly.
- The `--force` flag is the only way to execute deletions from a non-interactive shell. TUI mode (launched by `dsg clean` without `--force` in an interactive terminal) allows per-item selection.
- The binary name is `dsg` — short, memorable, no collision with common system utilities.
