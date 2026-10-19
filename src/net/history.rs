use std::collections::VecDeque;

/// One rate sample. `t` is seconds since the monitor started.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Point {
    pub t: f64,
    pub rx: f64,
    pub tx: f64,
    pub rx_packets: f64,
    pub tx_packets: f64,
}

/// Fixed-capacity ring buffer of rate samples.
#[derive(Debug, Clone)]
pub struct History {
    points: VecDeque<Point>,
    capacity: usize,
}

impl History {
    pub fn new(capacity: usize) -> Self {
        let capacity = capacity.max(2);
        Self {
            points: VecDeque::with_capacity(capacity),
            capacity,
        }
    }

    pub fn push(&mut self, point: Point) {
        if self.points.len() == self.capacity {
            self.points.pop_front();
        }
        self.points.push_back(point);
    }

    pub fn clear(&mut self) {
        self.points.clear();
    }

    pub fn len(&self) -> usize {
        self.points.len()
    }

    pub fn is_empty(&self) -> bool {
        self.points.is_empty()
    }

    pub fn iter(&self) -> impl DoubleEndedIterator<Item = &Point> + ExactSizeIterator {
        self.points.iter()
    }

    /// Samples taken at or after `t0`, oldest first.
    pub fn since(&self, t0: f64) -> impl DoubleEndedIterator<Item = &Point> {
        let start = self.points.partition_point(|p| p.t < t0);
        self.points.range(start..)
    }

    /// The `n` most recent samples, oldest first.
    pub fn recent(&self, n: usize) -> impl Iterator<Item = &Point> {
        self.points.iter().skip(self.points.len().saturating_sub(n))
    }
}

/// Descriptive statistics over a set of samples.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Summary {
    pub min: f64,
    pub max: f64,
    pub avg: f64,
    pub p95: f64,
    pub count: usize,
}

impl Summary {
    pub fn from_values(values: impl IntoIterator<Item = f64>) -> Self {
        let mut sorted: Vec<f64> = values.into_iter().filter(|v| v.is_finite()).collect();
        if sorted.is_empty() {
            return Self::default();
        }
        sorted.sort_by(f64::total_cmp);
        let count = sorted.len();
        // Nearest-rank percentile.
        let rank = ((0.95 * count as f64).ceil() as usize).clamp(1, count);
        Self {
            min: sorted[0],
            max: sorted[count - 1],
            avg: sorted.iter().sum::<f64>() / count as f64,
            p95: sorted[rank - 1],
            count,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pt(t: f64, rx: f64) -> Point {
        Point {
            t,
            rx,
            tx: rx * 2.0,
            ..Point::default()
        }
    }

    #[test]
    fn evicts_oldest_beyond_capacity() {
        let mut h = History::new(3);
        for i in 0..5 {
            h.push(pt(f64::from(i), f64::from(i)));
        }
        assert_eq!(h.len(), 3);
        let ts: Vec<f64> = h.iter().map(|p| p.t).collect();
        assert_eq!(ts, vec![2.0, 3.0, 4.0]);
    }

    #[test]
    fn since_and_recent_select_windows() {
        let mut h = History::new(10);
        for i in 0..6 {
            h.push(pt(f64::from(i), 0.0));
        }
        let since: Vec<f64> = h.since(3.5).map(|p| p.t).collect();
        assert_eq!(since, vec![4.0, 5.0]);
        let recent: Vec<f64> = h.recent(2).map(|p| p.t).collect();
        assert_eq!(recent, vec![4.0, 5.0]);
        assert_eq!(h.recent(100).count(), 6);
    }

    #[test]
    fn clear_empties() {
        let mut h = History::new(4);
        h.push(pt(0.0, 1.0));
        h.clear();
        assert!(h.is_empty());
    }

    #[test]
    fn summary_statistics() {
        let s = Summary::from_values((1..=100).map(f64::from));
        assert_eq!(s.min, 1.0);
        assert_eq!(s.max, 100.0);
        assert!((s.avg - 50.5).abs() < 1e-9);
        assert_eq!(s.p95, 95.0);
        assert_eq!(s.count, 100);
    }

    #[test]
    fn summary_of_nothing_is_zero() {
        assert_eq!(Summary::from_values(std::iter::empty()), Summary::default());
        let one = Summary::from_values([f64::NAN, 7.0]);
        assert_eq!((one.min, one.max, one.p95, one.count), (7.0, 7.0, 7.0, 1));
    }
}
