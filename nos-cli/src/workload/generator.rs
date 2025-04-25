#![allow(dead_code)]

use std::cell::RefCell;
use std::hash::Hasher;
use std::sync::atomic::{AtomicUsize, Ordering};

use ahash::AHasher;
use rand::prelude::*;
use rand_distr::Zipf;

use nos::Key;

use crate::workload::defs::*;
use crate::workload::trace::*;

/// A request to the key-value store.
#[derive(Debug)]
pub enum KvRequest {
    /// Read an object (key index).
    Read(usize),

    /// Write (insert/update) an object (key index, value).
    Write(usize, Box<[u8]>),
}

impl KvRequest {
    /// Get the key index of the request.
    pub fn key_idx(&self) -> usize {
        match self {
            KvRequest::Read(key) => *key,
            KvRequest::Write(key, _) => *key,
        }
    }
}

struct LatestWindow {
    window: Vec<usize>,
    len: usize,
    next: usize,
}

impl LatestWindow {
    pub fn new(len: usize) -> Self {
        Self {
            window: Vec::with_capacity(len),
            len,
            next: 0,
        }
    }

    pub fn push(&mut self, key: usize) {
        if self.window.len() < self.len {
            self.window.push(key);
        } else {
            self.window[self.next] = key;
            self.next = (self.next + 1) % self.len;
        }
    }

    /// Panics if there is no key in the window.
    pub fn get(&self) -> usize {
        let idx = rand::random::<usize>() % self.window.len();
        self.window[idx]
    }
}

/// Generate key-value request streams according to a workload.
pub struct WorkloadGenerator<'a> {
    workload: &'a Workload,
    has_insert: bool,
    rand_seed: usize,

    read_bound: f32,
    update_bound: f32,

    next_key: AtomicUsize,
    latest_window: RefCell<LatestWindow>,
    zipf: Zipf<f32>,
}

impl<'a> WorkloadGenerator<'a> {
    /// Create a new workload generator.
    pub fn new(workload: &'a Workload) -> Self {
        let read_bound = workload.proportion.read;
        let update_bound = read_bound + workload.proportion.update;

        let next_key = workload.count;
        let has_insert = workload.proportion.insert != 0.0f32;

        // Populate latest_window.
        let latest_window_len = (workload.count.max(1) - 1) / 20 + 1;
        let mut latest_window = LatestWindow::new(latest_window_len);
        if workload.count >= latest_window_len {
            for i in (workload.count - latest_window_len)..workload.count {
                latest_window.push(i);
            }
        }

        // Avoid Zipf rounding errors.
        const WORKLOAD_COUNT_THRESHOLD: usize = 1_000_000;
        let count = if matches!(workload.distribution, WorkloadDistribution::Zipf(_))
            && workload.count > WORKLOAD_COUNT_THRESHOLD
        {
            ((workload.count as f64) * 0.999f64).floor() as usize
        } else {
            workload.count
        };

        // Initiate the random generator.
        let zipf = Zipf::new(
            count.max(1) as u64,
            match workload.distribution {
                WorkloadDistribution::Uniform => 0.0,
                WorkloadDistribution::Zipf(alpha) => alpha,
            },
        )
        .unwrap();

        Self {
            workload,
            has_insert,
            rand_seed: rand::random::<usize>(),

            read_bound,
            update_bound,
            next_key: AtomicUsize::new(next_key),
            latest_window: RefCell::new(latest_window),
            zipf,
        }
    }

    /// Get an iterator over the trace records.
    #[inline]
    pub fn trace_iter(
        &self,
        idx: usize,
        step: usize,
    ) -> Option<impl Iterator<Item = &TraceRecord>> {
        self.workload
            .trace_content
            .as_ref()
            .map(|trace| trace.iter(idx, step))
    }

    /// Get the value size of the given key according to the workload.
    #[inline]
    pub fn value_len<K>(&self, key: K) -> usize
    where
        K: Key,
    {
        match self.workload.value_pattern {
            None => self.workload.value_size,
            Some(ref str) => match str.as_str() {
                "etc" => {
                    let mut hasher = AHasher::default();
                    key.hash(&mut hasher);
                    let h = hasher.finish() % 100;

                    match h {
                        0..=39 => 64,
                        40..=94 => 384,
                        _ => self.workload.value_size,
                    }
                }
                _ => panic!("unknown value pattern: {}", str),
            },
        }
    }

    /// Generate a request according to the workload.
    #[inline]
    pub fn generate(&self) -> KvRequest {
        let choice = rand::random::<f32>();
        match choice {
            x if x < self.read_bound => {
                // Read
                let key = if self.has_insert {
                    self.latest_window.borrow().get()
                } else {
                    match self.workload.distribution {
                        WorkloadDistribution::Uniform => {
                            rand::random::<usize>() % self.workload.count
                        }
                        WorkloadDistribution::Zipf(_) => {
                            rand::thread_rng().sample(self.zipf) as usize
                        }
                    }
                };
                KvRequest::Read(key)
            }
            x if x < self.update_bound => {
                // Update
                let key = rand::random::<usize>() % self.workload.count;
                KvRequest::Write(key, vec![1; self.value_len(key as u64)].into_boxed_slice())
            }
            _ => {
                // Insert
                let key = self
                    .next_key
                    .fetch_add(thread_rng().gen_range(1..=32), Ordering::Relaxed);
                self.latest_window.borrow_mut().push(key);
                KvRequest::Write(key, vec![1; self.value_len(key as u64)].into_boxed_slice())
            }
        }
    }
}
