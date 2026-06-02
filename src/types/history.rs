/// History of network speed measurements over time
#[derive(Debug, Clone)]
pub struct SpeedHistory {
    pub rx_speeds: Vec<f64>,
    pub tx_speeds: Vec<f64>,
    pub timestamps: Vec<f64>,
    pub max_rx: f64,
    pub max_tx: f64,
}

impl SpeedHistory {
    /// Create a new empty speed history
    pub fn new(capacity: usize) -> Self {
        Self {
            rx_speeds: Vec::with_capacity(capacity),
            tx_speeds: Vec::with_capacity(capacity),
            timestamps: Vec::with_capacity(capacity),
            max_rx: 1.0,
            max_tx: 1.0,
        }
    }

    /// Add a new speed measurement to the history
    pub fn push(&mut self, rx: f64, tx: f64, max_size: usize) {
        self.rx_speeds.push(rx);
        self.tx_speeds.push(tx);

        let timestamp = if self.timestamps.is_empty() {
            0.0
        } else {
            self.timestamps.last().unwrap() + 1.0
        };
        self.timestamps.push(timestamp);

        if self.rx_speeds.len() > max_size {
            self.rx_speeds.remove(0);
            self.tx_speeds.remove(0);
            self.timestamps.remove(0);
        }

        self.max_rx = self.max_rx.max(rx).max(1.0);
        self.max_tx = self.max_tx.max(tx).max(1.0);
    }

    /// Reset the history
    pub fn clear(&mut self) {
        self.rx_speeds.clear();
        self.tx_speeds.clear();
        self.timestamps.clear();
        self.max_rx = 1.0;
        self.max_tx = 1.0;
    }
}
