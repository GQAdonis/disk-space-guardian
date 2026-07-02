# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Current State: Spec-Only

This repository currently contains **only documentation** — there is no code, `Cargo.toml`, or build system yet. The full design lives in [docs/README.md](docs/README.md) (research analysis, architecture, and 16-week project plan). Do not fabricate build/test commands or claim features exist; scaffold from the spec when asked to implement.

When implementation begins, this becomes a **Rust workspace** (the `dsg` CLI, plus an optional MCP server crate). Standard toolchain once scaffolded: `cargo build --release`, `cargo test`, `cargo test <name>` for a single test, `cargo clippy --all-targets`, `cargo fmt`.

## What This Project Is

**Disk Space Guardian** — intelligent, safety-first disk management for developer workstations. Three coordinated deliverables (build in this order):

1. **Rust CLI (`dsg`)** — the execution engine. Fast parallel scanning, ecosystem cache detection, safe cleanup. Single binary, <10MB, cross-platform (macOS/Linux/Windows).
2. **AgentSkills.io `SKILL.md`** — a portable skill that teaches any AI assistant *how* to reason about cleanup, wrapping the CLI. Uses progressive disclosure (L1 metadata → L2 body → L3 `references/`).
3. **MCP server (optional, last)** — a thin `rmcp` wrapper exposing the CLI core as MCP tools.

**Architectural decision that governs sequencing: CLI + Skill first, MCP second.** MCP carries 1.3×–80× the token overhead of a CLI for local, I/O-heavy work, so it is added as a protocol layer only after the CLI core is solid. Do not lead with MCP.

## Non-Negotiable Safety Model

This is the heart of the product — the entire value proposition is *trustworthy* deletion. These are hardcoded rules, overridable only via an explicit `--unsafe`/`--force`/`--permanent` flag as noted:

- **Dry-run is the default.** Every destructive path previews first; execution requires `--force` or explicit confirmation.
- **Trash, never `rm`.** Move to Trash/Recycle Bin (via the `trash` crate) so every deletion is recoverable. `--permanent` is the only escape.
- **Verify activity before deleting:** `lsof`/`fuser` for open files, git status for uncommitted work. Never delete files held open or dirs with uncommitted changes.
- **Never delete:** files younger than the min-age threshold (default 24h), installed-binary dirs (`~/.cargo/bin`, `~/.local/bin`), SIP/system paths, or the home dir itself.
- **Conservative detection:** only treat a directory as a cache if it has a known marker (`Cargo.toml`, `package.json`, etc.). Preserve ecosystem-specific special cases (pnpm hardlinks, Gradle wrapper).

The layered safety pipeline (dry-run → activity verification → exclusion lists → trash → retention policy → audit log) is specified in [docs/README.md](docs/README.md) §3.3 and §6. Read it before touching any deletion logic.

## Core Design Concepts

- **Ecosystem-aware:** detects and cleans Rust, Node, Python, Go, Docker, Xcode, Homebrew, and ML/AI caches, each with its own retention matrix (§6.3). One tool spanning all ecosystems is the key differentiator vs. existing tools.
- **Context-aware retention:** beyond age/size, the Skill layer uses git status, process activity, and the AI's knowledge of active projects to reason about what's safe. A local **knowledge wiki** (`~/.config/dsg/wiki.md`, Karpathy-LLM-wiki pattern) persists user preferences, project importance scores, and cleanup history so decisions improve over time.
- **Pressure-aware automation:** threshold escalation (70% warn → 85% clean caches → 95% aggressive), wired to cron / systemd timers / launchd. Not just scheduled — reactive.
- **Config:** TOML at `~/.config/dsg/config.toml`. Async runtime is Tokio (required by `rmcp`). Scanning uses `jwalk` (fast) with `walkdir` as the safe-verification fallback.

Planned CLI surface (from spec — not yet built): `dsg scan|clean|caches|watch|schedule|config|status`, all defaulting to safe/preview behavior.

## Working Rules (Prometheus Base Rules Set)

[docs/Prometheus Base Rules Set.md](docs/Prometheus%20Base%20Rules%20Set.md) is the canonical agent ruleset and applies to all work here. The full set matters, but for *this* project the load-bearing rules are:

- **Simplicity & surgical changes (§2, §3):** minimum code that solves the problem; touch only what's necessary; match existing conventions.
- **Minimize irreversible actions (§8):** confirm intent, prefer reversible approaches, create rollback paths — this is the product's own thesis applied to your own edits.
- **Feature-based CLEAN architecture + strict layering (§15–§21):** organize by capability (Scanner, Safety, Knowledge, Automation modules per §3.1), not technical layers. If/when a UI exists, enforce UI → Hook → Store → Service → external; UI never calls services/APIs directly.
- **Verify dependency versions (§22, §23):** the crate versions in the spec are targets — check current stable releases and breaking changes before adding any dependency. Never assume training-era versions.
- **No hidden state, human override always (§13, §25):** disk decisions must be inspectable, auditable, and overridable. Every cleanup logs its rationale.
- **Strong typing, tests as completion (§29, §30):** no untyped domain models; work isn't done until type-checks, lints, and tests pass (or you state why they can't run).

Repo-level guidance (this file) may add stricter requirements but overrides the base rules only when explicit and non-contradictory with safety and user intent (§26).
