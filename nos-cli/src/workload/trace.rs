use std::fs::File;
use std::io::{self, BufRead, BufReader};

/// A record in a trace.
pub struct TraceRecord {
    /// Whether this is a `SET` operation.
    pub is_set: bool,

    /// Key hash.
    pub key: u64,

    /// Value size.
    pub len: usize,
}

/// A trace.
pub struct Trace {
    /// Records in the trace.
    pub records: Vec<TraceRecord>,

    /// Whether this is a preparation run.
    pub is_prepare: bool,
}

impl Trace {
    /// Maximum number of records.
    pub const MAX_RECORDS: usize = 20_000_000;

    /// Load trace from file.
    /// For file format, refer to the `trace-preprocess` crate.
    pub fn load(filename: impl AsRef<str>, is_prepare: bool) -> io::Result<Self> {
        let file = File::open(filename.as_ref())?;
        let reader = BufReader::new(file);

        let mut records = Vec::new();
        for line in reader.lines().take(Self::MAX_RECORDS) {
            let line = line?;

            let mut parts = line.split_whitespace();
            let is_set = parts.next().unwrap() == "1";
            let key = parts.next().unwrap().parse().unwrap();
            let len = parts.next().unwrap().parse().unwrap();

            if !is_prepare {
                records.push(TraceRecord { is_set, key, len });
            } else {
                // Populate record for all GETs.
                if !is_set {
                    records.push(TraceRecord {
                        is_set: true,
                        key,
                        len,
                    });
                }
            }
        }

        log::info!(
            "loaded {} records from trace {}",
            records.len(),
            filename.as_ref()
        );
        Ok(Self {
            records,
            is_prepare,
        })
    }

    /// Get a iterator over the records.
    /// This iterator never ends, as it will loop over the records.
    pub fn iter(&self, idx: usize, step: usize) -> impl Iterator<Item = &TraceRecord> + '_ {
        TraceIter {
            trace: self,
            initial: idx,
            idx,
            step,
        }
    }
}

/// Trace record iterator.
pub struct TraceIter<'a> {
    /// Reference to the trace.
    trace: &'a Trace,

    /// Initial index.
    initial: usize,

    /// Next index.
    idx: usize,

    /// Step size.
    step: usize,
}

impl<'a> Iterator for TraceIter<'a> {
    type Item = &'a TraceRecord;

    fn next(&mut self) -> Option<Self::Item> {
        if self.idx >= self.trace.records.len() {
            if self.trace.is_prepare {
                return None;
            }
            self.idx = self.initial;
        }
        let idx = self.idx;
        self.idx = self.idx + self.step;
        Some(unsafe { self.trace.records.get_unchecked(idx) })
    }
}
