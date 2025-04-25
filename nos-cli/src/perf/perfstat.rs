use std::mem;
use std::time::Duration;

use anyhow::Result;

#[derive(Debug, Clone, serde::Serialize)]
pub struct PerfStat {
    /// Number of threads used.
    pub threads: usize,
    /// Number of ops recorded.
    pub ops: usize,
    /// Average throughput (ops/sec).
    pub throughput: u64,

    /// Average GET latency.
    #[serde(serialize_with = "crate::utils::serialize_duration")]
    pub get_avg: Duration,
    /// Median GET latency.
    #[serde(serialize_with = "crate::utils::serialize_duration")]
    pub get_p50: Duration,
    /// 99% GET latency.
    #[serde(serialize_with = "crate::utils::serialize_duration")]
    pub get_p99: Duration,
    /// 99.9% GET latency.
    #[serde(serialize_with = "crate::utils::serialize_duration")]
    pub get_p999: Duration,

    /// Average GET latency.
    #[serde(serialize_with = "crate::utils::serialize_duration")]
    pub put_avg: Duration,
    /// Median GET latency.
    #[serde(serialize_with = "crate::utils::serialize_duration")]
    pub put_p50: Duration,
    /// 99% GET latency.
    #[serde(serialize_with = "crate::utils::serialize_duration")]
    pub put_p99: Duration,
    /// 99.9% GET latency.
    #[serde(serialize_with = "crate::utils::serialize_duration")]
    pub put_p999: Duration,
}

impl PerfStat {
    /// Create a new `PerfStat` instance with all field zeroed.
    #[inline]
    pub fn new() -> Self {
        unsafe { mem::zeroed() }
    }

    /// Determine whether the stat data is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.ops == 0
    }

    /// Determine whether the stat is meaningless (too few samples).
    #[inline]
    pub fn is_meaningless(&self) -> bool {
        self.is_empty()
    }

    /// Dump the `PerfStat` instance into a string buffer.
    pub fn dump(&self) -> Result<String> {
        let mut wtr = csv::Writer::from_writer(vec![]);
        wtr.serialize(self)?;
        wtr.flush()?;

        String::from_utf8(wtr.into_inner()?).map_err(Into::into)
    }
}

impl Default for PerfStat {
    fn default() -> Self {
        Self::new()
    }
}
