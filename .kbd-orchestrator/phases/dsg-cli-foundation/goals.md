# Goals

Phase: **dsg-cli-foundation** — Phase 1 (Foundation) from [docs/README.md](../../../docs/README.md) §7.

- Scaffold the `dsg` Rust workspace: clap CLI skeleton and TOML config loading (`~/.config/dsg/config.toml`).
- Implement the safety module **first**: dry-run default, trash-not-`rm` (via the `trash` crate), `lsof`/git activity verification, and exclusion lists.
- Implement a parallel filesystem scanner (`jwalk`, `walkdir` fallback) with multi-ecosystem cache detection (Rust, Node, Python, Go, Docker, Xcode, Homebrew).

## Success criteria

- `dsg scan` runs and reports reclaimable space for detected ecosystems in the target directory.
- `dsg clean` defaults to dry-run; destructive execution requires `--force`; deletions move to Trash, never `rm`.
- Safety guarantees in [.kbd-orchestrator/constraints.md](../../constraints.md) (BLOCKING rules) are enforced in code and covered by tests.
- `cargo check`, `cargo clippy`, and `cargo test` pass on the new workspace.
