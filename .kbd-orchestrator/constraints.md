# KBD Constraints — Disk Space Guardian

Project-specific blocking and warning rules for all KBD phases. Derived from the
"Non-Negotiable Safety Model" in [AGENTS.md](../AGENTS.md) / [CLAUDE.md](../CLAUDE.md)
and the [Prometheus Base Rules Set](../docs/Prometheus%20Base%20Rules%20Set.md).

## BLOCKING (must not happen — halt the phase)

These are the product's core safety guarantees and the base-rules safety floor. A change
that violates any of these is rejected regardless of goal progress.

1. **No permanent deletion by default.** All destructive paths move to Trash/Recycle Bin
   (via the `trash` crate), never `rm`. Permanent deletion only behind an explicit
   `--permanent` flag.
2. **No deletion without dry-run/preview.** Execution requires `--force` or explicit
   confirmation; dry-run is the default code path.
3. **No deletion of active/held resources.** Never delete files held open (`lsof`/`fuser`)
   or directories with uncommitted git changes (unless `--force`).
4. **No deletion of protected paths.** Never target files younger than the min-age
   threshold (default 24h), installed-binary dirs (`~/.cargo/bin`, `~/.local/bin`),
   SIP/system paths, or the user home directory itself.
5. **No fabricated commands/APIs/versions.** Do not invent CLI flags, crate APIs, or
   dependency versions; verify current stable releases before adding any dependency
   (Base Rules §22, §23).
6. **No irreversible restructuring without authorization.** Do not delete, overwrite, or
   rewrite major structures (docs, generated OpenSpec/tool adapters) without explicit
   approval (Base Rules §8).

## WARNING (flag, do not necessarily halt)

1. **CLI + Skill before MCP.** Do not lead with the MCP server; it is the last deliverable
   (token-overhead rationale in docs §2.1).
2. **Conservative cache detection.** Only treat a directory as a cache when it has a known
   marker (`Cargo.toml`, `package.json`, etc.); preserve ecosystem special cases
   (pnpm hardlinks, Gradle wrapper).
3. **Feature-based layering.** Organize by capability (Scanner / Safety / Knowledge /
   Automation), not technical layers (Base Rules §15–§21).
4. **Strong typing + tests as completion.** No untyped domain models; a change is not done
   until type-checks, lints, and tests pass or the reason they can't run is stated
   (Base Rules §29, §30).
5. **Auditability.** Every cleanup decision must be inspectable and log its rationale;
   preserve human override (Base Rules §13, §25, §34).
6. **Surgical changes.** Minimum code that solves the problem; touch only what's necessary
   (Base Rules §2, §3).

## Stack Note

No `Cargo.toml` exists yet (spec-only). The Rust build/test/lint commands in
`project.json` are targets — expect `kbd-assess` to report a pre-implementation state
until the `dsg` workspace is scaffolded.
