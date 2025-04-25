//! Serial number manager.

use super::{AtomicVersion, Version};
use std::sync::atomic::*;

const BITS: Version = 512;

pub(super) struct SnManager {
    sn: AtomicVersion,
    csn: AtomicVersion,
    pending: [AtomicU64; BITS as usize / 64],
}

impl SnManager {
    pub fn new() -> SnManager {
        Self {
            sn: AtomicVersion::new(1),
            csn: AtomicVersion::new(0),
            pending: std::array::from_fn(|_| AtomicU64::new(0)),
        }
    }

    /// Fetch a new serial number.
    pub fn fetch(&self) -> Version {
        self.sn.fetch_add(1, Ordering::Relaxed)
    }

    /// Get the current committed serial number.
    pub fn csn(&self) -> Version {
        self.csn.load(Ordering::Relaxed)
    }

    /// Commit a serial number.
    ///
    /// If returned `false`, it means the serial number is too large and not ready to be committed.
    /// The caller must retry later.
    pub fn commit(&self, sn: Version) -> bool {
        let mut cur_csn = self.csn.load(Ordering::Relaxed);

        // If the serial number is too large such that the window cannot cover it, return false.
        const MAX_INFLIGHT: Version = BITS - 64;
        if cur_csn + MAX_INFLIGHT <= sn {
            return false;
        }

        // Quick commit path with no contention.
        if cur_csn + 1 == sn {
            cur_csn =
                match self
                    .csn
                    .compare_exchange(cur_csn, sn, Ordering::SeqCst, Ordering::Relaxed)
                {
                    Err(csn) => csn,
                    _ => return true,
                };
        }

        // Previous SNs uncommitted. Record in the bitmap and try to make progress.
        let t = self.pending[(sn % BITS) as usize / 64].fetch_or(1 << (sn % 64), Ordering::Relaxed);
        assert!(t & (1 << (sn % 64)) == 0);

        while cur_csn < sn {
            let next_idx = ((cur_csn + 1) % BITS) / 64;
            let next_start_bit = (cur_csn + 1) % 64;
            let val = self.pending[next_idx as usize].load(Ordering::Relaxed);
            let leap = (val >> next_start_bit).trailing_ones() as u64;

            if leap > 0 {
                let bits_to_clear = if leap == 64 {
                    !0
                } else {
                    ((1 << leap) - 1) << next_start_bit
                };

                let p = &self.pending[next_idx as usize];
                let mut pv = p.load(Ordering::Relaxed);
                loop {
                    let res = p.compare_exchange(
                        pv,
                        pv & !bits_to_clear,
                        Ordering::SeqCst,
                        Ordering::Relaxed,
                    );
                    match res {
                        Ok(_) => break,
                        Err(v) => pv = v,
                    }
                }

                let target = cur_csn + leap as Version;
                while cur_csn < target {
                    let res = self.csn.compare_exchange(
                        cur_csn,
                        target,
                        Ordering::SeqCst,
                        Ordering::Relaxed,
                    );
                    cur_csn = match res {
                        Err(csn) => csn,
                        _ => target,
                    };
                }
            }

            std::hint::spin_loop();
            cur_csn = self.csn.load(Ordering::Relaxed);
        }
        true
    }
}
