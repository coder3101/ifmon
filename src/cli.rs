use std::ffi::OsString;
use std::path::PathBuf;
use std::time::Duration;

use clap::{Args, Parser, Subcommand, ValueEnum};
use clap_complete::Shell;

use crate::settings::{GraphStyle, GraphView};
use crate::units::{UnitMode, parse_duration};

/// Smallest sampling interval accepted; anything lower just produces noise.
pub const MIN_INTERVAL: Duration = Duration::from_millis(50);

const AFTER_HELP: &str = "\
Examples:
  ifmon                         Launch the interactive dashboard
  ifmon -i en0 -n 1s --bits     Focus en0, sample every second, show bits/s
  ifmon --theme dracula         Use a different color theme
  ifmon list --json             Dump interface details as JSON
  ifmon watch -i eth0 -f csv    Stream eth0 rates as CSV (pipe into a file)
  ifmon completions zsh         Print shell completions

Press 'h' or '?' inside the dashboard for all keyboard shortcuts.";

#[derive(Debug, Parser)]
#[command(
    name = "ifmon",
    version,
    about = "A beautiful, feature-rich terminal network interface monitor",
    after_help = AFTER_HELP
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,

    #[command(flatten)]
    pub common: CommonArgs,

    #[command(flatten)]
    pub tui: TuiArgs,
}

/// Options shared by the dashboard and the `list`/`watch` subcommands.
#[derive(Debug, Clone, Default, Args)]
pub struct CommonArgs {
    /// Sampling interval, e.g. 250ms, 1s, 2.5s [default: 500ms]
    #[arg(
        short = 'n',
        long,
        global = true,
        env = "IFMON_INTERVAL",
        value_parser = parse_interval,
        value_name = "DURATION"
    )]
    pub interval: Option<Duration>,

    /// Interface to focus (dashboard) or include (watch). Repeatable; matches name or alias
    #[arg(short, long = "interface", global = true, value_name = "NAME")]
    pub interfaces: Vec<String>,

    /// Include every interface (loopback, tunnels, virtual adapters)
    #[arg(short, long, global = true)]
    pub all: bool,

    /// Show throughput in bits per second
    #[arg(short, long, global = true, conflicts_with = "bytes")]
    pub bits: bool,

    /// Show throughput in bytes per second
    #[arg(short = 'B', long, global = true)]
    pub bytes: bool,

    /// Use SI (1000-based) byte units like MB instead of IEC (1024-based) like MiB
    #[arg(long, global = true)]
    pub si: bool,

    /// Path to a config file [default: ~/.config/ifmon/config.toml]
    #[arg(
        short,
        long,
        global = true,
        env = "IFMON_CONFIG",
        value_name = "FILE",
        conflicts_with = "no_config"
    )]
    pub config: Option<PathBuf>,

    /// Ignore any config file
    #[arg(long, global = true)]
    pub no_config: bool,
}

/// Dashboard-only options.
#[derive(Debug, Clone, Default, Args)]
pub struct TuiArgs {
    /// Color theme (see `ifmon config themes`)
    #[arg(short, long, env = "IFMON_THEME", value_name = "NAME")]
    pub theme: Option<String>,

    /// Visible graph time window, e.g. 30s, 1m, 5m [default: 1m]
    #[arg(short, long, value_parser = parse_duration, value_name = "DURATION")]
    pub window: Option<Duration>,

    /// Graph layout
    #[arg(long, value_enum)]
    pub view: Option<GraphView>,

    /// Graph drawing style
    #[arg(long, value_enum)]
    pub style: Option<GraphStyle>,

    /// Disable mouse support (keeps native terminal text selection working)
    #[arg(long)]
    pub no_mouse: bool,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// List interfaces with addresses and counters, then exit
    #[command(visible_alias = "ls")]
    List(ListArgs),
    /// Stream live rates to stdout as text, JSON lines or CSV
    Watch(WatchArgs),
    /// Generate shell completions
    Completions {
        #[arg(value_enum)]
        shell: Shell,
    },
    /// Inspect or create the configuration file
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },
}

#[derive(Debug, Clone, Args)]
pub struct ListArgs {
    /// Output as JSON
    #[arg(short, long)]
    pub json: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, ValueEnum)]
