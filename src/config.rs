//! TOML configuration file support.

use std::path::{Path, PathBuf};

use color_eyre::eyre::{Context, Result, bail};
use serde::{Deserialize, Serialize};

use crate::settings::{GraphStyle, GraphView};
use crate::theme::ColorOverrides;
use crate::units::UnitMode;

/// Settings that can be persisted in `config.toml`. Every field is optional.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct FileConfig {
    pub interval: Option<String>,
    pub window: Option<String>,
    pub units: Option<UnitMode>,
    pub si: Option<bool>,
    pub all: Option<bool>,
    pub theme: Option<String>,
    pub view: Option<GraphView>,
    pub style: Option<GraphStyle>,
    pub mouse: Option<bool>,
    pub interface: Option<String>,
    pub colors: ColorOverrides,
}

pub const DEFAULT_CONFIG: &str = r##"# ifmon configuration
# Every setting is optional; command-line flags take precedence over this file.

# How often to sample counters (e.g. "250ms", "1s").
interval = "500ms"

# Visible time window of the graphs (zoom with +/- in the dashboard).
window = "1m"

# "bytes" or "bits" per second.
units = "bytes"

# Use 1000-based units (kB, MB) instead of 1024-based ones (KiB, MiB).
si = false

# Show loopback, tunnel and virtual interfaces too.
all = false

# Color theme: default, dracula, nord, gruvbox, catppuccin, tokyonight, solarized, mono.
theme = "default"

# Graph layout: "split", "combined" or "packets".
view = "split"

# Graph style: "braille", "block", "bars" or "dots".
style = "braille"

# Enable mouse support (click to select, wheel to scroll/zoom).
mouse = true

# Interface to select on startup.
# interface = "en0"

# Override individual theme colors with names ("cyan") or hex ("#89b4fa").
[colors]
# rx = "#89b4fa"
# tx = "#a6e3a1"
"##;

/// Where a new config file should be created.
pub fn default_path() -> Option<PathBuf> {
    candidate_paths().into_iter().next()
}

/// Config locations in priority order: `$XDG_CONFIG_HOME`, `~/.config`, then the platform dir.
fn candidate_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    if let Some(xdg) = std::env::var_os("XDG_CONFIG_HOME").filter(|v| !v.is_empty()) {
        paths.push(PathBuf::from(xdg).join("ifmon").join("config.toml"));
    }
    if cfg!(unix)
        && let Some(home) = dirs::home_dir()
    {
        paths.push(home.join(".config").join("ifmon").join("config.toml"));
    }
    if let Some(dir) = dirs::config_dir() {
        paths.push(dir.join("ifmon").join("config.toml"));
    }
    paths.dedup();
    paths
}

/// Load the config. An explicit path must exist; default locations are optional.
pub fn load(explicit: Option<&Path>) -> Result<(FileConfig, Option<PathBuf>)> {
    if let Some(path) = explicit {
        return Ok((read(path)?, Some(path.to_path_buf())));
    }
    for path in candidate_paths() {
        if path.is_file() {
            return Ok((read(&path)?, Some(path)));
        }
    }
    Ok((FileConfig::default(), None))
}

fn read(path: &Path) -> Result<FileConfig> {
    let text = std::fs::read_to_string(path)
        .wrap_err_with(|| format!("failed to read config file {}", path.display()))?;
    parse(&text).wrap_err_with(|| format!("invalid config file {}", path.display()))
}

pub fn parse(text: &str) -> Result<FileConfig> {
    Ok(toml::from_str(text)?)
}

/// Write the default config to `path`.
pub fn init(path: &Path, force: bool) -> Result<()> {
    if path.exists() && !force {
        bail!(
            "{} already exists (use --force to overwrite)",
            path.display()
        );
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .wrap_err_with(|| format!("failed to create {}", parent.display()))?;
    }
    std::fs::write(path, DEFAULT_CONFIG)
        .wrap_err_with(|| format!("failed to write {}", path.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_parses() {
        let cfg = parse(DEFAULT_CONFIG).unwrap();
        assert_eq!(cfg.interval.as_deref(), Some("500ms"));
        assert_eq!(cfg.units, Some(UnitMode::Bytes));
        assert_eq!(cfg.view, Some(GraphView::Split));
        assert_eq!(cfg.style, Some(GraphStyle::Braille));
        assert_eq!(cfg.theme.as_deref(), Some("default"));
    }

    #[test]
    fn empty_config_is_all_defaults() {
        assert_eq!(parse("").unwrap(), FileConfig::default());
    }

    #[test]
    fn rejects_unknown_keys() {
        assert!(parse("intervall = \"1s\"").is_err());
        assert!(parse("[colors]\nfoo = \"red\"").is_err());
    }

    #[test]
    fn parses_colors_and_enums() {
        let cfg =
            parse("units = \"bits\"\nview = \"combined\"\n[colors]\nrx = \"#ff0000\"").unwrap();
        assert_eq!(cfg.units, Some(UnitMode::Bits));
        assert_eq!(cfg.view, Some(GraphView::Combined));
        assert_eq!(cfg.colors.rx.as_deref(), Some("#ff0000"));
    }

    #[test]
    fn init_refuses_to_clobber() {
        let dir = std::env::temp_dir().join(format!("ifmon-test-{}", std::process::id()));
        let path = dir.join("nested").join("config.toml");
        init(&path, false).unwrap();
        assert!(init(&path, false).is_err());
        init(&path, true).unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), DEFAULT_CONFIG);
        let _ = std::fs::remove_dir_all(dir);
    }
}
