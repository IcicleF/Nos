mod op;
mod perfstat;

use std::ops::{Deref, DerefMut};
use std::sync::atomic::{AtomicPtr, AtomicUsize, Ordering};
use std::time::Duration;
use std::{mem, ptr};

use crate::utils::ClAligned;
use crossbeam::atomic::AtomicCell;
pub use op::*;
pub use perfstat::*;
use quanta::Instant;

struct PerTypeLatencies {
    get: Vec<Duration>,
    put: Vec<Duration>,
}

impl PerTypeLatencies {
    fn new() -> Self {
        const DEFAULT_CAP: usize = 65536;
        Self {
            get: Vec::with_capacity(DEFAULT_CAP),
            put: Vec::with_capacity(DEFAULT_CAP),
        }
    }
}

struct Workset {
    latencies: Vec<ClAligned<PerTypeLatencies>>,
    throughputs: Vec<ClAligned<usize>>,

    unsampled: AtomicUsize,
    accessing: AtomicUsize,
}

impl Workset {
    const SAMPLE_RATE: usize = 100;

    fn new(num_threads: usize) -> Self {
        Self {
            latencies: (0..num_threads)
                .map(|_| ClAligned::new(PerTypeLatencies::new()))
                .collect(),
            throughputs: (0..num_threads).map(|_| ClAligned::new(0)).collect(),
            unsampled: AtomicUsize::new(0),
            accessing: AtomicUsize::new(0),
        }
    }

    fn push(&mut self, tid: usize, op_type: OpType, lat: Duration) {
        self.accessing.fetch_add(1, Ordering::SeqCst);
        *self.throughputs[tid] += 1;

        // Sample the results to prevent too much latency data from being recorded.
        if self.unsampled.fetch_add(1, Ordering::Relaxed) % Self::SAMPLE_RATE == 0 {
            match op_type {
                OpType::Get => (*self.latencies[tid]).get.push(lat),
                OpType::Put => (*self.latencies[tid]).put.push(lat),
            };
        }
        self.accessing.fetch_sub(1, Ordering::SeqCst);
    }

    fn take(&mut self) -> WorksetStat {
        let latencies = self
            .latencies
            .iter_mut()
            .map(|l| {
                let mut ptl = PerTypeLatencies::new();
                mem::swap(l.deref_mut(), &mut ptl);
                ptl
            })
            .collect::<Vec<_>>();
        let throughputs = self
            .throughputs
            .iter()
            .map(|t| *(t.deref()))
            .collect::<Vec<_>>();

        WorksetStat {
            latencies,
            throughputs,
        }
    }
}

struct WorksetStat {
    latencies: Vec<PerTypeLatencies>,
    throughputs: Vec<usize>,
}

/// Performance recorder.
pub struct Perfmon {
    // Points to a `*mut Workset` from `Box::leak`.
    workset: AtomicPtr<Workset>,
    last_wakeup: AtomicCell<Instant>,
    num_threads: usize,
}

impl Perfmon {
    /// Create a new performance monitor with the given maximum number of threads.
    pub fn new(num_threads: usize) -> Self {
        Self {
            workset: AtomicPtr::new(Box::leak(Box::new(Workset::new(num_threads)))),
            last_wakeup: AtomicCell::new(Instant::now()),
            num_threads,
        }
    }

    /// Push a record into the current workset.
    pub fn push(&self, tid: usize, op_type: OpType, lat: Duration) {
        let workset = unsafe { self.workset.load(Ordering::Relaxed).as_mut().unwrap() };
        workset.push(tid, op_type, lat);
    }

    /// Report the current statistics and reset the workset.
    pub fn stat(&self) -> PerfStat {
        // Switch to another workset and stat the current one.
        let workset = {
            let new_workset = Box::leak(Box::new(Workset::new(self.num_threads)));
            let old_workset = self.workset.swap(new_workset, Ordering::SeqCst);

            // Wait until no thread is accessing the old workset.
            let accessing = unsafe { ptr::addr_of!((*old_workset).accessing).as_ref() }.unwrap();
            while accessing.load(Ordering::Acquire) > 0 {
                std::hint::spin_loop();
            }
            unsafe { old_workset.as_mut().unwrap() }
        };
        let duration = {
            let now = Instant::now();
            let last_wakeup = self.last_wakeup.swap(now);
            now - last_wakeup
        };
        let ws_stat = workset.take();
        let _ = unsafe { Box::from_raw(workset) };

        let mut stat = PerfStat::new();
        stat.threads = ws_stat.throughputs.iter().filter(|t| **t > 0).count();
        stat.ops = ws_stat.throughputs.iter().sum::<usize>();
        stat.throughput = (stat.ops as f64 / duration.as_secs_f64()) as u64;

        // Sample-related stats.
        if stat.ops > 0 {
            let mut get_lats = ws_stat
                .latencies
                .iter()
                .flat_map(|l| l.get.iter())
                .copied()
                .collect::<Vec<_>>();
            get_lats.sort();

            let mut put_lats = ws_stat
                .latencies
                .iter()
                .flat_map(|l| l.put.iter())
                .copied()
                .collect::<Vec<_>>();
            put_lats.sort();

            // GET performance.
            if !get_lats.is_empty() {
                stat.get_avg = get_lats.iter().sum::<Duration>() / get_lats.len() as u32;
                stat.get_p50 = if get_lats.len() % 2 == 1 {
                    get_lats[get_lats.len() / 2]
                } else {
                    let pivot = get_lats.len() / 2;
                    (get_lats[pivot - 1] + get_lats[pivot]) / 2
                };
                stat.get_p99 = get_lats[(get_lats.len() as f64 * 0.99) as usize];
                stat.get_p999 = get_lats[(get_lats.len() as f64 * 0.999) as usize];
            } else {
                stat.get_avg = Duration::from_nanos(0);
                stat.get_p50 = Duration::from_nanos(0);
                stat.get_p99 = Duration::from_nanos(0);
                stat.get_p999 = Duration::from_nanos(0);
            }

            // PUT performance.
            if !put_lats.is_empty() {
                stat.put_avg = put_lats.iter().sum::<Duration>() / put_lats.len() as u32;
                stat.put_p50 = if put_lats.len() % 2 == 1 {
                    put_lats[put_lats.len() / 2]
                } else {
                    let pivot = put_lats.len() / 2;
                    (put_lats[pivot - 1] + put_lats[pivot]) / 2
                };
                stat.put_p99 = put_lats[(put_lats.len() as f64 * 0.99) as usize];
                stat.put_p999 = put_lats[(put_lats.len() as f64 * 0.999) as usize];
            } else {
                stat.put_avg = Duration::from_nanos(0);
                stat.put_p50 = Duration::from_nanos(0);
                stat.put_p99 = Duration::from_nanos(0);
                stat.put_p999 = Duration::from_nanos(0);
            }
        }
        stat
    }
}
