# Disk Space Guardian: An AgentSkills.io-Compliant Skill for Intelligent Disk Management

## Research Analysis, Architecture, and Project Plan

**Date:** 2026-07-02  
**Research Scope:** AI skill frameworks, disk management tools, build cache ecosystems, machine profiling, automation patterns, LLM integration, safety mechanisms, Rust tooling  
**Methodology:** 8-facet wide exploration via parallel research agents, cross-verification against 200+ sources  

---

## 1. Executive Summary

Modern development workstations routinely accumulate 50–200+ GB of reclaimable disk space across build caches (`target/`, `node_modules`, `.venv`), Docker layers, application caches, and stale temporary directories. Existing tools fall into two inadequate categories: **dumb bulk cleaners** (BleachBit, Stacer) that cannot distinguish active projects from abandoned ones, and **manual CLI tools** (null-e, oxiclean) that require human judgment on every run. Neither category leverages the contextual awareness that AI assistants already possess about a user's projects, activity patterns, and toolchain usage.

This document proposes **Disk Space Guardian** — a hybrid system consisting of:
1. An **AgentSkills.io-compliant SKILL.md** that teaches any AI assistant (Claude, Kimi, Codex, Cursor) how to intelligently manage disk space
2. A **Rust CLI** (`dsg`) that provides fast, safe, cross-platform disk scanning and cleanup
3. An **optional MCP server** that exposes disk management tools to any MCP-compatible agent

The skill is designed to be **context-aware**: it uses the AI's knowledge of active projects, git status, process activity, and user preferences to make intelligent retention decisions. It is **safety-first**: all deletions are previewed, files are moved to trash (not `rm`), and open/active files are never touched. It is **automation-ready**: supports cron, systemd timers, and event-driven triggers with pressure-aware thresholds.

### Key Differentiators

| Capability | Existing Tools | Disk Space Guardian |
|-----------|---------------|---------------------|
| Build cache awareness | Manual scan per ecosystem | Automatic detection of all ecosystems |
| Active project protection | None (or manual exclude lists) | Git status + process activity + AI knowledge |
| Context-aware retention | Age-based only | AI-reasoned: "this project was mentioned in today's standup" |
| LLM integration | None | Native SKILL.md + MCP server dual mode |
| Safety model | Some have dry-run | Dry-run + trash (not rm) + lsof verification + exclusion lists |
| Automation | Cron only | Cron + systemd + event-driven + pressure-aware |
| Cross-platform | Partial | macOS + Linux + Windows (Rust) |

---

## 2. Landscape Analysis

### 2.1 AI Skill Frameworks: The Integration Layer

The AI tooling ecosystem has converged on two complementary standards: **Agent Skills** (agentskills.io) and **MCP** (Model Context Protocol). Both originated from Anthropic but serve different purposes.

**Agent Skills** are file-based, portable knowledge packages. A skill is a directory containing a `SKILL.md` file (YAML frontmatter + Markdown body) plus optional `references/`, `scripts/`, and `assets/` subdirectories. The specification mandates progressive disclosure: Level 1 = metadata (always loaded), Level 2 = full `SKILL.md` body (loaded on trigger), Level 3 = bundled resources (loaded as needed). Skills are supported by Claude Code, Claude Desktop, VS Code, OpenCode, Cursor, OpenAI Codex, GitHub Copilot, Gemini CLI, Goose, Letta, and Kimi Code via the `npx skills` universal installer. The de-facto global installation path is `~/.agents/skills/` (project-local: `.agents/skills/`). [^1][^2][^3]

**MCP** is a JSON-RPC 2.0 protocol that connects AI applications to external systems. MCP servers expose three primitives: Tools (executable functions), Resources (data), and Prompts (templates). Transports include stdio (local) and HTTP+SSE (remote). The official Rust SDK is `rmcp` (crate), which achieves sub-5ms cold starts and 5–15MB binaries — ideal for local system tools. [^4][^5]

The critical insight for our design: **build CLI + Skill first, wrap as MCP second**. Benchmarks show MCP uses 1.3x–80x more tokens than CLI due to tool schema overhead, but MCP wins on structured multi-system tasks. For a disk management tool that will often run locally with heavy filesystem I/O, a fast Rust CLI with a Skill wrapper is the optimal architecture. MCP can be added later as a thin protocol layer. [^6][^7]

### 2.2 Existing Disk Management Tools: The Gap Analysis

We analyzed 15+ tools across categories:

