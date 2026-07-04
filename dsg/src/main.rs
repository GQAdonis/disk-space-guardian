mod config;

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

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
    let effective_dry_run = if force { false } else { dry_run || cfg.dry_run_default };
    eprintln!(
        "[stub] dsg clean — full implementation lands in change-dsg-003 (safety module) + change-dsg-005 (clean integration). dry_run={effective_dry_run}{}{}",
        target.map(|t| format!(" target={}", t.display())).unwrap_or_default(),
        ecosystem.map(|e| format!(" ecosystem={e}")).unwrap_or_default(),
    );
    if effective_dry_run {
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

    let env_filter = std::env::var("DSG_LOG_LEVEL")
        .unwrap_or_else(|_| level.to_string());

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
}
