use std::collections::HashMap;
use std::time::{Duration, Instant};

use netdev::Interface;

use super::counters::{self, Counters, Snapshot};
use super::history::{History, Point};

/// Per-second rates derived from two consecutive counter readings.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Rates {
    pub rx_bytes: f64,
    pub tx_bytes: f64,
    pub rx_packets: f64,
    pub tx_packets: f64,
    pub rx_errors: f64,
    pub tx_errors: f64,
    pub rx_drops: f64,
    pub tx_drops: f64,
}

impl Rates {
    fn from_delta(d: &Counters, secs: f64) -> Self {
        let per = |v: u64| v as f64 / secs;
        Self {
            rx_bytes: per(d.rx_bytes),
            tx_bytes: per(d.tx_bytes),
            rx_packets: per(d.rx_packets),
            tx_packets: per(d.tx_packets),
            rx_errors: per(d.rx_errors),
            tx_errors: per(d.tx_errors),
            rx_drops: per(d.rx_drops),
            tx_drops: per(d.tx_drops),
        }
    }
}

/// Everything tracked for a single interface across samples.
#[derive(Debug, Clone)]
pub struct Track {
    last: Option<(Counters, Instant)>,
    /// Latest raw OS counters (typically cumulative since boot).
    pub raw: Counters,
    /// Traffic observed since ifmon started (or the last reset).
    pub session: Counters,
    /// Most recent per-second rates.
    pub rate: Rates,
    /// Highest byte rates seen this session.
    pub peak_rx: f64,
    pub peak_tx: f64,
    pub history: History,
}

impl Track {
    fn new(capacity: usize) -> Self {
        Self {
            last: None,
            raw: Counters::default(),
            session: Counters::default(),
            rate: Rates::default(),
            peak_rx: 0.0,
            peak_tx: 0.0,
            history: History::new(capacity),
        }
    }

    fn reset(&mut self) {
        self.session = Counters::default();
        self.rate = Rates::default();
        self.peak_rx = 0.0;
        self.peak_tx = 0.0;
        self.history.clear();
    }
}

/// Samples interface counters and keeps rate history for every interface.
#[derive(Debug, Clone)]
pub struct Monitor {
    interfaces: Vec<Interface>,
    tracks: HashMap<String, Track>,
    epoch: Instant,
    /// Seconds since `epoch` of the latest sample.
    now: f64,
    capacity: usize,
    extended: bool,
}

/// Samples closer together than this are skipped to avoid noisy rates.
const MIN_SAMPLE_GAP: Duration = Duration::from_millis(20);

impl Monitor {
    /// Enumerate the system's interfaces and keep up to `capacity` samples each.
    pub fn new(capacity: usize) -> Self {
        Self::with_interfaces(netdev::get_interfaces(), capacity)
    }

    pub fn with_interfaces(interfaces: Vec<Interface>, capacity: usize) -> Self {
        Self {
            interfaces,
            tracks: HashMap::new(),
            epoch: Instant::now(),
            now: 0.0,
            capacity,
            extended: false,
        }
    }

    pub fn interfaces(&self) -> &[Interface] {
        &self.interfaces
    }

    pub fn track(&self, name: &str) -> Option<&Track> {
        self.tracks.get(name)
    }

    /// Seconds since start of the most recent sample (the graph's "now").
    pub fn now(&self) -> f64 {
        self.now
    }

    pub fn uptime(&self) -> Duration {
        self.epoch.elapsed()
    }

    /// Whether packet/error/drop counters are available.
    pub fn extended(&self) -> bool {
        self.extended
    }

    /// Re-enumerate interfaces so hot-plugged adapters and address changes show up.
    /// Returns `true` if the set of interface names changed.
    pub fn refresh_interfaces(&mut self) -> bool {
        self.replace_interfaces(netdev::get_interfaces())
    }

    pub fn replace_interfaces(&mut self, interfaces: Vec<Interface>) -> bool {
        let changed = interfaces.len() != self.interfaces.len()
            || interfaces
                .iter()
                .zip(&self.interfaces)
                .any(|(a, b)| a.name != b.name);
        self.tracks
            .retain(|name, _| interfaces.iter().any(|i| &i.name == name));
        self.interfaces = interfaces;
        changed
    }

    /// Read the OS counters and update rates and history.
    pub fn sample(&mut self) {
        let snapshot = counters::read(&mut self.interfaces);
        self.ingest(Instant::now(), &snapshot);
    }

