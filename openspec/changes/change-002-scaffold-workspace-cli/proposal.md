---
id: change-002-scaffold-workspace-cli
title: Scaffold Cargo workspace + CLI skeleton
phase: dsg-cli-foundation
priority: P0
effort: M
wave: 1
agent: general-purpose
status: in_progress
gap_id: dsg-002
verdict: BUILD
scope:
  - /Users/gqadonis/Projects/prometheus/disk-space-guardian (dsg repo)
  - Cargo.toml (workspace root — NEW)
  - dsg/Cargo.toml (binary crate — NEW)
  - dsg/src/main.rs (clap 4 skeleton — NEW)
  - dsg/src/config.rs (TOML config loader — NEW)
  - .github/workflows/ci.yml (GitHub Actions CI — NEW)
---

# change-dsg-002 — Scaffold Cargo workspace + CLI skeleton

## Context

The dsg project has zero Rust code. This change creates the foundational
Cargo workspace, binary crate, and clap 4 CLI skeleton so all subsequent
changes have a build target and can add incremental implementations.

## Scope

1. `Cargo.toml` (workspace root) — single-member workspace `["dsg"]`
2. `dsg/Cargo.toml` — dependencies: `clap@4`, `anyhow@1`, `toml@0.8`, `serde@1`, `tracing@0.1`, `tracing-subscriber@0.3`, `dirs@5`
3. `dsg/src/main.rs` — clap skeleton with `scan`, `clean`, `caches` subcommands; each prints a `[stub]` message and exits 0
4. `dsg/src/config.rs` — `Config` struct with `#[derive(Deserialize, Default)]`; `Config::load(path: Option<&Path>)` reads `~/.config/dsg/config.toml` or override path
5. `.github/workflows/ci.yml` — `cargo check`, `cargo clippy -- -D warnings`, `cargo test`, `cargo fmt --check`
6. `.gitignore` — add `target/` entry

## Config schema (from openspec/specs/config.md)

```toml
exclude_paths = []
min_age_days = 1
min_size_mb = 10
dry_run_default = true
log_level = "info"
```

## Command surface (from openspec/specs/cli.md)

All subcommands are stubbed in this change. Full implementations land in changes 003–005.

- `dsg scan [--deep] [--ecosystem <name>] [--stale <duration>] [--json]` → [stub]
- `dsg clean [--dry-run] [--force] [--target <path>] [--ecosystem <name>]` → [stub]
- `dsg caches [--list] [--clean <ecosystem>]` → [stub]

Global flags (accepted by all subcommands):
- `--config <path>` — override config file location
- `--log-level <level>` — logging verbosity
- `--no-color` — disable ANSI color codes
- `--quiet` — suppress informational output

## Verification

- `cargo build --release` exits 0
- `./target/release/dsg --help` shows usage
- `./target/release/dsg scan --help` shows scan flags
- `./target/release/dsg --version` prints version
- `cargo test` passes (basic smoke tests)
