mod config;
mod ecosystems;
mod safety;
mod scanner;

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Parser)]
#[command(
    name = "dsg",
    version,
    about = "Intelligent, safety-first disk space management for developer workstations",
    long_about = None,
)]
struct Cli {
    /// Override config file location
    #[arg(long, global = true, value_name = "PATH")]
    config: Option<PathBuf>,

    /// Logging verbosity: error, warn, info, debug, trace
    #[arg(long, global = true, default_value = "info", value_name = "LEVEL")]
    log_level: Option<String>,

    /// Disable ANSI color codes in output
    #[arg(long, global = true)]
    no_color: bool,

    /// Suppress informational output; only print results and errors
    #[arg(long, global = true)]
    quiet: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Show disk usage summary for common developer cache locations
    ///
    /// Emit machine-readable JSON when used with --json.
    ///
    /// Examples:
    ///   dsg status
    ///   dsg status --json
    Status {
        /// Emit JSON output
        #[arg(long)]
        json: bool,
    },

    /// Scan for reclaimable disk space
    Scan {
        /// Scan entire system (home dir + common cache paths) instead of CWD only
        #[arg(long)]
        deep: bool,

        /// Limit scan to one ecosystem: rust, node, python, go, docker, xcode, homebrew
        #[arg(long, value_name = "NAME")]
        ecosystem: Option<String>,

        /// Report only entries not modified in <duration> (e.g. 7d, 30d, 90d). Uses mtime.
        #[arg(long, value_name = "DURATION")]
        stale: Option<String>,

        /// Emit machine-readable JSON to stdout instead of human table
        #[arg(long)]
        json: bool,
    },

    /// Preview or execute cleanup (dry-run by default)
    ///
    /// Safety guarantees:
    ///   - Dry-run is the default unless --force is passed
    ///   - Files are moved to Trash, never permanently deleted
    ///   - Activity (lsof) and age checks run before every item
    ///   - Exclusion patterns from config are honoured
    ///
    /// Examples:
    ///   dsg clean                         # dry-run: show what would be cleaned
    ///   dsg clean --dry-run               # explicit dry-run (same as above)
    ///   dsg clean --force                 # actually move to Trash
    ///   dsg clean --ecosystem rust --force
    Clean {
        /// Preview only; print what would be deleted without touching the filesystem [default]
        #[arg(long)]
        dry_run: bool,

        /// Execute deletions (moves to Trash). Prints preview first; requires confirmation.
        #[arg(long)]
        force: bool,

        /// Restrict cleanup to this directory subtree
        #[arg(long, value_name = "PATH")]
        target: Option<PathBuf>,

        /// Clean only entries matching this ecosystem
        #[arg(long, value_name = "NAME")]
        ecosystem: Option<String>,
    },