**Rust CLI Developer Tools:**
- **null-e**: Modular design, 18 TUI scan modes, 4-level Git protection, safe trash-by-default, TOML config, JSON output. Reclaims 100+ GB. Limitations: unsigned macOS binary, requires Full Disk Access. [^8]
- **oxiclean**: Single static binary, distro auto-detection, explicit safety rules (never touches `~/.cargo/bin`, preserves pnpm hardlinks, keeps Gradle wrapper). Limitations: Linux-only. [^9]
- **cleaner-upper-rs**: Cross-platform, parallel processing, recursive "cache" directory scanning. Limitations: 168 lines, no CLI options, no safety mechanisms. [^10]
- **storage_ballast_helper**: Innovative predictive pressure control with PID controller, EWMA forecasting, ballast files, progressive delivery. Targets AI coding workloads specifically. Limitations: daemon-only, no interactive mode. [^11]

**Traditional System Cleaners:**
- **BleachBit**: 250+ cleaning rules, aggressive, GTK GUI. Limitations: over-cleans, can break applications, no dev-tool awareness. [^12]
- **Stacer**: Resource monitor + cleaner, Electron-based. Limitations: 150MB+ RAM, limited dev-tool support. [^13]
- **CleanMyMac**: macOS-only, $40/year, proprietary. [^14]

**Ecosystem-Specific Tools:**
- **cargo-sweep**: Time-based Cargo target cleanup (`cargo sweep --time 30`). [^15]
- **cargo-cache**: Cargo cache inspection and cleanup. [^16]
- **Docker builder prune**: Build cache management with `--keep-storage` and `--filter until`. [^17]

**Key Gaps Identified:**
1. **No cross-ecosystem tool** handles Rust + Node + Python + Go + Docker + Xcode simultaneously with safety
2. **No AI context integration** — tools don't know which projects are active, which are abandoned, which are referenced in recent conversations
3. **No pressure-aware automation** — cron runs regardless of disk state; tools don't react to 95% full in real-time
4. **Weak safety model** — many tools default to `rm`, not trash; limited exclusion list support
5. **No knowledge retention** — each run starts from scratch; no learning from user preferences or past decisions
6. **No dual-mode architecture** — tools are either CLI or daemon, not both with AI skill integration

### 2.3 Build Cache Ecosystems: What Consumes Space

| Ecosystem | Primary Locations | Typical Size | Safe Cleanup |
|-----------|-------------------|--------------|--------------|
| **Rust** | `target/`, `~/.cargo/registry`, `~/.cargo/git` | 2–15 GB/project | `cargo clean`, `cargo cache --autoclean` |
| **Node.js** | `node_modules/`, `~/.npm`, `~/.pnpm-store` | 1–5 GB/project | `npm prune`, `pnpm store prune` |
| **Python** | `.venv/`, `~/.cache/pip`, `~/.cache/uv` | 500MB–2GB/project | `pip cache purge`, `uv cache clean` |
| **Go** | `~/go/pkg/mod`, `GOCACHE` | 1–10 GB | `go clean -cache` |
| **Docker** | Build cache, images, containers, volumes | 10–100 GB | `docker builder prune`, `docker image prune` |
| **Xcode** | DerivedData, Simulators, Archives | 20–100 GB | `xcrun simctl runtime delete` |
| **Homebrew** | `~/Library/Caches/Homebrew` | 100MB–1GB | `brew cleanup` |
| **ML/AI** | `~/.cache/huggingface`, Ollama models | 10–100 GB | Manual model management |
| **IDE** | JetBrains, VS Code, Cursor caches | 2–20 GB | IDE-specific cache clearing |

### 2.4 Machine Profiling: Detecting Safe-to-Delete Files

**Process Activity Detection:**
- `lsof` lists all open files; `lsof +D /path` recursively checks directory usage; `lsof | grep deleted` finds space-hogging deleted-but-open files
- `fuser` identifies processes holding specific files; can signal/kill holders
- `pgrep` / `pkill` for process name matching
- `/proc/<pid>/fd` for direct kernel inspection (works on minimal systems)

**Filesystem Timestamps:**
- `atime` (access time): updated on read; `relatime` default means only updated if >24h or if older than mtime/ctime
- `mtime` (modification time): updated on content change — most reliable for "last used"
- `ctime` (change time): updated on metadata change — useful for detecting renames/moves
- macOS `fs_usage` and FSEvents provide real-time filesystem activity monitoring

**Project Activity Detection:**
- Git status: uncommitted changes indicate active work
- Last commit date: projects with no commits in 6+ months are likely abandoned
- Git remote sync status: unpushed commits suggest recent activity

**Container State Detection:**
- Docker: running vs stopped vs exited containers
- Images: tagged vs untagged, referenced by running containers
- Build cache: dangling vs in-use, age-filterable

### 2.5 Automation Patterns: When and How to Clean

