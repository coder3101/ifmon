use std::fs::File;
use std::io::{self, BufWriter, Write};
use std::path::{Path, PathBuf};

use jiff::{SignedDuration, Timestamp, Zoned};

use crate::commands::watch::csv_field;
use crate::net::Track;

/// Write `track`'s history as CSV into `dir`. Returns the file path and number of rows.
pub fn write_csv(name: &str, track: &Track, now: f64, dir: &Path) -> io::Result<(PathBuf, usize)> {
    let safe: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    let stamp = Zoned::now().strftime("%Y%m%d-%H%M%S").to_string();
    let path = dir.join(format!("ifmon-{safe}-{stamp}.csv"));

    let wall_now = Timestamp::now();
    let mut out = BufWriter::new(File::create(&path)?);
    writeln!(
        out,
        "timestamp,interface,rx_bytes_per_sec,tx_bytes_per_sec,rx_packets_per_sec,tx_packets_per_sec"
    )?;
    let iface = csv_field(name);
    for p in track.history.iter() {
        let ago = SignedDuration::from_secs_f64((now - p.t).max(0.0));
        let ts = wall_now.checked_sub(ago).unwrap_or(wall_now);
        writeln!(
            out,
            "{ts},{iface},{:.3},{:.3},{:.3},{:.3}",
            p.rx, p.tx, p.rx_packets, p.tx_packets
        )?;
    }
    out.flush()?;
    Ok((path, track.history.len()))
}