pub enum OutputFormat {
    /// Aligned, human-readable columns
    #[default]
    Text,
    /// One JSON object per interface per sample (NDJSON)
    Json,
    /// Comma-separated values with a header row
    Csv,
}

#[derive(Debug, Clone, Args)]
pub struct WatchArgs {
    /// Output format
    #[arg(short, long, value_enum, default_value_t = OutputFormat::Text)]
    pub format: OutputFormat,

    /// Stop after this many samples
    #[arg(short = 'N', long, value_name = "N")]
    pub count: Option<u64>,

    /// Omit the header row (text and CSV)
    #[arg(long)]
    pub no_header: bool,

    /// Also print an aggregate row summing all included interfaces
    #[arg(long)]
    pub total: bool,
}

#[derive(Debug, Clone, Copy, Subcommand)]
pub enum ConfigAction {
    /// Print the config file location
    Path,
    /// Write a commented default config file (fails if one already exists)
    Init {
        /// Overwrite an existing file
        #[arg(long)]
        force: bool,
    },
    /// Print the default configuration
    Default,
    /// List the built-in color themes
    Themes,
}

impl Cli {
    pub fn units_mode(&self) -> Option<UnitMode> {
        if self.common.bits {
            Some(UnitMode::Bits)
        } else if self.common.bytes {
            Some(UnitMode::Bytes)
        } else {
            None
        }
    }
}

fn parse_interval(s: &str) -> Result<Duration, String> {
    let d = parse_duration(s)?;
    if d < MIN_INTERVAL {
        return Err(format!(
            "interval must be at least {}ms",
            MIN_INTERVAL.as_millis()
        ));
    }
    Ok(d)
}

/// Translate the legacy `ifmon <millis>` form into `ifmon --interval <millis>ms`.
pub fn normalize_args(args: impl IntoIterator<Item = OsString>) -> Vec<OsString> {
    let mut args: Vec<OsString> = args.into_iter().collect();
    if let Some(first) = args.get(1).and_then(|a| a.to_str())
        && !first.is_empty()
        && first.bytes().all(|b| b.is_ascii_digit())
    {
        let ms = first
            .parse::<u64>()
            .unwrap_or(0)
            .max(MIN_INTERVAL.as_millis() as u64);
        args[1] = "--interval".into();
        args.insert(2, format!("{ms}ms").into());
    }
    args
}

#[cfg(test)]
mod tests {
    use clap::CommandFactory;

    use super::*;

    fn parse(args: &[&str]) -> Cli {
        Cli::try_parse_from(normalize_args(args.iter().map(OsString::from))).unwrap()
    }

    #[test]
    fn cli_is_well_formed() {
        Cli::command().debug_assert();
    }

    #[test]
    fn parses_dashboard_flags() {
        let cli = parse(&[
            "ifmon", "-n", "1s", "-i", "en0", "--bits", "--theme", "nord",
        ]);
        assert!(cli.command.is_none());
        assert_eq!(cli.common.interval, Some(Duration::from_secs(1)));
        assert_eq!(cli.common.interfaces, vec!["en0"]);
        assert_eq!(cli.units_mode(), Some(UnitMode::Bits));
        assert_eq!(cli.tui.theme.as_deref(), Some("nord"));
    }

    #[test]
    fn legacy_positional_interval_still_works() {
        let cli = parse(&["ifmon", "250"]);
        assert_eq!(cli.common.interval, Some(Duration::from_millis(250)));
        let clamped = parse(&["ifmon", "5"]);
        assert_eq!(clamped.common.interval, Some(MIN_INTERVAL));
    }

    #[test]
    fn rejects_tiny_intervals() {
        assert!(Cli::try_parse_from(["ifmon", "-n", "10ms"]).is_err());
    }

    #[test]
    fn global_flags_work_after_subcommands() {
        let cli = parse(&[
            "ifmon", "watch", "-i", "eth0", "-n", "2s", "-f", "json", "-N", "3",
        ]);
        let Some(Command::Watch(w)) = cli.command else {
            panic!("expected watch");
        };
        assert_eq!(w.format, OutputFormat::Json);
        assert_eq!(w.count, Some(3));
        assert_eq!(cli.common.interfaces, vec!["eth0"]);
        assert_eq!(cli.common.interval, Some(Duration::from_secs(2)));
    }
}
