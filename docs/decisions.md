# Design Decisions — disk-space-guardian

This file records binding architectural decisions for the `dsg` project. Each entry includes the decision, its rationale, and any consequences for implementation. Once recorded here, a decision is binding for Phase 1 and may only be changed via an explicit openspec change that references this file.

---

## D-01: lsof TOCTOU Handling

**Status:** decided  
**Bound in:** `openspec/specs/safety.md` §RULE-03

**Decision:** Take a snapshot via `lsof +D <path>` at the start of the delete operation (after the user confirms, before the first `trash()` call). If a race is detected — i.e., a file is deleted between the scan step and the lsof check — log a warning and proceed with the deletion of remaining candidates.

**Rationale:** Eliminating TOCTOU completely requires OS-level file locking (e.g., `flock`, `F_OFD_SETLK`) and coordination with all other processes that might open the files — not portable, not practical for a dev tool cache cleaner. A snapshot-based approach with a warning is safe enough: the worst outcome is that `dsg` tries to trash a file that was already deleted by another process, which is an idempotent no-op (the `trash` crate returns an error for missing files, which is logged and counted as "already gone").

**Consequences:**
- The lsof check runs once per candidate path, not continuously.
- A 5-second timeout is applied to the `lsof` spawn to prevent hangs on large directories.
- If lsof reports open handles, the entry is skipped. If the lsof process times out, the entry is also skipped (conservative default).

---

## D-02: Symlink Handling

**Status:** decided  
**Bound in:** `openspec/specs/scanner.md` §Symlink Handling

**Decision:** Follow symlinks for size calculation (report the target's size). When a symlink itself is a cleanup candidate, delete the symlink (not the target).

**Rationale:** `node_modules/` in pnpm-managed projects may contain symlinks to shared packages in the pnpm global store. Deleting the target would affect other projects that share that store entry. Deleting the symlink removes the project's link to the shared resource without corrupting the store. Size reporting follows the target because the symlink's on-disk cost reflects the real space consumed by the target (when not shared).

**Consequences:**
- `entry_type = Symlink` in `ScanResult` indicates a symlink candidate.
- `trash()` is called on the symlink path (e.g., `node_modules/.bin/webpack → /path/to/store`), not the resolved target.
- Cycle detection is delegated to `jwalk`.
- For size computation: if multiple symlinks point to the same target, the target's size is counted once per symlink (potential overcount). This is acceptable for Phase 1 — exact deduplication requires inode tracking and is Phase 2.

---

## D-03: Trash Failure Semantics

**Status:** decided  
**Bound in:** `openspec/specs/safety.md` §Trash Failure Semantics

**Decision:** If `trash::trash()` returns an error for a single entry in a batch, log the error and continue with the remaining entries. Do not abort the cleanup run.

**Rationale:** Batch operations should maximize utility. A single untrashable file — for example, a file with macOS immutable flag set (`chflags uchg`), a cross-filesystem move failure, or a network-mounted path — should not prevent cleaning hundreds of other valid entries. The user is informed of the failure in the summary line and can investigate manually.

**Consequences:**
- The summary output distinguishes "moved to Trash" from "skipped (trash error)".
- The audit log records `action: "trash_failed"` with the error message.
- The exit code is still `0` if at least one entry was successfully trashed. A run where all trash attempts fail exits with `1`.
- If `trash()` is unavailable (e.g., headless Linux server with no XDG trash): the entire run is aborted at startup with a clear error message (`trash: no XDG_DATA_HOME / no trash directory available`). This is different from per-item failure.

---

## D-04: mtime vs atime Anchoring

**Status:** decided  
**Bound in:** `openspec/specs/safety.md` §RULE-05, `openspec/specs/scanner.md` §Core Data Types

**Decision:** Use `mtime` (last modification time) as the sole timestamp for staleness determination (`min_age_days` guard and scan output `Age` column).

**Rationale:** `atime` (last access time) is unreliable in modern filesystem configurations:
- Linux filesystems are commonly mounted with `noatime` or `relatime` options (the default on most distributions since 2009) to reduce I/O. Under `relatime`, `atime` is only updated if it is older than `mtime` or `ctime`, or if the last update was more than 24 hours ago. This makes `atime` meaningless for precise staleness measurement.
- macOS APFS uses `noatime` semantics by default for SSD volumes.
- Container environments and CI runners frequently use `noatime`.

`mtime` is always updated on content writes and is a reliable indicator of "last time this file/directory changed." For cache directories, the last write = last build/install, which is the signal we care about.

**Consequences:**
- `ScanResult.last_modified` is populated from `fs::metadata().modified()` (which maps to `st_mtime`).
- `ctime` (metadata change time) is not used in Phase 1.
- `atime` is not read, even if available.
- Users who last *read* a cache (e.g., a build that read but did not write an artifact) will not see the age reset. This is intentional — a read-only hit does not make the cache "fresh."
