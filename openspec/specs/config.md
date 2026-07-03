# Capability Spec: Configuration Schema

**Spec ID:** dsg-spec-config  
**Phase:** Phase 1 (MVP)  
**Status:** approved  

---

## Overview

`dsg` reads a TOML configuration file on startup. All settings have sensible defaults; the file is entirely optional. If the config file is absent, `dsg` uses built-in defaults without error.

---

## File Location

**Default path:** `~/.config/dsg/config.toml`

**Resolution order:**
1. Path supplied via `--config <path>` flag
2. `$DSG_CONFIG` environment variable (if set)
3. `~/.config/dsg/config.toml` (default)

If none of the above paths resolve to an existing file, all defaults are used.

---

## Schema

```toml
# ~/.config/dsg/config.toml
#
# All fields are optional. Values shown are defaults.

# Paths to exclude from all scan and clean operations.
# Supports glob patterns. Evaluated before any other rule.
exclude_paths = []

# Minimum age in days before a cache entry is eligible for cleanup.
# Entries with mtime within this window are always skipped.
# Applies to both scan results and clean operations.
min_age_days = 1

# Minimum size in megabytes for a scan entry to be reported.
# Smaller entries are silently omitted from output to reduce noise.
min_size_mb = 10

# If true, all clean operations default to dry-run (preview only).
# Setting this to false does NOT auto-execute deletions; it changes
# the default so that `dsg clean` without --dry-run will prompt
# for confirmation rather than assume preview mode.
#
# Recommendation: leave this true. Override per-invocation with --force.
dry_run_default = true

# Logging verbosity. Valid values: "error", "warn", "info", "debug", "trace".
# Log output goes to stderr.
log_level = "info"
```

---

## Field Reference

### `exclude_paths`

**Type:** array of strings  
**Default:** `[]` (empty — no exclusions beyond built-in safety rules)  
**Description:** Glob patterns for paths that `dsg` will never scan or delete. Evaluated against absolute path. Patterns are matched using standard glob syntax (`*`, `**`, `?`).

**Examples:**
```toml
exclude_paths = [
    "/Users/alice/projects/important-project/**",
    "*/backups/**",
    "**/.cargo/bin",
]
```

**Built-in exclusions** (always active, not configurable here):
- `~/.cargo/bin/**`
- `~/.local/bin/**`
- `**/node_modules/.bin/**`
- All SIP-protected macOS paths (`/System/**`, `/usr/**`, etc.)

---

### `min_age_days`

**Type:** integer  
**Default:** `1` (24 hours)  
**Valid range:** `0` – `3650` (0 = no minimum; not recommended)  
**Description:** Entries whose `mtime` is newer than `min_age_days` ago are silently excluded from scan results and cleanup candidates. This is a hard floor — it cannot be overridden by `--force` (it can only be lowered to 0 via config, which is logged as a warning).

**Relationship to spec §6.1 Rule 1:** `min_age_days = 1` implements "never delete files younger than 24 hours."

---

### `min_size_mb`

**Type:** integer  
**Default:** `10` (10 MB)  
**Valid range:** `0` – no upper limit  
**Description:** Scan results and cleanup candidates smaller than this threshold are omitted from output. This reduces noise from tiny cache entries. The filtering applies to display only — the safety rules still apply to all entries regardless of size.

---

### `dry_run_default`

**Type:** boolean  
**Default:** `true`  
**Description:** Controls whether `dsg clean` (without explicit `--dry-run` or `--force`) defaults to preview mode.

| `dry_run_default` | Behavior of `dsg clean` |
|------------------|------------------------|
| `true` (default) | Preview mode; prints dry-run output, exits 2 |
| `false` | Interactive confirmation prompt before executing |

In either case, `--dry-run` and `--force` always override this setting.

---

### `log_level`

**Type:** string  
**Default:** `"info"`  
**Valid values:** `"error"`, `"warn"`, `"info"`, `"debug"`, `"trace"`  
**Description:** Controls log verbosity for messages emitted to stderr. `"info"` is the recommended production setting. `"debug"` and `"trace"` emit filesystem traversal details useful for troubleshooting.

---

## Config Loading Behavior

1. `dsg` reads the config file at startup (before parsing subcommand arguments).
2. If the file is absent: all defaults are used, no error is logged.
3. If the file exists but is malformed TOML: `dsg` exits with code `1` and prints the parse error to stderr.
4. If a field has an invalid value (e.g., `log_level = "verbose"`): `dsg` exits with code `1`.
5. Unknown fields in the config file are **silently ignored** (forward-compatibility).

---

## Directory Creation

`dsg` does **not** create `~/.config/dsg/` automatically. If the directory does not exist, the default is used. A future `dsg config --init` command will scaffold the config file; that is a Phase 2 feature.

---

## Environment Variable Overrides

The following environment variables override their config-file equivalents:

| Variable | Overrides | Notes |
|----------|-----------|-------|
| `DSG_CONFIG` | config file path | Must be an absolute path |
| `DSG_LOG_LEVEL` | `log_level` | Same valid values as the config field |
| `DSG_DRY_RUN` | `dry_run_default` | `"1"` or `"true"` = true; `"0"` or `"false"` = false |
