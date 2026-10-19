//! Effective runtime settings, merged from defaults, config file, environment and flags.

use std::path::PathBuf;
use std::time::Duration;

use clap::ValueEnum;
use color_eyre::eyre::{Result, WrapErr, eyre};
use serde::{Deserialize, Serialize};

use crate::cli::{Cli, MIN_INTERVAL};
use crate::config::{self, FileConfig};
use crate::theme::Theme;
use crate::units::{UnitMode, Units, parse_duration};

pub const DEFAULT_INTERVAL: Duration = Duration::from_millis(500);
pub const DEFAULT_WINDOW: Duration = Duration::from_secs(60);
/// Longest graph window offered when zooming out.
pub const MAX_WINDOW: Duration = Duration::from_secs(30 * 60);
/// Upper bound on samples kept per interface, regardless of interval.
pub const MAX_HISTORY_POINTS: usize = 7_200;

/// How the throughput graphs are laid out.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, ValueEnum, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GraphView {
    /// Separate RX and TX charts
    #[default]
    Split,
    /// RX and TX overlaid in one chart
    Combined,
    /// Separate RX and TX packet-rate charts
    Packets,
}

impl GraphView {
    pub fn next(self) -> Self {
        match self {
            Self::Split => Self::Combined,
            Self::Combined => Self::Packets,
            Self::Packets => Self::Split,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Split => "split",
            Self::Combined => "combined",
            Self::Packets => "packets",
        }
    }
}

/// How graph data points are drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, ValueEnum, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GraphStyle {
    /// Smooth braille line
    #[default]
    Braille,
    /// Chunky half-block line
    Block,
    /// Filled vertical bars
    Bars,
    /// Scatter dots
    Dots,
}

impl GraphStyle {
    pub fn next(self) -> Self {
        match self {
            Self::Braille => Self::Block,
            Self::Block => Self::Bars,
            Self::Bars => Self::Dots,
            Self::Dots => Self::Braille,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Braille => "braille",
            Self::Block => "block",
            Self::Bars => "bars",
            Self::Dots => "dots",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Settings {
    pub interval: Duration,
    pub window: Duration,
    pub units: Units,
    pub theme: Theme,
    pub show_all: bool,
    pub mouse: bool,
    pub view: GraphView,
    pub style: GraphStyle,
    /// Interfaces requested on the command line or in the config file.
    pub focus: Vec<String>,
    pub config_path: Option<PathBuf>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            interval: DEFAULT_INTERVAL,
            window: DEFAULT_WINDOW,
            units: Units::default(),
            theme: Theme::default(),
            show_all: false,
            mouse: true,
            view: GraphView::default(),
            style: GraphStyle::default(),
            focus: Vec::new(),
            config_path: None,
        }
    }
}

impl Settings {
    pub fn resolve(cli: &Cli) -> Result<Self> {
        let (file, config_path) = if cli.common.no_config {
            (FileConfig::default(), None)
        } else {
            config::load(cli.common.config.as_deref())?
        };
        let source = config_path.clone();
        Self::merge(cli, &file, config_path).wrap_err_with(|| match source {
            Some(path) => format!("invalid settings (config file: {})", path.display()),
            None => "invalid settings".to_string(),
        })
    }

    fn merge(cli: &Cli, file: &FileConfig, config_path: Option<PathBuf>) -> Result<Self> {
        let interval = match cli.common.interval {
            Some(d) => d,
            None => match legacy_interval_env() {
                Some(d) => d,
                None => file
                    .interval
                    .as_deref()
                    .map(|s| parse_duration(s).map_err(|e| eyre!("config `interval`: {e}")))
                    .transpose()?
                    .unwrap_or(DEFAULT_INTERVAL),
            },
        }
        .max(MIN_INTERVAL);

        let window = match cli.tui.window {
            Some(d) => d,
            None => file
                .window
                .as_deref()
                .map(|s| parse_duration(s).map_err(|e| eyre!("config `window`: {e}")))
                .transpose()?
                .unwrap_or(DEFAULT_WINDOW),
        }
        .clamp(Duration::from_secs(5), MAX_WINDOW);

        let units = Units::new(
            cli.units_mode().or(file.units).unwrap_or(UnitMode::Bytes),
            cli.common.si || file.si.unwrap_or(false),
        );

        let no_color = std::env::var_os("NO_COLOR").is_some_and(|v| !v.is_empty());
        let theme_name = cli
            .tui
            .theme
            .clone()
            .or_else(|| no_color.then(|| Theme::MONO.name.to_string()))
            .or_else(|| file.theme.clone());
        let base = match theme_name {
            Some(name) => Theme::by_name(&name).ok_or_else(|| {
                eyre!(
                    "unknown theme '{name}' (available: {})",
                    Theme::names().collect::<Vec<_>>().join(", ")
                )
            })?,
            None => Theme::default(),
        };
        let theme = base
            .with_overrides(&file.colors)
            .map_err(|e| eyre!(e))
            .wrap_err("config [colors]")?;

        let mut focus = cli.common.interfaces.clone();
        if focus.is_empty() {
            focus.extend(file.interface.clone());
        }

        Ok(Self {
            interval,
            window,
            units,
            theme,
            show_all: cli.common.all || file.all.unwrap_or(false),
            mouse: !cli.tui.no_mouse && file.mouse.unwrap_or(true),
            view: cli.tui.view.or(file.view).unwrap_or_default(),
            style: cli.tui.style.or(file.style).unwrap_or_default(),
            focus,
            config_path,
        })
    }

