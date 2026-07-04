# Tasks: change-002-scaffold-workspace-cli

- [ ] Create `.gitignore` with `target/` entry
- [ ] Create workspace `Cargo.toml` at dsg repo root
- [ ] Create `dsg/Cargo.toml` with all required dependencies
- [ ] Create `dsg/src/config.rs` — Config struct + Config::load()
- [ ] Create `dsg/src/main.rs` — clap 4 skeleton with scan/clean/caches subcommands
- [ ] Create `.github/workflows/ci.yml` — check/clippy/test/fmt
- [ ] Run `cargo build --release` — must exit 0
- [ ] Run `cargo test` — all tests must pass
- [ ] Commit to dsg repo
