//! Human-friendly formatting of rates, sizes, counts and durations.

use std::fmt::Write as _;
use std::time::Duration;

use clap::ValueEnum;
use serde::{Deserialize, Serialize};

/// Whether throughput is shown in bytes or bits per second.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, ValueEnum, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UnitMode {
    #[default]
    Bytes,
    Bits,
}

/// Display settings for throughput and volume values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Units {
    pub mode: UnitMode,
    /// Use 1000-based byte prefixes (kB, MB) instead of 1024-based ones (KiB, MiB).
    pub si: bool,
}

const IEC_RATE: [&str; 6] = ["B/s", "KiB/s", "MiB/s", "GiB/s", "TiB/s", "PiB/s"];
const SI_RATE: [&str; 6] = ["B/s", "kB/s", "MB/s", "GB/s", "TB/s", "PB/s"];
const BIT_RATE: [&str; 6] = ["b/s", "kb/s", "Mb/s", "Gb/s", "Tb/s", "Pb/s"];
const IEC_SIZE: [&str; 6] = ["B", "KiB", "MiB", "GiB", "TiB", "PiB"];
const SI_SIZE: [&str; 6] = ["B", "kB", "MB", "GB", "TB", "PB"];
const COUNT: [&str; 6] = ["", "k", "M", "G", "T", "P"];

impl Units {
    pub const fn new(mode: UnitMode, si: bool) -> Self {
        Self { mode, si }
    }

    pub fn toggle_mode(&mut self) {
        self.mode = match self.mode {
            UnitMode::Bytes => UnitMode::Bits,
            UnitMode::Bits => UnitMode::Bytes,
        };
    }

    pub const fn label(self) -> &'static str {
        match (self.mode, self.si) {
            (UnitMode::Bits, _) => "bits",
            (UnitMode::Bytes, true) => "bytes (SI)",
            (UnitMode::Bytes, false) => "bytes",
        }
    }

    /// Format a throughput given in bytes per second, e.g. `12.3 MiB/s` or `98.4 Mb/s`.
    pub fn rate(self, bytes_per_sec: f64) -> String {
        match (self.mode, self.si) {
            (UnitMode::Bits, _) => scaled(bytes_per_sec * 8.0, 1000.0, &BIT_RATE),
            (UnitMode::Bytes, true) => scaled(bytes_per_sec, 1000.0, &SI_RATE),
            (UnitMode::Bytes, false) => scaled(bytes_per_sec, 1024.0, &IEC_RATE),
        }
    }

    /// A "nice" axis maximum for a byte rate, round in the units actually displayed.
    pub fn nice_axis(self, bytes_per_sec: f64) -> f64 {
        match (self.mode, self.si) {
            (UnitMode::Bits, _) => nice_ceil(bytes_per_sec * 8.0) / 8.0,
            (UnitMode::Bytes, true) => nice_ceil(bytes_per_sec),
            (UnitMode::Bytes, false) => {
                let v = bytes_per_sec.max(1.0);
                let scale = 1024f64.powi(v.log(1024.0).floor() as i32);
                nice_ceil(v / scale) * scale
            }
        }
    }

    /// Format a volume of data. Volumes are always shown in bytes.
    pub fn size(self, bytes: u64) -> String {
        if self.si {
            scaled(bytes as f64, 1000.0, &SI_SIZE)
        } else {
            scaled(bytes as f64, 1024.0, &IEC_SIZE)
        }
    }
}

/// Format a link speed reported in bits per second, e.g. `1 Gb/s`.
pub fn link_speed(bits_per_sec: u64) -> String {
    scaled(bits_per_sec as f64, 1000.0, &BIT_RATE)
}

/// Format a plain count with a metric suffix, e.g. `1.23k`.
pub fn count(value: f64) -> String {
    scaled(value, 1000.0, &COUNT).trim_end().to_string()
}

/// Scale `value` by `base` until it fits below it, keeping roughly three significant digits.
fn scaled(value: f64, base: f64, units: &[&str]) -> String {
    let value = if value.is_finite() {
        value.max(0.0)
    } else {
        0.0
    };
    let mut v = value;
    let mut idx = 0;
    while v >= base && idx < units.len() - 1 {
        v /= base;
        idx += 1;
    }
    let unit = units[idx];
    if idx == 0 {
        return format!("{v:.0} {unit}");
    }
    if v < 10.0 {
        format!("{v:.2} {unit}")
    } else if v < 100.0 {
        format!("{v:.1} {unit}")
    } else {
        format!("{v:.0} {unit}")
    }
}

/// Round `value` up to a "nice" axis bound (1, 2, 2.5, 5 × 10ⁿ) so labels are readable.
pub fn nice_ceil(value: f64) -> f64 {
    if !value.is_finite() || value <= 0.0 {
        return 1.0;
    }
    let exp = value.log10().floor();
    let magnitude = 10f64.powf(exp);
    let fraction = value / magnitude;
    let nice = [1.0, 2.0, 2.5, 5.0, 10.0]
        .into_iter()
        .find(|&n| fraction <= n + f64::EPSILON)
        .unwrap_or(10.0);
    nice * magnitude
}

