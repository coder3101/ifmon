use std::collections::VecDeque;

/// Minimum scale so graph and max never collapse to zero for an empty/flat window.
const MIN_SCALE: f64 = 1.0;

/// History of network speed measurements over time.
///
/// Implemented as a fixed-capacity ring buffer ([`VecDeque`]) so pushing a sample
/// and evicting the oldest one are both O(1).
#[derive(Debug, Clone, Default)]
pub struct SpeedHistory {
    pub rx_speeds: VecDeque<f64>,
    pub tx_speeds: VecDeque<f64>,
    pub timestamps: VecDeque<f64>,
}

impl SpeedHistory {
    /// Create a new empty speed history.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a new speed measurement, evicting the oldest sample when over capacity.
    pub fn push(&mut self, rx: f64, tx: f64, max_size: usize) {
        let timestamp = self.timestamps.back().copied().unwrap_or(-1.0) + 1.0;

        self.rx_speeds.push_back(rx);
        self.tx_speeds.push_back(tx);
        self.timestamps.push_back(timestamp);

        while self.rx_speeds.len() > max_size {
            self.rx_speeds.pop_front();
            self.tx_speeds.pop_front();
            self.timestamps.pop_front();
        }
    }

    /// Peak RX speed over the stored window (`>= MIN_SCALE` to avoid a dead axis).
    pub fn max_rx(&self) -> f64 {
        self.rx_speeds.iter().copied().fold(MIN_SCALE, f64::max)
    }

    /// Peak TX speed over the stored window (`>= MIN_SCALE` to avoid a dead axis).
    pub fn max_tx(&self) -> f64 {
        self.tx_speeds.iter().copied().fold(MIN_SCALE, f64::max)
    }

    /// Reset the history to empty.
    pub fn clear(&mut self) {
        self.rx_speeds.clear();
        self.tx_speeds.clear();
        self.timestamps.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn respects_capacity() {
        let mut h = SpeedHistory::new();
        for i in 0..10 {
            h.push(i as f64, i as f64, 5);
        }
        assert_eq!(h.rx_speeds.len(), 5);
        // Oldest 5 samples evicted.
        assert_eq!(*h.rx_speeds.front().unwrap(), 5.0);
        assert_eq!(*h.rx_speeds.back().unwrap(), 9.0);
    }

    #[test]
    fn timestamps_are_sequential() {
        let mut h = SpeedHistory::new();
        h.push(1.0, 1.0, 10);
        h.push(2.0, 2.0, 10);
        assert_eq!(h.timestamps, VecDeque::from([0.0, 1.0]));
    }

    #[test]
    fn max_reflects_window_not_all_time_history() {
        let mut h = SpeedHistory::new();
        // A single early spike should not pin the max forever.
        h.push(1000.0, 1.0, 3);
        h.push(1.0, 1.0, 3);
        h.push(1.0, 1.0, 3);
        h.push(1.0, 1.0, 3); // evicts the 1000.0 sample
        assert_eq!(h.max_rx(), 1.0);
    }

    #[test]
    fn max_never_below_min_scale() {
        let mut h = SpeedHistory::new();
        h.push(0.0, 0.0, 10);
        assert_eq!(h.max_rx(), MIN_SCALE);
        assert_eq!(h.max_tx(), MIN_SCALE);
    }

    #[test]
    fn empty_history_has_min_scale_max() {
        let h = SpeedHistory::new();
        assert!(h.rx_speeds.is_empty());
        assert_eq!(h.max_rx(), MIN_SCALE);
        assert_eq!(h.max_tx(), MIN_SCALE);
    }

    #[test]
    fn clear_resets_everything() {
        let mut h = SpeedHistory::new();
        h.push(5.0, 7.0, 10);
        h.clear();
        assert!(h.rx_speeds.is_empty());
        assert_eq!(h.max_rx(), MIN_SCALE);
    }
}
