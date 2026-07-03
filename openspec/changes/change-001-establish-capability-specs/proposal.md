---
id: change-001-establish-capability-specs
title: Establish Phase 1 capability specs + bind 4 open design decisions
phase: dsg-cli-foundation
priority: P0
effort: S
wave: 1
agent: general-purpose
status: done
gap_id: dsg-pre-cond
verdict: BUILD
scope:
  - /Users/gqadonis/Projects/prometheus/disk-space-guardian/openspec/specs/
  - /Users/gqadonis/Projects/prometheus/disk-space-guardian/docs/decisions.md
---

# change-001 — Establish Phase 1 Capability Specs + Bind Design Decisions

## Context

The `disk-space-guardian` repository is spec-only (no code). Before any implementation change can proceed, the CLI command surface, config schema, safety rules, and scanner algorithm must be formally specified. Additionally, four open design decisions (lsof TOCTOU, symlink handling, trash failure semantics, mtime anchoring) were identified in `docs/README.md` but not yet bound.

This change creates the `openspec/specs/` directory and populates it with four capability specs, then records the four design decisions in `docs/decisions.md`. All subsequent implementation changes (change-002 through change-005) depend on this change being complete.

## Scope

1. `openspec/specs/cli.md` — Command surface and UX contract for all Phase 1 commands
2. `openspec/specs/config.md` — TOML config schema with defaults and field reference
3. `openspec/specs/safety.md` — 7 safety rules, safety pipeline, audit log format
4. `openspec/specs/scanner.md` — `ScanResult` type, `EcosystemDetector` trait, output formats, performance target
5. `docs/decisions.md` — Binding records for D-01 through D-04

## Why These Specs First

The four specs encode every contract that implementation changes will be verified against:
- `cli.md`: defines exit codes, flag semantics, output formats — tested by integration tests
- `config.md`: defines TOML schema — tested by config parsing unit tests
- `safety.md`: defines the deletion pipeline — tested by safety module unit tests
- `scanner.md`: defines data types and trait interface — tested by scanner unit tests

Without these, each implementation change would need to make ad-hoc decisions about interfaces, creating drift across the codebase.

## Verification

- `openspec/specs/cli.md` exists and covers all commands listed in `docs/README.md` §3.2.2
- `openspec/specs/config.md` exists and matches the TOML structure in `docs/README.md` §3.2.2
- `openspec/specs/safety.md` exists and enumerates 7 rules including all rules from `docs/README.md` §6.1
- `openspec/specs/scanner.md` exists and defines `ScanResult`, `EntryType`, `Ecosystem`, `EcosystemDetector`
- `docs/decisions.md` exists and contains entries for D-01, D-02, D-03, D-04
- `docs/decisions.md` cross-references the spec files that bind each decision