**Scheduling Strategies:**
- **Cron**: Simple, portable, but no pressure awareness, no dependency management, missed runs are lost
- **systemd timers**: Better logging (journalctl), `Persistent=true` catches missed runs, resource limits (`CPUQuota`, `MemoryMax`), security sandboxing (`PrivateTmp`, `ProtectSystem`), calendar syntax (`Mon..Fri *-*-* 02:30:00`)
- **Event-driven (inotify/incron)**: React to filesystem events (e.g., clean build artifacts when `Cargo.toml` changes); 92% latency reduction vs polling; risk of infinite loops without debouncing
- **macOS launchd**: Sleep-safe scheduling, `WatchPaths` for event-driven, `StartInterval` for periodic
- **Kubernetes CronJobs**: For containerized environments; kubelet native GC with `HighThresholdPercent`/`LowThresholdPercent`

**Pressure-Aware Triggers:**
- Storage pressure detection with EWMA forecasting (storage_ballast_helper pattern)
- Threshold-based escalation: 70% → log warning, 85% → clean caches, 95% → aggressive cleanup + alert
- Budget-based cleanup: `docker builder prune --keep-storage 10g` — maintain a cap rather than wipe everything

### 2.6 LLM Integration: Context-Aware Decision Making

AI coding agents produce gigabytes of build artifacts per hour. Existing solutions cannot distinguish build artifacts from source files because they lack context. An LLM-integrated disk manager can:

1. **Reason over project structure**: Read `Cargo.toml`, `package.json`, `pyproject.toml` to understand build artifact patterns
2. **Check git status**: Uncommitted changes = active project; recent commits = recently used
3. **Inspect process lists**: `lsof` verification before deleting any directory
4. **Use knowledge memory**: Learn from past cleanup decisions ("user always keeps `librefang` target directories", "user deletes `node_modules` after 7 days")
5. **Apply reflection**: Review past cleanup outcomes, adjust retention policies based on false positives

The Karpathy LLM Wiki pattern — where an LLM maintains a knowledge wiki of facts, decisions, and learnings — can be adapted for disk management. The skill logs cleanup decisions, user feedback, and outcomes to a local wiki file, then uses that knowledge to improve future decisions. [^18]

### 2.7 Safety and Retention: The Trust Model

**Dry-Run Patterns:**
- Default to preview mode; require `--force` or explicit confirmation for deletion
- TUI mode with per-item selection (ratatui)
- JSON output for CI/CD integration
- Show both what would be deleted AND what was excluded by safety rules

**Recovery Mechanisms:**
- Move to Trash/Recycle Bin instead of `rm` — always recoverable
- Use `trash-cli` (`trash`, `trash-list`, `trash-restore`) on Linux, `trash` on macOS
- APFS snapshots on macOS, LVM snapshots on Linux for pre-cleanup state capture

**Retention Policies:**
- **Age-based**: TTL (e.g., delete files older than 30 days)
- **Size-based**: `--keep-storage` budget (e.g., keep 10GB of Docker cache)
- **Count-based**: Keep last N backups, last N simulator runtimes
- **Hybrid**: Combine age + size + activity signals
- **AI-reasoned**: Context-aware retention using project importance, recent usage, user preferences

**Safety Rules:**
- Never delete files younger than configurable threshold (default: 24 hours)
- Never delete open/locked files (verified via `lsof`/`fuser`)
- Never delete without `Cargo.toml`/`package.json` sibling unless explicitly marked as cache
- Preserve `~/.cargo/bin`, `~/.local/bin`, and other installed-binary directories
- Respect `.gitignore` and exclusion lists
- Honor SIP (System Integrity Protection) on macOS

---

## 3. Architecture Design

### 3.1 System Architecture Overview