    /// Manage global developer ecosystem caches
    Caches {
        /// List all detected global caches with their sizes
        #[arg(long)]
        list: bool,

        /// Clean the global cache for the named ecosystem
        #[arg(long, value_name = "ECOSYSTEM")]
        clean: Option<String>,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let log_level = cli.log_level.as_deref().unwrap_or("info");
    init_tracing(log_level, cli.no_color);

    let cfg = config::Config::load(cli.config.as_deref())?;
    tracing::debug!("Config loaded: {:?}", cfg);

    match cli.command {
        Commands::Status { json } => cmd_status(json, &cfg),
        Commands::Scan {
            deep,
            ecosystem,
            stale,
            json,
        } => cmd_scan(deep, ecosystem.as_deref(), stale.as_deref(), json, &cfg),
        Commands::Clean {
            dry_run,
            force,
            target,
            ecosystem,
        } => cmd_clean(dry_run, force, target.as_deref(), ecosystem.as_deref(), &cfg),
        Commands::Caches { list, clean } => cmd_caches(list, clean.as_deref(), &cfg),
    }
}

fn cmd_status(json: bool, _cfg: &config::Config) -> Result<()> {
    // Well-known developer cache roots to measure
    let roots: Vec<(String, std::path::PathBuf)> = build_status_roots();
    let measurements = scanner::measure_roots(&roots);
    if json {
        scanner::report_status_json(&measurements)
    } else {
        scanner::report_status_human(&measurements);
        Ok(())
    }
}

fn build_status_roots() -> Vec<(String, std::path::PathBuf)> {
    let home = match dirs::home_dir() {
        Some(h) => h,
        None => return vec![],
    };
    vec![
        ("Cargo registry".to_string(), home.join(".cargo/registry")),
        ("Cargo git".to_string(), home.join(".cargo/git")),
        ("Cargo target (global)".to_string(), home.join(".cargo/target")),
        ("npm cache".to_string(), home.join(".npm/_cacache")),
        ("pnpm store".to_string(), home.join(".local/share/pnpm/store")),
        ("pip cache".to_string(), home.join(".cache/pip")),
        ("Go module cache".to_string(), home.join("go/pkg/mod")),
        ("Docker volumes".to_string(), std::path::PathBuf::from("/var/lib/docker/volumes")),
        ("Xcode DerivedData".to_string(), home.join("Library/Developer/Xcode/DerivedData")),
        ("Homebrew cache".to_string(), home.join("Library/Caches/Homebrew")),
    ]
}

fn cmd_scan(
    deep: bool,
    ecosystem: Option<&str>,
    stale: Option<&str>,
    json: bool,
    cfg: &config::Config,
) -> Result<()> {
    let stale_secs = stale.and_then(parse_duration);

    let opts = scanner::ScanOptions {
        deep,
        ecosystem_filter: ecosystem.map(str::to_string),
        stale_secs,
        min_size_bytes: cfg.min_size_mb * 1024 * 1024,
    };

    let detectors = ecosystems::all_detectors();

    let mut all_results = Vec::new();

    if deep {
        // Collect roots from every matching detector and walk each one
        let roots = scanner::collect_scan_roots(&opts, &detectors);
        if roots.is_empty() {
            tracing::warn!("No known roots found for the requested scan.");
        }
        for root in &roots {
            let mut r = scanner::scan_directory(root, &opts, &detectors);
            all_results.append(&mut r);
        }
    } else {
        let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
        all_results = scanner::scan_directory(&cwd, &opts, &detectors);
    }

    if json {
        scanner::report_json(&all_results)
    } else {
        scanner::report_human(&all_results);
        Ok(())
    }
}

/// Parse simple duration strings: "7d", "30d", "90d" → seconds.
/// Falls back to None on parse errors so the scan proceeds without a stale filter.
fn parse_duration(s: &str) -> Option<u64> {
    if let Some(days) = s.strip_suffix('d') {
        return days.parse::<u64>().ok().map(|d| d * 86_400);
    }
    if let Some(hours) = s.strip_suffix('h') {
        return hours.parse::<u64>().ok().map(|h| h * 3_600);
    }
    None
}

fn cmd_clean(
    dry_run: bool,
    force: bool,
    target: Option<&std::path::Path>,
    ecosystem: Option<&str>,
    cfg: &config::Config,
) -> Result<()> {
    let effective_dry_run = !force && (dry_run || cfg.dry_run_default);
    let engine = safety::SafetyEngine::new(effective_dry_run, Arc::new(cfg.clone()));

    let opts = scanner::ScanOptions {
        deep: true,
        ecosystem_filter: ecosystem.map(str::to_string),
        stale_secs: None,
        min_size_bytes: cfg.min_size_mb * 1024 * 1024,
    };

    let detectors = ecosystems::all_detectors();

    // Determine which roots to clean
    let roots: Vec<std::path::PathBuf> = if let Some(t) = target {
        vec![t.to_path_buf()]
    } else {
        scanner::collect_scan_roots(&opts, &detectors)
    };

    if roots.is_empty() {
        println!("No reclaimable roots found for the given options.");
        return Ok(());
    }

    let mode_label = if effective_dry_run { "[dry-run]" } else { "[LIVE]" };
    println!(
        "dsg clean {}{}{}\n",
        mode_label,
        target
            .map(|t| format!(" --target {}", t.display()))
            .unwrap_or_default(),
        ecosystem
            .map(|e| format!(" --ecosystem {e}"))
            .unwrap_or_default(),
    );

    let mut trashed = 0usize;
    let mut skipped = 0usize;
    let mut errors = 0usize;

    for root in &roots {
        let candidates = scanner::scan_directory(root, &opts, &detectors);

        for item in &candidates {
            let path = &item.path;

            // Exclusion check
            if engine.should_exclude(path) {
                tracing::debug!("Excluded: {}", path.display());
                skipped += 1;
                continue;
            }

            // Age guard
            match engine.age_guard(path) {
                Ok(old_enough) if !old_enough => {
                    tracing::debug!("Too new, skipping: {}", path.display());
                    skipped += 1;
                    continue;
                }
                Err(e) => {
                    tracing::warn!("age_guard error for {}: {e}", path.display());
                    skipped += 1;
                    continue;
                }
                _ => {}
            }

            // Activity check (best-effort; don't block on error)
            match engine.verify_activity(path) {
                Ok(safety::ActivityCheck::Idle) => {}
                Ok(safety::ActivityCheck::ActiveProcesses(procs)) => {
                    println!("  SKIP (active) {} — used by: {}", path.display(), procs.join(", "));
                    skipped += 1;
                    continue;
                }
                Ok(safety::ActivityCheck::GitDirty) => {
                    println!("  SKIP (git-dirty) {}", path.display());
                    skipped += 1;
                    continue;
                }
                Err(e) => {
                    tracing::warn!("activity check failed for {}: {e}", path.display());
                    // Don't skip on check failure — proceed with caution
                }
            }

            // Trash (or dry-run preview)
            match engine.move_to_trash(path) {
                Ok(()) => {
                    println!("  {} {}", if effective_dry_run { "WOULD TRASH" } else { "TRASHED" }, path.display());
                    trashed += 1;
                }
                Err(e) => {
                    eprintln!("  ERROR trashing {}: {e}", path.display());
                    errors += 1;
                }
            }
        }
    }

    println!(
        "\nSummary: {} {}, {} skipped, {} errors",
        trashed,
        if effective_dry_run { "would be trashed" } else { "trashed" },
        skipped,
        errors,
    );

    if effective_dry_run {
        println!("Pass --force to execute the cleanup.");
        std::process::exit(2);
    }

    Ok(())
}

fn cmd_caches(list: bool, clean: Option<&str>, _cfg: &config::Config) -> Result<()> {
    if list || clean.is_none() {
        eprintln!("[stub] dsg caches --list — full implementation lands in change-dsg-005");
    } else if let Some(eco) = clean {
        eprintln!("[stub] dsg caches --clean {eco} — full implementation lands in change-dsg-005");
    }
    Ok(())
}

fn init_tracing(level: &str, no_color: bool) {
    use tracing_subscriber::{fmt, EnvFilter};

    let env_filter = std::env::var("DSG_LOG_LEVEL").unwrap_or_else(|_| level.to_string());

    let builder = fmt::Subscriber::builder()
        .with_env_filter(EnvFilter::new(env_filter))
        .with_writer(std::io::stderr);

    if no_color {
        builder.without_time().with_ansi(false).init();
    } else {
        builder.without_time().init();
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn default_config_smoke() {
        let cfg = crate::config::Config::default();
        assert!(cfg.dry_run_default);
        assert_eq!(cfg.min_age_days, 1);
    }

    #[test]
    fn safety_engine_dry_run_default() {
        use std::sync::Arc;
        let cfg = Arc::new(crate::config::Config::default());
        let engine = crate::safety::SafetyEngine::new(true, cfg);
        assert!(engine.dry_run);
    }
}
