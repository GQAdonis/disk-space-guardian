ASSESSMENT: dsg-cli-foundation
Project: Disk Space Guardian
Date: 2026-07-02
Codebase baseline: Spec-only greenfield — documentation and tooling config exist (docs/, CLAUDE.md/AGENTS.md, OpenSpec adapters, KBD state); zero source code, no Cargo workspace, no tests, no CI.
Cross-tool progress: none — progress.json changes map is empty; no other tool has recorded work.

IMPLEMENTATION STATUS
- Rust workspace scaffold (Cargo.toml, clap CLI skeleton): MISSING — no Cargo.toml, no src/ anywhere in the repo.
- TOML config loading (~/.config/dsg/config.toml): MISSING — no config module; schema exists only as prose in docs/README.md §3.2.2.
- Safety module (dry-run default, trash-not-rm, lsof/git verification, exclusion lists): MISSING — specified in docs §3.3/§6; no implementation.
- Parallel filesystem scanner + multi-ecosystem detection: MISSING — specified in docs §2.3/§3.1; no implementation.
- TUI mode (ratatui): MISSING — listed P0 for spec Phase 1 (docs §4.1) but NOT included in this phase's goals.md. Scope discrepancy — see gaps.
- OpenSpec capability specs: MISSING — openspec/specs/ is empty; the entire design lives in the docs/README.md monolith and has not been decomposed into canonical specs.

CROSS-TOOL PROGRESS
NONE — no cross-tool activity recorded.

SPEC GAP SUMMARY
- Empty openspec/specs/: AGENTS.md mandates change proposals before coding, but there is no spec baseline to write deltas against. First change should establish the initial capability specs (cli, safety, scanner, config) or the workflow will run on an implicit spec.
- Unrunnable stack commands: project.json build/test/lint commands are declared targets; none can execute until the workspace is scaffolded.
- Unverified dependency versions: crate versions in docs §3.2.2 (ratatui 0.30, sysinfo 0.30, trash 5, fs2 0.4, jwalk 0.8, etc.) are research-era pins. Base Rules §22/§23 require verification at scaffold time. fs2 in particular is a stale crate and may need replacement (e.g., sysinfo disk queries or rustix).
- TUI scope ambiguity: docs §4.1 marks TUI as P0 for Phase 1, but goals.md success criteria only require scan + dry-run clean CLI. Needs an explicit include/defer decision at plan time — deferring is defensible (TUI is presentation over the same engine) but must be recorded, not silently dropped.
- No CI: docs §8.3 assumes CI on macOS/Linux/Windows; nothing exists. Decide at plan time whether CI lands in this phase or the next.

BUILD HEALTH
- build check: UNKNOWN — `cargo check --workspace --all-targets` not runnable (no Cargo.toml).
- known violations: NONE observable (no code).
- test coverage: NONE — no tests exist.

CONSTRAINT CHECK
- AGENTS.md violations: NONE — vacuously; there is no code to violate them. This says nothing about future compliance.
- constraints.md violations: NONE — same vacuous caveat. The six BLOCKING safety rules only become checkable once the safety module exists; the plan should make safety-module tests the first acceptance gate so BLOCKING rules are enforced by tests from the first commit of destructive-path code.

DESIGN RISKS TO RESOLVE AT PLAN TIME
- TOCTOU race in activity verification: lsof check → trash operation has a time-of-check/time-of-use window the spec does not address. Mitigation options: re-verify immediately before each trash call, or accept and document residual risk.
- Symlink traversal: jwalk following symlinks could carry the scanner (and later the cleaner) outside the target scope — a direct safety hazard. Default must be no-follow; needs a test.
- Trash semantics on external/network volumes: the trash crate behaves differently across volumes (macOS .Trashes, failures on some mounts). Spec assumes trash always works; error path needs definition and tests.
- Timestamp ambiguity: the 24h min-age rule does not bind to mtime vs ctime vs atime. docs §2.4 discusses trade-offs (relatime caveats) but the safety rule needs one authoritative choice — mtime is the defensible default per the spec's own analysis.

GOAL PROGRESS
- Scaffold dsg Rust workspace (clap CLI + TOML config): NOT MET — nothing exists.
- Safety module first (dry-run, trash, lsof/git, exclusions): NOT MET — nothing exists.
- Parallel scanner + multi-ecosystem detection: NOT MET — nothing exists.

ASSESSMENT COMPLETE

---
Sycophancy review: detect_sycophancy score 0.0 (standard strictness, no patterns) — saved to sycophancy/assess-2026-07-02T17-29-04Z.json.