    pub fn history_capacity(&self) -> usize {
        history_capacity(self.interval)
    }
}

/// Samples to keep per interface so the widest zoom level is fully covered.
pub fn history_capacity(interval: Duration) -> usize {
    let needed = MAX_WINDOW.as_secs_f64() / interval.as_secs_f64();
    (needed.ceil() as usize + 2).min(MAX_HISTORY_POINTS)
}

/// `IFMON_UPDATE_INTERVAL_MS` from ifmon 0.1, still honored for compatibility.
fn legacy_interval_env() -> Option<Duration> {
    std::env::var("IFMON_UPDATE_INTERVAL_MS")
        .ok()?
        .trim()
        .parse::<u64>()
        .ok()
        .map(Duration::from_millis)
}

#[cfg(test)]
mod tests {
    use clap::Parser;

    use super::*;

    fn merged(args: &[&str], file: &str) -> Result<Settings> {
        let cli = Cli::try_parse_from(args).unwrap();
        Settings::merge(&cli, &config::parse(file).unwrap(), None)
    }

    #[test]
    fn defaults_without_config() {
        let s = merged(&["ifmon"], "").unwrap();
        assert_eq!(s.window, DEFAULT_WINDOW);
        assert_eq!(s.units, Units::default());
        assert_eq!(s.view, GraphView::Split);
        assert!(s.mouse);
        assert!(!s.show_all);
    }

    #[test]
    fn config_file_applies() {
        let s = merged(
            &["ifmon"],
            "interval = \"2s\"\nwindow = \"5m\"\nunits = \"bits\"\nall = true\nmouse = false\nview = \"packets\"\ninterface = \"en0\"",
        )
        .unwrap();
        assert_eq!(s.interval, Duration::from_secs(2));
        assert_eq!(s.window, Duration::from_secs(300));
        assert_eq!(s.units.mode, UnitMode::Bits);
        assert!(s.show_all);
        assert!(!s.mouse);
        assert_eq!(s.view, GraphView::Packets);
        assert_eq!(s.focus, vec!["en0"]);
    }

    #[test]
    fn flags_override_config() {
        let s = merged(
            &[
                "ifmon", "-n", "1s", "--bytes", "--theme", "nord", "-i", "eth1",
            ],
            "interval = \"2s\"\nunits = \"bits\"\ntheme = \"dracula\"\ninterface = \"en0\"",
        )
        .unwrap();
        assert_eq!(s.interval, Duration::from_secs(1));
        assert_eq!(s.units.mode, UnitMode::Bytes);
        assert_eq!(s.theme.name, "nord");
        assert_eq!(s.focus, vec!["eth1"]);
    }

    #[test]
    fn reports_bad_values() {
        assert!(merged(&["ifmon"], "interval = \"soon\"").is_err());
        assert!(merged(&["ifmon", "--theme", "nope"], "").is_err());
        assert!(merged(&["ifmon"], "[colors]\nrx = \"blurple\"").is_err());
    }

    #[test]
    fn history_covers_max_window_but_is_capped() {
        let s = Settings {
            interval: Duration::from_secs(1),
            ..Settings::default()
        };
        assert_eq!(s.history_capacity(), 1802);
        let fast = Settings {
            interval: Duration::from_millis(50),
            ..Settings::default()
        };
        assert_eq!(fast.history_capacity(), MAX_HISTORY_POINTS);
    }
}