    /// Fold a counter snapshot taken at `at` into the tracked state.
    pub fn ingest(&mut self, at: Instant, snapshot: &Snapshot) {
        self.extended = snapshot.extended;
        self.now = at.saturating_duration_since(self.epoch).as_secs_f64();
        for iface in &self.interfaces {
            let Some(current) = snapshot.counters.get(&iface.name) else {
                continue;
            };
            let track = self
                .tracks
                .entry(iface.name.clone())
                .or_insert_with(|| Track::new(self.capacity));

            if let Some((prev, t0)) = track.last {
                let elapsed = at.saturating_duration_since(t0);
                if elapsed < MIN_SAMPLE_GAP {
                    continue;
                }
                let delta = current.delta_since(&prev);
                let rate = Rates::from_delta(&delta, elapsed.as_secs_f64());
                track.session = track.session.saturating_add(&delta);
                track.rate = rate;
                track.peak_rx = track.peak_rx.max(rate.rx_bytes);
                track.peak_tx = track.peak_tx.max(rate.tx_bytes);
                track.history.push(Point {
                    t: self.now,
                    rx: rate.rx_bytes,
                    tx: rate.tx_bytes,
                    rx_packets: rate.rx_packets,
                    tx_packets: rate.tx_packets,
                });
            }
            track.raw = *current;
            track.last = Some((*current, at));
        }
    }

    /// Forget session totals, peaks and history for every interface.
    pub fn reset(&mut self) {
        for track in self.tracks.values_mut() {
            track.reset();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn iface(name: &str) -> Interface {
        let mut i = Interface::dummy();
        i.name = name.into();
        i
    }

    fn snap(entries: &[(&str, u64, u64)]) -> Snapshot {
        Snapshot {
            counters: entries
                .iter()
                .map(|&(n, rx, tx)| {
                    (
                        n.to_string(),
                        Counters {
                            rx_bytes: rx,
                            tx_bytes: tx,
                            rx_packets: rx / 100,
                            ..Counters::default()
                        },
                    )
                })
                .collect(),
            extended: true,
        }
    }

    #[test]
    fn computes_rates_totals_and_history() {
        let mut m = Monitor::with_interfaces(vec![iface("eth0")], 16);
        let t0 = Instant::now();
        m.ingest(t0, &snap(&[("eth0", 1_000, 500)]));
        let track = m.track("eth0").unwrap();
        assert!(track.history.is_empty(), "first sample is only a baseline");
        assert_eq!(track.raw.rx_bytes, 1_000);

        m.ingest(
            t0 + Duration::from_secs(2),
            &snap(&[("eth0", 5_000, 1_500)]),
        );
        let track = m.track("eth0").unwrap();
        assert_eq!(track.rate.rx_bytes, 2_000.0);
        assert_eq!(track.rate.tx_bytes, 500.0);
        assert_eq!(track.rate.rx_packets, 20.0);
        assert_eq!(track.session.rx_bytes, 4_000);
        assert_eq!(track.history.len(), 1);
        assert_eq!(track.peak_rx, 2_000.0);
        assert!(m.extended());
    }

    #[test]
    fn peak_is_sticky_and_reset_clears() {
        let mut m = Monitor::with_interfaces(vec![iface("eth0")], 16);
        let t0 = Instant::now();
        m.ingest(t0, &snap(&[("eth0", 0, 0)]));
        m.ingest(t0 + Duration::from_secs(1), &snap(&[("eth0", 10_000, 0)]));
        m.ingest(t0 + Duration::from_secs(2), &snap(&[("eth0", 10_100, 0)]));
        let track = m.track("eth0").unwrap();
        assert_eq!(track.rate.rx_bytes, 100.0);
        assert_eq!(track.peak_rx, 10_000.0);

        m.reset();
        let track = m.track("eth0").unwrap();
        assert_eq!(track.peak_rx, 0.0);
        assert_eq!(track.session, Counters::default());
        assert!(track.history.is_empty());
    }

    #[test]
    fn ignores_samples_that_are_too_close() {
        let mut m = Monitor::with_interfaces(vec![iface("eth0")], 16);
        let t0 = Instant::now();
        m.ingest(t0, &snap(&[("eth0", 0, 0)]));
        m.ingest(t0 + Duration::from_millis(1), &snap(&[("eth0", 1_000, 0)]));
        assert!(m.track("eth0").unwrap().history.is_empty());
    }

    #[test]
    fn replacing_interfaces_drops_vanished_tracks() {
        let mut m = Monitor::with_interfaces(vec![iface("eth0"), iface("wlan0")], 16);
        m.ingest(Instant::now(), &snap(&[("eth0", 0, 0), ("wlan0", 0, 0)]));
        assert!(m.replace_interfaces(vec![iface("eth0")]));
        assert!(m.track("eth0").is_some());
        assert!(m.track("wlan0").is_none());
        assert!(!m.replace_interfaces(vec![iface("eth0")]));
    }
}