```
┌─────────────────────────────────────────────────────────────────────┐
│                        AI Assistant Layer                            │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐           │
│  │  Claude  │  │   Kimi   │  │  Codex   │  │  Cursor  │  ...      │
│  └────┬─────┘  └────┬─────┘  └────┬─────┘  └────┬─────┘           │
│       │             │             │             │                    │
│       └─────────────┴──────┬────┴─────────────┘                    │
│                            │                                         │
│                    ┌───────┴───────┐                                 │
│                    │   SKILL.md    │  ← Procedural intelligence      │
│                    │  (Level 1/2)  │    (how to do disk cleanup)    │
│                    └───────┬───────┘                                 │
│                            │                                         │
│       ┌────────────────────┼────────────────────┐                   │
│       │                    │                    │                   │
│  ┌────┴────┐        ┌────┴────┐        ┌────┴────┐                │
│  │  CLI    │        │  MCP    │        │ Scripts │                │
│  │  Mode   │        │ Server  │        │/Assets  │                │
│  └────┬────┘        └────┬────┘        └────┬────┘                │
│       │                  │                    │                    │
└───────┼──────────────────┼────────────────────┼────────────────────┘
        │                  │                    │
        └──────────────────┼────────────────────┘
                           │
              ┌────────────┴────────────┐
              │    Disk Space Guardian   │
              │      Rust Core Engine      │
              │  ┌─────────────────────┐  │
              │  │  Scanner Module     │  │  ← walkdir/jwalk parallel
              │  │  - Ecosystem detect │  │    filesystem scanning
              │  │  - Size analysis    │  │
              │  │  - Staleness scoring│  │
              │  └─────────────────────┘  │
              │  ┌─────────────────────┐  │
              │  │  Safety Module      │  │  ← lsof/fuser verification
              │  │  - Open file check  │  │    exclusion lists, git status
              │  │  - Trash (not rm)   │  │
              │  │  - Exclusion lists  │  │
              │  └─────────────────────┘  │
              │  ┌─────────────────────┐  │
              │  │  Knowledge Module   │  │  ← LLM wiki, user preferences
              │  │  - Cleanup log      │  │    learned patterns
              │  │  - User preferences │  │
              │  │  - Retention policy │  │
              │  └─────────────────────┘  │
              │  ┌─────────────────────┐  │
              │  │  Automation Module  │  │  ← cron, systemd, event triggers
              │  │  - Scheduler hooks    │  │    pressure thresholds
              │  │  - Event listeners    │  │
              │  │  - Pressure detection │  │
              │  └─────────────────────┘  │
              └───────────────────────────┘
                           │
              ┌────────────┼────────────┐
              │            │            │
         ┌────┴────┐  ┌────┴────┐  ┌────┴────┐
         │  Rust   │  │  Node   │  │ Python  │  ...
         │ Caches  │  │ Caches  │  │ Caches  │
         └─────────┘  └─────────┘  └─────────┘
```

### 3.2 Component Specifications

#### 3.2.1 SKILL.md (AgentSkills.io Format)

The skill is a directory with this structure:

```
disk-space-guardian/
├── SKILL.md                    # Main skill definition (Level 1/2)
├── references/
│   ├── architecture.md         # Full architecture docs (Level 3)
│   ├── ecosystem-guide.md      # Per-ecosystem cleanup strategies
│   ├── safety-rules.md         # Safety rules and exclusion lists
│   └── troubleshooting.md      # Common issues and solutions
├── scripts/
│   ├── install.sh              # One-line installer
│   ├── scan.sh                 # Quick disk scan wrapper
│   └── setup-systemd.sh        # Systemd timer setup
└── assets/
    └── demo.png                # Screenshot for skill registry
```

**SKILL.md frontmatter:**
```yaml
---
name: disk-space-guardian
description: >
  Intelligent disk space management for development workstations.
  Detects build caches (Rust, Node, Python, Go, Docker, Xcode),
  identifies stale/abandoned projects, and safely cleans with
  AI-informed retention policies. Always dry-run by default.
  Trigger when: user mentions disk space, running out of storage,
  cleaning caches, or build artifacts taking too much space.
  Do NOT trigger when: user is in the middle of a build or test.
metadata:
  author: Disk Space Guardian Team
  version: 1.0.0
  license: MIT
  platforms: [macos, linux, windows]
  ecosystems: [rust, node, python, go, docker, xcode, homebrew]
---
```

**SKILL.md body structure:**
1. **Quick Start**: One-line scan command
2. **Safety First**: Dry-run, trash-not-rm, exclusion lists
3. **Ecosystem Detection**: How to identify caches per ecosystem
4. **Activity Verification**: Using lsof, git status, process lists
5. **Retention Policies**: Age-based, size-based, AI-reasoned
6. **Automation Setup**: Cron, systemd, event-driven triggers
7. **Knowledge Logging**: How to learn from user preferences
8. **Troubleshooting**: Common issues and solutions

#### 3.2.2 Rust CLI (`dsg`)

The CLI is the execution engine. It must be:
- **Fast**: Parallel filesystem scanning (jwalk for speed, walkdir for safety)
- **Safe**: Dry-run by default, trash-not-rm, lsof verification
- **Cross-platform**: macOS, Linux, Windows via conditional compilation
- **Lightweight**: Single binary, <10MB, no runtime dependencies

**Core crates:**
```toml
[dependencies]
clap = { version = "4", features = ["derive"] }
ratatui = "0.30"
crossterm = "0.29"
jwalk = "0.8"          # Parallel directory walking
walkdir = "2"          # Fallback safe walker
sysinfo = "0.30"       # Process and system info
fs2 = "0.4"            # Disk space queries
trash = "5"            # Cross-platform trash (not rm)
serde = { version = "1", features = ["derive"] }
toml = "0.8"           # Config file parsing
chrono = "0.4"         # Timestamp handling
humansize = "2"        # Human-readable sizes
tokio = { version = "1", features = ["full"] }
anyhow = "1"
tracing = "0.1"
```

