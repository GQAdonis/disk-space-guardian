# Tasks: change-001-establish-capability-specs

- [x] Create `openspec/specs/` directory
- [x] Write `openspec/specs/cli.md` — all Phase 1 commands, flags, output formats, exit codes
- [x] Write `openspec/specs/config.md` — TOML schema, field reference, env var overrides, loading behavior
- [x] Write `openspec/specs/safety.md` — 7 safety rules, safety pipeline order, trash failure semantics, audit log format
- [x] Write `openspec/specs/scanner.md` — ScanResult type, EcosystemDetector trait, output formats, performance target, symlink handling
- [x] Write `docs/decisions.md` — bind D-01 (lsof TOCTOU), D-02 (symlink handling), D-03 (trash failure), D-04 (mtime anchoring)
- [x] Verify each spec cross-references the design decisions it binds
- [x] Commit all new files with message referencing this change
