mod config;
mod safety;

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
    if json {
        println!(r#"{{"status":"ok","message":"dsg status stub — scanner core arrives in change-dsg-004"}}"#);
    } else {
        println!("dsg status — scanner core arrives in change-dsg-004");
    }
    Ok(())
}

fn cmd_scan(
    _deep: bool,
    ecosystem: Option<&str>,
    _stale: Option<&str>,
    _json: bool,
    _cfg: &config::Config,
) -> Result<()> {
    eprintln!(
        "[stub] dsg scan — full implementation lands in change-dsg-004 (scanner core){}",
        ecosystem
            .map(|e| format!(" [ecosystem: {e}]"))
            .unwrap_or_default()
    );
    Ok(())
}

fn cmd_clean(
    dry_run: bool,
    force: bool,
    target: Option<&std::path::Path>,
    ecosystem: Option<&str>,
    cfg: &config::Config,
) -> Result<()> {
    // dry-run is the default; --force overrides; --dry-run is also explicit
    let effective_dry_run = !force && (dry_run || cfg.dry_run_default);

    let engine = safety::SafetyEngine::new(effective_dry_run, Arc::new(cfg.clone()));

    if effective_dry_run {
        println!(
            "dsg clean [dry-run]{}{}\n\
             No files will be deleted. Pass --force to actually trash items.",
            target
                .map(|t| format!(" --target {}", t.display()))
                .unwrap_or_default(),
            ecosystem
                .map(|e| format!(" --ecosystem {e}"))
                .unwrap_or_default(),
        );
        println!(
            "\nSafety engine ready: min_age={}d, {} exclusion(s) in config.",
            cfg.min_age_days,
            cfg.exclude_paths.len()
        );
        println!("[stub] scanner integration arrives in change-dsg-004 + change-dsg-005");
        // Non-zero exit so callers can distinguish dry-run from real clean
        std::process::exit(2);
    }

    // --force path: safety checks would run per item here once scanner is wired in
    println!(
        "dsg clean --force{}{}\n\
         Safety engine: dry_run={}, min_age_secs={}",
        target
            .map(|t| format!(" --target {}", t.display()))
            .unwrap_or_default(),
        ecosystem
            .map(|e| format!(" --ecosystem {e}"))
            .unwrap_or_default(),
        engine.dry_run,
        engine.min_age_secs,
    );
    println!("[stub] item-level clean integration arrives in change-dsg-005");
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