**CLI Commands:**
```
dsg scan                    # Quick scan of current directory
dsg scan --deep             # Deep scan of entire system
dsg scan --ecosystem rust   # Scan only Rust caches
dsg scan --stale 30d        # Find files not accessed in 30 days

dsg clean                   # Interactive cleanup (TUI)
dsg clean --dry-run         # Preview only (default)
dsg clean --force           # Execute after preview
dsg clean --target ~/proj   # Clean specific directory

dsg caches                  # Manage global dev caches
dsg caches --list           # List all detected caches
dsg caches --clean cargo    # Clean only cargo caches

dsg watch                   # Watch mode (event-driven)
dsg schedule                # Setup systemd timer / cron job
dsg config                  # Edit configuration (TOML)

dsg status                  # Show disk pressure status
dsg status --json           # Machine-readable output
```

**Configuration file (`~/.config/dsg/config.toml`):**
```toml
[general]
dry_run_default = true
trash_instead_of_delete = true
min_age_hours = 24
exclude_patterns = [
    "*/.cargo/bin",
    "*/.local/bin",
    "*/node_modules/.bin",
]

[ecosystems]
rust = { enabled = true, target_dirs = ["target"], registry_cache = true }
node = { enabled = true, node_modules = true, npm_cache = true, pnpm_store = true }
python = { enabled = true, venvs = true, pip_cache = true, uv_cache = true }
go = { enabled = true, module_cache = true, build_cache = true }
docker = { enabled = true, build_cache = true, dangling_images = true }
xcode = { enabled = true, derived_data = true, simulators = true }

[automation]
pressure_threshold_percent = 85
pressure_check_interval_minutes = 15
systemd_timer = { enabled = false, on_calendar = "daily" }

[knowledge]
wiki_path = "~/.config/dsg/wiki.md"
log_cleanup_decisions = true
```

#### 3.2.3 MCP Server (Optional, Layer 2)

The MCP server is a thin wrapper around the CLI core, exposing the same functionality through MCP tools:

**Tools:**
- `scan_disk`: Scan for reclaimable space, return structured report
- `get_disk_status`: Return current disk usage and pressure level
- `preview_cleanup`: Dry-run cleanup, return what would be deleted
- `execute_cleanup`: Execute cleanup (requires confirmation parameter)
- `get_cache_list`: List all detected caches with sizes
- `set_retention_policy`: Configure age/size thresholds
- `get_exclusion_list`: Return current exclusion patterns
- `add_exclusion`: Add a path to exclusion list

**Resources:**
- `config://dsg`: Current configuration (TOML)
- `wiki://dsg`: Knowledge wiki (Markdown)
- `status://dsg`: Real-time disk pressure status (JSON)

**Implementation:**
```rust
use rmcp::{model::ServerInfo, schemars, ServerHandler, tool, handler::server::TypedServerHandler};

#[derive(ServerHandler)]
struct DsgMcpServer {
    engine: DsgEngine,
}

#[tool]
async fn scan_disk(
    &self,
    #[tool(aggr)] params: ScanParams,
) -> Result<ScanResult, rmcp::Error> {
    self.engine.scan(params).await
}
```

Critical implementation notes from research:
- All logging must route to stderr; stdout pollution corrupts JSON-RPC
- Use absolute binary paths in MCP config; relative paths fail silently
- Protocol version alignment: match rmcp version to client protocol version

### 3.3 Safety Architecture

The safety model is layered:

```
Layer 1: DRY-RUN (always preview first)
  └── Default mode: show what would be deleted
  └── Require --force or explicit confirmation

Layer 2: ACTIVITY VERIFICATION
  └── lsof check: verify no process holds files open
  └── fuser check: verify no process has cwd in target dir
  └── Git status check: verify no uncommitted changes

Layer 3: EXCLUSION LISTS
  └── Global exclusions (installed binaries, system dirs)
  └── User exclusions (configurable patterns)
  └── Ecosystem-specific exclusions (pnpm hardlinks, Gradle wrapper)
  └── AI-reasoned exclusions ("user mentioned this project today")

Layer 4: TRASH (not rm)
  └── Move to Trash/Recycle Bin instead of permanent deletion
  └── Cross-platform: trash-cli (Linux), trash (macOS), Recycle Bin (Windows)

Layer 5: RETENTION POLICIES
  └── Age-based: configurable TTL (default: 24h minimum, 30d typical)
  └── Size-based: --keep-storage budget
  └── Activity-based: recently accessed = protected
  └── AI-reasoned: context-aware importance scoring

Layer 6: AUDIT LOG
  └── Log every cleanup decision with rationale
  └── Log user overrides and feedback
  └── Log before/after disk usage
```

