use std::fmt::Write as _;
use std::io::{self, Write};
use std::time::{Duration, Instant};

use color_eyre::eyre::Result;
use serde::Serialize;

use crate::cli::{OutputFormat, WatchArgs};
use crate::net::{Monitor, Rates, Track, display_name};
use crate::settings::Settings;
use crate::units::{self, Units};

/// How often the interface list is re-enumerated while streaming.
const REFRESH_EVERY: Duration = Duration::from_secs(10);

#[derive(Debug, Serialize)]
struct Row<'a> {
    timestamp: &'a str,
    interface: &'a str,
    rx_bytes_per_sec: f64,
    tx_bytes_per_sec: f64,
    rx_packets_per_sec: f64,
    tx_packets_per_sec: f64,
    rx_errors_per_sec: f64,
    tx_errors_per_sec: f64,
    rx_drops_per_sec: f64,
    tx_drops_per_sec: f64,
    rx_session_bytes: u64,
    tx_session_bytes: u64,
}

const CSV_HEADER: &str = "timestamp,interface,rx_bytes_per_sec,tx_bytes_per_sec,rx_packets_per_sec,tx_packets_per_sec,rx_errors_per_sec,tx_errors_per_sec,rx_drops_per_sec,tx_drops_per_sec,rx_session_bytes,tx_session_bytes";

fn round3(v: f64) -> f64 {
    (v * 1000.0).round() / 1000.0
}

impl<'a> Row<'a> {
    fn new(timestamp: &'a str, interface: &'a str, rate: &Rates, session: (u64, u64)) -> Self {
        Self {
            timestamp,
            interface,
            rx_bytes_per_sec: round3(rate.rx_bytes),
            tx_bytes_per_sec: round3(rate.tx_bytes),
            rx_packets_per_sec: round3(rate.rx_packets),
            tx_packets_per_sec: round3(rate.tx_packets),
            rx_errors_per_sec: round3(rate.rx_errors),
            tx_errors_per_sec: round3(rate.tx_errors),
            rx_drops_per_sec: round3(rate.rx_drops),
            tx_drops_per_sec: round3(rate.tx_drops),
            rx_session_bytes: session.0,
            tx_session_bytes: session.1,
        }
    }

    fn csv(&self) -> String {
        format!(
            "{},{},{},{},{},{},{},{},{},{},{},{}",
            self.timestamp,
            csv_field(self.interface),
            self.rx_bytes_per_sec,
            self.tx_bytes_per_sec,
            self.rx_packets_per_sec,
            self.tx_packets_per_sec,
            self.rx_errors_per_sec,
            self.tx_errors_per_sec,
            self.rx_drops_per_sec,
            self.tx_drops_per_sec,
            self.rx_session_bytes,
            self.tx_session_bytes,
        )
    }

    fn text(&self, units: Units, name_width: usize, extended: bool) -> String {
        let mut line = format!(
            "{:<8}  {:<name_width$}  {:>11}  {:>11}",
            self.timestamp,
            self.interface,
            units.rate(self.rx_bytes_per_sec),
            units.rate(self.tx_bytes_per_sec),
        );
        if extended {
            let _ = write!(
                line,
                "  {:>9}  {:>9}",
                units::count(self.rx_packets_per_sec),
                units::count(self.tx_packets_per_sec)
            );
        }
        let _ = write!(
            line,
            "  {:>10}  {:>10}",
            units.size(self.rx_session_bytes),
            units.size(self.tx_session_bytes)
        );
        line
    }
}

