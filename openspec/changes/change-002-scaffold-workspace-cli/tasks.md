# Tasks: change-002-scaffold-workspace-cli

- [x] Create `.gitignore` with `target/` entry
- [x] Create workspace `Cargo.toml` at dsg repo root
- [x] Create `dsg/Cargo.toml` with all required dependencies
- [x] Create `dsg/src/config.rs` — Config struct + Config::load()
- [x] Create `dsg/src/main.rs` — clap 4 skeleton with scan/clean/caches subcommands
- [x] Create `.github/workflows/ci.yml` — check/clippy/test/fmt
- [x] Run `cargo build --release` — must exit 0
- [x] Run `cargo test` — all tests must pass
- [x] Commit to dsg repo