### 3.4 Knowledge Module: The LLM Wiki

The knowledge module maintains a local wiki file that the AI skill reads and writes to:

```markdown
# Disk Space Guardian Knowledge Wiki

## User Preferences
- Always keep `librefang` target directories (user mentioned active development)
- Delete `node_modules` after 7 days of inactivity
- Never clean Docker images tagged `surrealdb/surrealdb`
- Prefer aggressive cleanup on `/private/tmp`

## Project Importance Scores
| Project | Last Commit | Last Mentioned | Importance | Notes |
|---------|-------------|----------------|------------|-------|
| prometheus | 2026-06-30 | 2026-07-02 | HIGH | Active, running cargo test on external drive |
| librefang | 2026-06-28 | 2026-07-01 | HIGH | Upstream merge work |
| old-prototype | 2025-12-01 | never | LOW | No commits in 6 months |

## Cleanup History
| Date | Action | Space Recovered | User Feedback |
|------|--------|-------------------|---------------|
| 2026-07-02 | Deleted librefang merge dirs in /private/tmp | 37GB | Approved |
| 2026-07-02 | Docker builder prune + image prune | 36.5GB | Approved |
| 2026-07-02 | Deleted local target/ directories | 3.2GB | Approved |

## Learned Patterns
- User always approves Docker build cache cleanup when >20GB
- User prefers to keep all Rust toolchains (7 installed)
- User has external drive builds that should not affect local cleanup
```

The AI skill reads this wiki before each cleanup session, uses it to inform decisions, and updates it after each session with new learnings.

---

## 4. Feature Specification

### 4.1 Phase 1: Core Features (MVP)

| Feature | Priority | Description |
|---------|----------|-------------|
| Multi-ecosystem scan | P0 | Detect Rust, Node, Python, Go, Docker, Xcode, Homebrew caches |
| Parallel filesystem scan | P0 | Fast directory traversal using jwalk |
| Dry-run preview | P0 | Show what would be deleted without deleting |
| Trash-based deletion | P0 | Move to Trash instead of `rm` |
| lsof verification | P0 | Check for open files before deletion |
| Git status integration | P0 | Detect active vs abandoned projects |
| TUI mode | P0 | Interactive ratatui-based cleanup |
| SKILL.md | P0 | agentskills.io-compliant skill definition |
| Config file (TOML) | P0 | User-customizable settings |
| Cross-platform support | P0 | macOS, Linux, Windows |

### 4.2 Phase 2: Intelligence Layer

| Feature | Priority | Description |
|---------|----------|-------------|
| Knowledge wiki | P1 | Local markdown wiki for user preferences and learned patterns |
| AI-reasoned retention | P1 | Context-aware importance scoring using conversation history |
| Project staleness detection | P1 | Git commit age + last access + conversation mentions |
| Pressure-aware automation | P1 | React to disk pressure in real-time |
| Exclusion list management | P1 | User-configurable + AI-suggested exclusions |
| JSON output | P1 | Machine-readable output for CI/CD |
| Systemd timer setup | P1 | Automated systemd timer configuration |
| Cron job setup | P1 | Automated cron job generation |

### 4.3 Phase 3: Advanced Features

| Feature | Priority | Description |
|---------|----------|-------------|
| MCP server | P2 | JSON-RPC 2.0 server exposing disk management tools |
| Event-driven cleanup | P2 | inotify/fs_event triggers for automatic cleanup |
| Duplicate detection | P2 | Find duplicate files across caches |
| Compression suggestions | P2 | Identify compressible directories |
| Remote cache integration | P2 | sccache/remote cache cleanup |
| Dashboard/monitoring | P2 | Long-term disk usage trends |
| Team/shared mode | P2 | Multi-user environment support |
| Plugin system | P2 | Custom ecosystem plugins |

---

## 5. Integration Guide

### 5.1 AI Assistant Integration

**Claude Code / Claude Desktop:**
```bash
# Install via skills.sh
npx skills add disk-space-guardian

# Or manually
git clone https://github.com/disk-space-guardian/skill.git ~/.claude/skills/disk-space-guardian
```

**Kimi Code:**
```bash
npx skills add disk-space-guardian --agent kimi
# Installs to ~/.agents/skills/disk-space-guardian/
```

**OpenAI Codex:**
```bash
codex --enable skills
npx skills add disk-space-guardian --agent codex
# Installs to ~/.codex/skills/disk-space-guardian/
```

**Cursor:**
```bash
npx skills add disk-space-guardian --agent cursor
# Installs to ~/.cursor/skills/disk-space-guardian/
```