/// Parse a duration such as `500ms`, `1s`, `1.5s`, `2m`, `1h`. A bare number means milliseconds.
pub fn parse_duration(input: &str) -> Result<Duration, String> {
    let s = input.trim();
    if s.is_empty() {
        return Err("empty duration".into());
    }
    let split = s
        .find(|c: char| !(c.is_ascii_digit() || c == '.'))
        .unwrap_or(s.len());
    let (num, unit) = s.split_at(split);
    let value: f64 = num
        .parse()
        .map_err(|_| format!("invalid duration '{input}' (try 500ms, 1s, 2m)"))?;
    let secs = match unit.trim().to_ascii_lowercase().as_str() {
        "" | "ms" => value / 1000.0,
        "s" | "sec" | "secs" => value,
        "m" | "min" | "mins" => value * 60.0,
        "h" | "hr" | "hrs" => value * 3600.0,
        other => {
            return Err(format!(
                "unknown duration unit '{other}' (use ms, s, m or h)"
            ));
        }
    };
    if !secs.is_finite() || secs < 0.0 {
        return Err(format!("invalid duration '{input}'"));
    }
    Ok(Duration::from_secs_f64(secs))
}

/// Compact duration label for axis/window display: `45s`, `2m`, `1m30s`, `1h`.
pub fn short_duration(d: Duration) -> String {
    let total = d.as_secs_f64().round() as u64;
    if total == 0 {
        return format!("{}ms", d.as_millis());
    }
    let (h, m, s) = (total / 3600, (total % 3600) / 60, total % 60);
    let mut out = String::new();
    if h > 0 {
        let _ = write!(out, "{h}h");
    }
    if m > 0 {
        let _ = write!(out, "{m}m");
    }
    if s > 0 || out.is_empty() {
        let _ = write!(out, "{s}s");
    }
    out
}

/// Clock-style elapsed time: `05:12` or `1:02:03`.
pub fn clock(d: Duration) -> String {
    let total = d.as_secs();
    let (h, m, s) = (total / 3600, (total % 3600) / 60, total % 60);
    if h > 0 {
        format!("{h}:{m:02}:{s:02}")
    } else {
        format!("{m:02}:{s:02}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const IEC: Units = Units::new(UnitMode::Bytes, false);
    const SI: Units = Units::new(UnitMode::Bytes, true);
    const BITS: Units = Units::new(UnitMode::Bits, false);

    #[test]
    fn formats_rates() {
        assert_eq!(IEC.rate(0.0), "0 B/s");
        assert_eq!(IEC.rate(512.0), "512 B/s");
        assert_eq!(IEC.rate(1536.0), "1.50 KiB/s");
        assert_eq!(IEC.rate(50.0 * 1024.0 * 1024.0), "50.0 MiB/s");
        assert_eq!(SI.rate(250_000.0), "250 kB/s");
        assert_eq!(BITS.rate(125_000.0), "1.00 Mb/s");
    }

    #[test]
    fn rates_handle_garbage() {
        assert_eq!(IEC.rate(f64::NAN), "0 B/s");
        assert_eq!(IEC.rate(-5.0), "0 B/s");
    }

    #[test]
    fn formats_sizes_and_counts() {
        assert_eq!(IEC.size(1023), "1023 B");
        assert_eq!(IEC.size(3 * 1024 * 1024 * 1024), "3.00 GiB");
        assert_eq!(SI.size(1_500_000), "1.50 MB");
        assert_eq!(count(42.0), "42");
        assert_eq!(count(12_345.0), "12.3 k");
        assert_eq!(link_speed(1_000_000_000), "1.00 Gb/s");
    }

    #[test]
    fn toggles_mode() {
        let mut u = IEC;
        u.toggle_mode();
        assert_eq!(u.mode, UnitMode::Bits);
        u.toggle_mode();
        assert_eq!(u.mode, UnitMode::Bytes);
    }

    #[test]
    fn nice_ceilings() {
        assert_eq!(nice_ceil(0.0), 1.0);
        assert_eq!(nice_ceil(1.0), 1.0);
        assert_eq!(nice_ceil(1.3), 2.0);
        assert_eq!(nice_ceil(2.2), 2.5);
        assert_eq!(nice_ceil(4.0), 5.0);
        assert_eq!(nice_ceil(730.0), 1000.0);
        assert_eq!(nice_ceil(1_200_000.0), 2_000_000.0);
    }

    #[test]
    fn nice_axis_is_round_in_display_units() {
        assert_eq!(IEC.rate(IEC.nice_axis(3.3 * 1024.0 * 1024.0)), "5.00 MiB/s");
        assert_eq!(SI.rate(SI.nice_axis(730_000.0)), "1.00 MB/s");
        assert_eq!(BITS.rate(BITS.nice_axis(1_300_000.0)), "20.0 Mb/s");
        assert_eq!(IEC.nice_axis(0.0), 1.0);
    }

    #[test]
    fn parses_durations() {
        assert_eq!(parse_duration("250").unwrap(), Duration::from_millis(250));
        assert_eq!(parse_duration("250ms").unwrap(), Duration::from_millis(250));
        assert_eq!(parse_duration("1.5s").unwrap(), Duration::from_millis(1500));
        assert_eq!(parse_duration(" 2m ").unwrap(), Duration::from_secs(120));
        assert_eq!(parse_duration("1h").unwrap(), Duration::from_secs(3600));
        assert!(parse_duration("").is_err());
        assert!(parse_duration("abc").is_err());
        assert!(parse_duration("5 parsecs").is_err());
    }

    #[test]
    fn formats_durations() {
        assert_eq!(short_duration(Duration::from_secs(45)), "45s");
        assert_eq!(short_duration(Duration::from_secs(120)), "2m");
        assert_eq!(short_duration(Duration::from_secs(90)), "1m30s");
        assert_eq!(short_duration(Duration::from_millis(250)), "250ms");
        assert_eq!(clock(Duration::from_secs(312)), "05:12");
        assert_eq!(clock(Duration::from_secs(3723)), "1:02:03");
    }
}
