PLAN: dsg-cli-foundation
Project: Disk Space Guardian
Date: 2026-07-02
OpenSpec available: YES
Changes to implement: 5

CHANGE LIST (ordered)
1. change-001-establish-capability-specs: Decompose docs/README.md into initial OpenSpec capability specs (cli, config, safety, scanner) and bind the four open design decisions.
   - Scope: specs only (no code)
   - Depends on: NONE
   - Recommended agent: Claude Code
   - Est. complexity: M
   - Complexity score: High
   - Model class: frontier
   - Customer value: MEDIUM (indirect — makes every later change verifiable against a real spec baseline)
   - Details: Write openspec/specs/{cli,config,safety,scanner}.md distilled from docs/README.md §3/§6. Must BIND the four risks the assessment left open: (a) re-verify lsof immediately before each trash op (TOCTOU), (b) scanner/cleaner never follow symlinks, (c) trash failure on external/network volumes is a hard per-item error, never an rm fallback, (d) min-age rule anchors to mtime. Also records the two scope decisions below (TUI defer, minimal CI).

2. change-002-scaffold-workspace-cli: Create the Cargo workspace, dsg binary crate, clap command skeleton, and TOML config loading.
   - Scope: build system | cli | config
   - Depends on: change-001
   - Recommended agent: Codex
   - Est. complexity: M
   - Complexity score: Medium
   - Model class: medium
   - Customer value: MEDIUM
   - Details: Workspace with single dsg crate (structure anticipates a future MCP crate without creating it). Subcommands scan/clean/status parse and print stubs; clean defaults to dry-run at the CLI-contract level from day one. Config loads ~/.config/dsg/config.toml via serde+toml with documented defaults. MUST verify all crate versions against current stable (Base Rules §22/§23) — the docs pins are research-era; fs2 is expected to be replaced. Includes a minimal CI workflow (cargo check + clippy + test on macOS/Linux) — deliberate small addition beyond goals.md, justified because every later change's acceptance depends on these gates; full 3-OS matrix deferred.

3. change-003-safety-module: Implement the safety engine — the trust core of the product.
   - Scope: domain (safety) | tests
   - Depends on: change-002
   - Recommended agent: Claude Code
   - Est. complexity: L
   - Complexity score: High
   - Model class: frontier
   - Customer value: HIGH
   - Details: Dry-run-by-default plan/execute split, trash-not-rm via trash crate with hard-error path on trash failure, lsof + git-status activity verification with pre-trash re-check, exclusion lists (global + user TOML patterns), min-age (mtime) and protected-path guards. Acceptance gate: every BLOCKING rule in constraints.md has at least one test that fails if the rule is violated. No destructive-path code lands in any later change except through this module.

4. change-004-scanner-core: Parallel filesystem scanner with size/staleness reporting.
   - Scope: domain (scanner) | cli (scan)
   - Depends on: change-002
   - Recommended agent: Codex or Claude Code
   - Est. complexity: M
   - Complexity score: Medium
   - Model class: medium
   - Customer value: HIGH
   - Details: jwalk-based parallel walk (walkdir fallback), symlinks never followed (spec-bound, tested), size aggregation + mtime staleness, ecosystem-detector trait so change-005 plugs in, human and --json output for dsg scan. Read-only — deliberately does NOT depend on change-003 so it can run in parallel with it.

5. change-005-ecosystem-detectors-clean: Ecosystem detectors + wire dsg clean end-to-end through the safety module.
   - Scope: domain (detectors) | cli (clean) | integration tests
   - Depends on: change-003, change-004
   - Recommended agent: Claude Code
   - Est. complexity: L
   - Complexity score: High
   - Model class: frontier
   - Customer value: HIGH
   - Details: Detectors for Rust, Node, Python, Go, Docker, Xcode, Homebrew using conservative marker rules (Cargo.toml/package.json siblings; pnpm hardlink and Gradle-wrapper preservation). Wires dsg clean → scanner → safety engine: preview by default, --force to execute, everything through trash. Integration tests cover the full preview→confirm→trash path against fixture trees.

EXECUTION ROUND ORDER
Round 1: change-001
Round 2: change-002
Round 3 (parallel): change-003, change-004
Round 4: change-005

SCOPE DECISIONS AND TRADE-OFFS (explicit)
- TUI DEFERRED out of this phase, despite docs §4.1 listing it P0 for spec Phase 1. Rationale: TUI is presentation over the same engine; goals.md success criteria are met by the CLI surface; deferring keeps changes one-session-sized. Recorded here and in change-001 so the drop is a decision, not an omission — next phase should pick it up.
- Windows support DEFERRED: lsof/fuser verification is unix-only; this phase targets macOS/Linux. Windows activity-verification needs a different mechanism (RestartManager/handle) and is a later-phase change. The spec's cross-platform claim is NOT met by this phase.
- CI is minimal (2 OS, check/clippy/test), not the spec's full matrix.
- MCP server and knowledge wiki: out of scope entirely (docs Phases 2–3); no work here.
- Risk accepted: docker/xcode detectors depend on external tool state (docker daemon, xcrun) — change-005 must degrade gracefully when absent, which the spec does not currently describe; change-001 must add this.

COMMANDS TO RUN
/opsx:new change-001-establish-capability-specs
/opsx:new change-002-scaffold-workspace-cli
/opsx:new change-003-safety-module
/opsx:new change-004-scanner-core
/opsx:new change-005-ecosystem-detectors-clean

PLAN COMPLETE

---
Sycophancy review: detect_sycophancy score 0.0 (standard strictness, no patterns) — saved to sycophancy/plan-2026-07-02T17-33-06Z.json.