### 5.2 MCP Server Integration

**Claude Desktop:**
Add to `claude_desktop_config.json`:
```json
{
  "mcpServers": {
    "disk-space-guardian": {
      "command": "/usr/local/bin/dsg",
      "args": ["mcp", "serve"]
    }
  }
}
```

**VS Code:**
Add to `.vscode/mcp.json`:
```json
{
  "servers": {
    "disk-space-guardian": {
      "type": "stdio",
      "command": "dsg",
      "args": ["mcp", "serve"]
    }
  }
}
```

### 5.3 Automation Integration

**systemd timer setup:**
```bash
dsg schedule --systemd --on-calendar="daily" --threshold=85
# Creates: /etc/systemd/system/dsg-cleanup.timer
#          /etc/systemd/system/dsg-cleanup.service
```

**cron setup:**
```bash
dsg schedule --cron --expression="0 2 * * *" --threshold=85
# Adds entry to user's crontab
```

**launchd (macOS):**
```bash
dsg schedule --launchd --interval=86400 --threshold=85
# Creates: ~/Library/LaunchAgents/com.dsg.cleanup.plist
```

---

## 6. Safety & Retention Framework

### 6.1 Default Safety Rules

These rules are hardcoded and cannot be overridden without explicit `--unsafe` flag:

1. **Never delete files younger than 24 hours** (configurable, default: 24h)
2. **Never delete files held open by any process** (verified via lsof/fuser)
3. **Never delete directories containing uncommitted git changes** (unless --force)
4. **Never delete installed binaries** (`~/.cargo/bin`, `~/.local/bin`, etc.)
5. **Never delete without dry-run preview** (unless --force)
6. **Never use `rm`; always move to Trash** (unless --permanent)
7. **Never delete system directories** (SIP-protected paths, `/System`, `/usr`, etc.)
8. **Never delete user home directory** (unless explicitly targeting a subdir)

### 6.2 Exclusion Patterns

Default global exclusions:
```
~/.cargo/bin/**
~/.local/bin/**
~/.npm/lib/node_modules/**
**/node_modules/.bin/**
**/target/release/.fingerprint/**
**/target/debug/.fingerprint/**
/System/**
/usr/**
/private/var/db/**
```

User-configurable exclusions via `~/.config/dsg/config.toml`:
```toml
exclude_patterns = [
    "*/important-project/**",
    "*/backups/**",
]
```

### 6.3 Retention Policy Matrix

| Ecosystem | Default Min Age | Default Max Age | Size Budget | Activity Signal |
|-----------|-----------------|-----------------|-------------|-----------------|
| Rust target/ | 24h | 30d | None | Git commits, cargo test runs |
| Cargo registry | 7d | 90d | 2GB | Crate downloads |
| node_modules | 24h | 14d | None | npm install, package.json changes |
| npm cache | 7d | 30d | 1GB | Cache hits |
| Docker build cache | 1h | 7d | 10GB | Build activity |
| Docker images | 24h | 30d | None | Container references |
| Xcode DerivedData | 1h | 7d | 20GB | Build activity |
| /tmp, /private/tmp | 1h | 7d | None | File access |
| Python .venv | 24h | 30d | None | pip install, pyproject.toml changes |
| pip cache | 7d | 30d | 500MB | Cache hits |

### 6.4 Pressure-Aware Escalation

```
Disk Usage Level    Action
─────────────────────────────────────────────────────
< 70%               No action (monitoring only)
70-80%              Log warning, suggest cleanup
80-85%              Clean safe caches (package managers, temp)
85-90%              Clean build caches (target, node_modules)
90-95%              Aggressive: clean old targets, Docker cache
95-99%              Emergency: move to Trash, alert user
> 99%               Critical: stop non-essential processes, alert
```

---

## 7. Project Plan & Milestones

### Phase 1: Foundation (Weeks 1–4)
- **Week 1**: Project scaffolding, CLI structure with clap, config file parsing
- **Week 2**: Filesystem scanner (jwalk integration), ecosystem detection
- **Week 3**: Safety module (lsof, trash, exclusion lists), dry-run implementation
- **Week 4**: TUI mode (ratatui), cross-platform trash implementation

**Deliverable:** `dsg` CLI v0.1.0 — scan, preview, clean with safety

### Phase 2: Intelligence (Weeks 5–8)
- **Week 5**: Git integration (status, commit age, remote sync)
- **Week 6**: Knowledge wiki module (markdown read/write, preference learning)
- **Week 7**: AI-reasoned retention policies (project importance scoring)
- **Week 8**: Pressure detection and automation hooks (systemd, cron, launchd)