pub(crate) fn csv_field(s: &str) -> String {
    if s.contains([',', '"', '\n']) {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

pub fn run(settings: &Settings, args: &WatchArgs) -> Result<()> {
    super::quiet_broken_pipe(stream(settings, args))
}

fn stream(settings: &Settings, args: &WatchArgs) -> io::Result<()> {
    let mut monitor = Monitor::new(2);
    // Validate the requested interfaces up front so typos fail fast.
    let initial = super::select(monitor.interfaces(), &settings.focus, settings.show_all)
        .map_err(|e| io::Error::new(io::ErrorKind::NotFound, e.to_string()))?;
    let name_width = initial
        .iter()
        .map(|i| display_name(i).len())
        .max()
        .unwrap_or(9)
        .max(if args.total { 9 } else { 5 });

    monitor.sample();

    let mut out = io::stdout().lock();
    let header_wanted = !args.no_header;
    match args.format {
        OutputFormat::Csv if header_wanted => writeln!(out, "{CSV_HEADER}")?,
        OutputFormat::Text if header_wanted => {
            let mut h = format!(
                "{:<8}  {:<name_width$}  {:>11}  {:>11}",
                "TIME", "INTERFACE", "RX", "TX"
            );
            if monitor.extended() {
                let _ = write!(h, "  {:>9}  {:>9}", "RX PKT/S", "TX PKT/S");
            }
            let _ = write!(h, "  {:>10}  {:>10}", "RX TOTAL", "TX TOTAL");
            writeln!(out, "{h}")?;
        }
        _ => {}
    }
    out.flush()?;

    let mut next = Instant::now() + settings.interval;
    let mut last_refresh = Instant::now();
    let mut emitted = 0u64;

    loop {
        std::thread::sleep(next.saturating_duration_since(Instant::now()));
        monitor.sample();
        let now = Instant::now();
        next += settings.interval;
        if next < now {
            next = now + settings.interval;
        }
        if last_refresh.elapsed() >= REFRESH_EVERY {
            monitor.refresh_interfaces();
            last_refresh = now;
        }

        let timestamp = match args.format {
            OutputFormat::Text => jiff::Zoned::now().strftime("%H:%M:%S").to_string(),
            _ => jiff::Timestamp::now().to_string(),
        };
        let selected = super::select(monitor.interfaces(), &settings.focus, settings.show_all)
            .unwrap_or_default();
        let tracks: Vec<(&str, &Track)> = selected
            .iter()
            .filter_map(|i| Some((display_name(i), monitor.track(&i.name)?)))
            .collect();

        let mut rows: Vec<Row> = tracks
            .iter()
            .map(|(name, t)| {
                Row::new(
                    &timestamp,
                    name,
                    &t.rate,
                    (t.session.rx_bytes, t.session.tx_bytes),
                )
            })
            .collect();
        if args.total && !tracks.is_empty() {
            let total = tracks.iter().fold(Rates::default(), |acc, (_, t)| Rates {
                rx_bytes: acc.rx_bytes + t.rate.rx_bytes,
                tx_bytes: acc.tx_bytes + t.rate.tx_bytes,
                rx_packets: acc.rx_packets + t.rate.rx_packets,
                tx_packets: acc.tx_packets + t.rate.tx_packets,
                rx_errors: acc.rx_errors + t.rate.rx_errors,
                tx_errors: acc.tx_errors + t.rate.tx_errors,
                rx_drops: acc.rx_drops + t.rate.rx_drops,
                tx_drops: acc.tx_drops + t.rate.tx_drops,
            });
            let session = tracks.iter().fold((0u64, 0u64), |acc, (_, t)| {
                (
                    acc.0.saturating_add(t.session.rx_bytes),
                    acc.1.saturating_add(t.session.tx_bytes),
                )
            });
            rows.push(Row::new(&timestamp, "TOTAL", &total, session));
        }

        for row in &rows {
            match args.format {
                OutputFormat::Text => writeln!(
                    out,
                    "{}",
                    row.text(settings.units, name_width, monitor.extended())
                )?,
                OutputFormat::Json => {
                    serde_json::to_writer(&mut out, row)?;
                    writeln!(out)?;
                }
                OutputFormat::Csv => writeln!(out, "{}", row.csv())?,
            }
        }
        out.flush()?;

        emitted += 1;
        if args.count.is_some_and(|c| emitted >= c) {
            return Ok(());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row() -> Row<'static> {
        let rate = Rates {
            rx_bytes: 1_536.123_456,
            tx_bytes: 10.0,
            rx_packets: 3.0,
            ..Rates::default()
        };
        Row::new("2026-10-20T00:00:00Z", "eth0", &rate, (4096, 128))
    }

    #[test]
    fn csv_row_matches_header() {
        let line = row().csv();
        assert_eq!(line.split(',').count(), CSV_HEADER.split(',').count());
        assert!(line.starts_with("2026-10-20T00:00:00Z,eth0,1536.123,10,3,"));
    }

    #[test]
    fn csv_quotes_awkward_names() {
        assert_eq!(csv_field("plain"), "plain");
        assert_eq!(csv_field("a,b"), "\"a,b\"");
        assert_eq!(csv_field("say \"hi\""), "\"say \"\"hi\"\"\"");
    }

    #[test]
    fn json_row_has_stable_keys() {
        let v: serde_json::Value = serde_json::to_value(row()).unwrap();
        assert_eq!(v["interface"], "eth0");
        assert_eq!(v["rx_bytes_per_sec"], 1536.123);
        assert_eq!(v["rx_session_bytes"], 4096);
    }

    #[test]
    fn text_row_is_human_readable() {
        let line = row().text(Units::default(), 6, true);
        assert!(line.contains("eth0"));
        assert!(line.contains("1.50 KiB/s"));
        assert!(line.contains("4.00 KiB"));
    }
}
