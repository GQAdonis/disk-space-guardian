# Capability Spec: Safety Rules

**Spec ID:** dsg-spec-safety  
**Phase:** Phase 1 (MVP)  
**Status:** approved  

---

## Overview

The safety model is the core value proposition of Disk Space Guardian. Every deletion path goes through a pipeline of layered guards. Rules are ordered from cheapest-to-check to most-expensive, and failing any rule skips the entry with a warning — it never aborts the entire operation.

**The golden rule: when in doubt, skip and warn, never abort and never delete.**

---

## Safety Rules (ordered: checked in sequence)

### RULE-01: Dry-run is default

**Level:** Behavioral default  
**Overridable:** Yes — `--force` flag or explicit TUI confirmation

All deletion operations default to preview mode. The filesystem is never modified unless the user explicitly passes `--force` or confirms in the TUI.

**Implementation constraint:** The `--force` flag must appear explicitly in the command. `dry_run_default = false` in config changes the interactive TUI behavior, but non-interactive invocations (piped, CI) still require `--force`.

---

### RULE-02: Trash, never `rm`

**Level:** Hardcoded  
**Overridable:** No (Phase 1). `--permanent` flag is reserved for Phase 3.

All deletions are performed via the `trash` crate, which moves entries to the OS trash:
- **macOS:** moves to `~/.Trash/` via macOS Finder API
- **Linux:** moves to `~/.local/share/Trash/` per XDG spec
- **Windows:** moves to Recycle Bin via Shell API

`std::fs::remove_file`, `std::fs::remove_dir_all`, and any equivalent are **forbidden** in the deletion path. Clippy lint or a custom lint should flag any direct `fs::remove_*` call in the cleanup module.

---

### RULE-03: Activity verification — open file check

**Level:** Hardcoded  
**Overridable:** No (Phase 1)

Before deleting any entry, run `lsof +D <path>` with a **5-second timeout**.

**Algorithm:**
1. Spawn `lsof +D <absolute_path>` as a child process.
2. Set a 5-second wall-clock timeout. If the process does not return within 5s, kill it.
3. If `lsof` returns any output (any open file handles found): **skip the entry**, log warning.
4. If `lsof` returns empty output (exit 0, no lines): proceed.
5. If `lsof` is not found on PATH: log a one-time warning at `warn` level, then **skip this check for all entries** (do not fail the run). This handles minimal environments.

**Warning format:**
```
[SKIP] /path/to/target  — open file handles detected (lsof); skipping
```

**Design decision binding (D-01):** See `docs/decisions.md` §D-01 for TOCTOU handling.

---

### RULE-04: Activity verification — git status check

**Level:** Hardcoded  
**Overridable:** Yes — `--force` flag adds a confirmation step but does not skip the check

If the directory to be deleted (or any ancestor up to the project root) contains a `.git/` directory:
1. Run `git -C <project_root> status --porcelain` with a **10-second timeout**.
2. If output is non-empty (uncommitted or untracked files exist): **skip the entry**, log warning.
3. If `git` is not on PATH: skip this check for all entries, log a one-time warning.

**Project root resolution:** walk up from the entry's path until `.git/` is found or the filesystem root is reached.

**Warning format:**
```
[SKIP] /path/to/target  — uncommitted git changes in parent project; skipping
```

---

### RULE-05: Minimum age guard

**Level:** Configurable (hardcoded minimum of 0)  
**Default:** `min_age_days = 1` (24 hours)

Any entry whose `mtime` (last modification time) is within `min_age_days * 86400` seconds of the current time is **always skipped**.

**mtime rationale:** see `docs/decisions.md` §D-04 (mtime vs atime).

**Skipped silently** (no warning emitted) unless `--log-level debug` is set.

---

### RULE-06: Exclusion list

**Level:** Configurable + hardcoded built-ins  
**Default:** built-in list applies; user list is empty

Check the entry's absolute path against all patterns in:
1. Built-in exclusion list (hardcoded, see below)
2. `exclude_paths` from `~/.config/dsg/config.toml`

If any pattern matches: **skip the entry, no warning** (exclusions are expected, not notable).

**Built-in exclusion patterns (cannot be removed):**

```
~/.cargo/bin/**
~/.local/bin/**
~/.npm/lib/node_modules/**
**/node_modules/.bin/**
/System/**
/usr/**
/bin/**
/sbin/**
/private/var/db/**
/private/etc/**
/Library/Apple/**
```

**Symlink handling (D-02 binding):** exclusion list is checked against the symlink path, not the target. See `docs/decisions.md` §D-02.

---

### RULE-07: Never delete dsg itself

**Level:** Hardcoded  
**Overridable:** No

The binary path of the running `dsg` process and its config directory (`~/.config/dsg/`) are always excluded, regardless of any other rule or flag.

**Implementation:** Resolved at startup via `std::env::current_exe()`. Add the result and `~/.config/dsg/**` to the built-in exclusion list before any scan begins.

---

## Trash Failure Semantics

**Design decision binding (D-03):** If `trash::trash()` returns an error for a single entry, log the error and continue the batch. Do not abort the entire cleanup run.

**Error format:**
```
[ERROR] Failed to trash /path/to/item: <error message>. Skipping.
```

The final summary line counts items skipped due to trash failures separately:

```
Done: 15 items moved to Trash (8.2 GB). 1 item skipped (trash error). 2 items skipped (safety rules).
```

---

## Summary: Safety Pipeline

Each candidate entry passes through these checks in order. Failure at any step means the entry is skipped (with or without a warning, as noted).

```
1. RULE-06: Exclusion list check          (skip silently on match)
2. RULE-07: dsg self-check               (skip silently)
3. RULE-05: Minimum age guard            (skip silently unless debug)
4. RULE-04: Git status check             (skip + warn on uncommitted changes)
5. RULE-03: lsof open-file check         (skip + warn on open handles)
6. RULE-01: Dry-run gate                 (print only, unless --force)
7. RULE-02: Trash deletion               (move to Trash; log error + continue on failure)
```

---

## What Is NOT a Safety Rule

The following are ecosystem-specific behaviors, not safety rules:
- Detecting whether a `node_modules/` was installed with pnpm hardlinks
- Detecting whether a Gradle wrapper should be preserved
- Docker image reference counting

These are handled in the ecosystem detectors (`scanner.md`), not the safety module.

---

## Audit Log

Every deletion (successful or skipped) is appended to `~/.config/dsg/audit.log` in JSONL format:

```json
{"ts": "2026-07-03T10:00:00Z", "action": "trash", "path": "/Users/alice/projects/old/target", "size_bytes": 1073741824, "ecosystem": "rust"}
{"ts": "2026-07-03T10:00:01Z", "action": "skip", "reason": "git_uncommitted", "path": "/Users/alice/projects/active/target"}
```

The audit log is Phase 1 but is append-only; no log rotation is implemented until Phase 2.