**Deliverable:** `dsg` CLI v0.2.0 + SKILL.md v1.0 — context-aware cleanup

### Phase 3: Integration (Weeks 9–12)
- **Week 9**: SKILL.md refinement, progressive disclosure optimization
- **Week 10**: MCP server implementation (rmcp crate)
- **Week 11**: Event-driven cleanup (inotify/fsevents)
- **Week 12**: Testing, documentation, registry publishing

**Deliverable:** `dsg` v1.0.0 + published to skills.sh + MCP Registry

### Phase 4: Advanced Features (Weeks 13–16)
- **Week 13**: Duplicate detection, compression suggestions
- **Week 14**: Remote cache integration (sccache cleanup)
- **Week 15**: Dashboard/monitoring, long-term trends
- **Week 16**: Team/shared mode, plugin system

**Deliverable:** `dsg` v2.0.0 — enterprise-ready with advanced features

### Resource Requirements

| Role | Count | Time |
|------|-------|------|
| Rust developer (core engine) | 1 | Full-time |
| Rust developer (CLI/TUI) | 1 | Full-time |
| AI integration engineer | 1 | Part-time (Weeks 5-12) |
| Technical writer | 1 | Part-time (Weeks 8-12) |
| QA / tester | 1 | Part-time (Weeks 4-12) |

---

## 8. Recommendations

### 8.1 Immediate Actions (This Week)

1. **Create the SKILL.md skeleton** with progressive disclosure (L1/L2/L3) and the 8 sections defined in Section 3.2.1
2. **Scaffold the Rust CLI** with clap, basic scan command, and dry-run mode
3. **Implement the safety module** first — lsof check, trash integration, exclusion lists
4. **Test on the user's machine** using the exact scenarios from today's cleanup session (librefang merge dirs, Docker caches, local targets)

### 8.2 Architecture Decisions

| Decision | Recommendation | Rationale |
|----------|----------------|-----------|
| CLI vs daemon | **CLI first, daemon optional** | CLI is simpler, safer, and easier to integrate with AI skills. Daemon adds complexity for marginal benefit. |
| Skill vs MCP | **Skill first, MCP second** | Skills have lower token overhead and are universally supported. MCP adds ~1.3x–80x token cost but enables IDE integration. |
| Trash vs rm | **Always trash** | Recovery is essential. Even "safe" deletions can be mistakes. |
| Parallel walker | **jwalk + walkdir fallback** | jwalk is faster but walkdir is safer for edge cases. Use jwalk for scanning, walkdir for verification. |
| Config format | **TOML** | TOML is human-readable, widely used in Rust ecosystem, supports comments. |
| Async runtime | **Tokio** | Required by rmcp for MCP server. Dominant ecosystem. |
| macOS trash | **trash crate** | Cross-platform Rust crate that handles macOS Trash, Linux trash-cli, Windows Recycle Bin. |

### 8.3 Risk Mitigation

| Risk | Mitigation |
|------|------------|
| Accidental deletion of active project | 3-layer safety: dry-run + lsof + git status; never delete without preview |
| False positives in ecosystem detection | Conservative detection: only clean directories with known markers (Cargo.toml, package.json) |
| Token overhead in AI integration | Progressive disclosure: L1 = 100 tokens, L2 = 500 tokens, L3 = on-demand |
| Platform-specific bugs | CI testing on macOS, Linux, Windows; conditional compilation for OS-specific features |
| User trust ("I don't want AI deleting my files") | Full transparency: show exactly what will be deleted, require explicit confirmation, all moves to Trash |

### 8.4 Success Metrics

| Metric | Target |
|--------|--------|
| Time to first scan | < 5 seconds for current directory |
| Time to full system scan | < 30 seconds for 1M files |
| Space recovered per run | 5–50 GB average |
| False positive rate | < 1% (files deleted that user wanted to keep) |
| User adoption | 1000+ installs in first 3 months |
| Skill registry rating | 4.5+ stars on skills.sh |

---

## 9. References

This research synthesized findings from 200+ sources across 8 research facets. The primary source documents are preserved in the research directory:

- `disk_space_skill_wide01.md` — AI Skill Frameworks & Integration
- `disk_space_skill_wide02.md` — Existing Disk Management Tools
- `disk_space_skill_wide03.md` — Build Cache Management
- `disk_space_skill_wide04.md` — Machine Profiling & Activity Detection
- `disk_space_skill_wide05.md` — Automation Patterns
- `disk_space_skill_wide06.md` — LLM Integration & Smart Decision Making
- `disk_space_skill_wide07.md` — Safety & Retention Policies
- `disk_space_skill_wide08.md` — Rust CLI & MCP Server Architecture

Key external references cited inline throughout this document.

---

*End of Document*
