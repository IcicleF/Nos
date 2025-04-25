use std::sync::atomic::*;

/// Statistics for the object store of time spent on various types of tasks.
pub struct ObjStoreStats {
    pub samples: AtomicU64,
    pub coding_time: AtomicU64,
    pub repl_time: AtomicU64,
    pub kvs_time: AtomicU64,
    pub other_time: AtomicU64,
}

impl ObjStoreStats {
    /// Create an empty statistics instance.
    pub fn new() -> Self {
        Self {
            samples: AtomicU64::new(0),
            coding_time: AtomicU64::new(0),
            repl_time: AtomicU64::new(0),
            kvs_time: AtomicU64::new(0),
            other_time: AtomicU64::new(0),
        }
    }

    /// Add a sample.
    #[inline]
    pub fn record(&self, coding_time: u64, repl_time: u64, kvs_time: u64, other_time: u64) {
        self.samples.fetch_add(1, Ordering::Relaxed);
        self.coding_time.fetch_add(coding_time, Ordering::Relaxed);
        self.repl_time.fetch_add(repl_time, Ordering::Relaxed);
        self.kvs_time.fetch_add(kvs_time, Ordering::Relaxed);
        self.other_time.fetch_add(other_time, Ordering::Relaxed);
    }

    /// Report statistics.
    pub fn report(&self) {
        let samples = self.samples.load(Ordering::Relaxed);
        if samples == 0 {
            log::info!("STATS: no samples");
        } else {
            log::info!(
                "STATS: samples {}; coding = {:.2}, repl = {:.2}, kvs = {:.2}, other = {:.2}",
                samples,
                self.coding_time.load(Ordering::Relaxed) as f64 / samples as f64 / 1000.0,
                self.repl_time.load(Ordering::Relaxed) as f64 / samples as f64 / 1000.0,
                self.kvs_time.load(Ordering::Relaxed) as f64 / samples as f64 / 1000.0,
                self.other_time.load(Ordering::Relaxed) as f64 / samples as f64 / 1000.0,
            );
        }
    }
}

impl Default for ObjStoreStats {
    fn default() -> Self {
        Self::new()
    }
}
