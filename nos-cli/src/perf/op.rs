use super::Perfmon;
use quanta::Instant;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpType {
    Get,
    Put,
}

/// A guard struct that represents an operation to be measured.
///
/// When dropped, the operation is considered finished and the
/// corresponding latency and throughput will be recorded.
pub struct Op<'a> {
    mon: &'a Perfmon,
    tid: usize,
    op_type: OpType,
    start: Instant,
}

impl<'a> Op<'a> {
    #[inline]
    pub fn new(mon: &'a Perfmon, tid: usize, op_type: OpType) -> Self {
        Self {
            mon,
            tid,
            op_type,
            start: Instant::now(),
        }
    }
}

impl<'a> Drop for Op<'a> {
    #[inline]
    fn drop(&mut self) {
        let elapsed = self.start.elapsed();
        self.mon.push(self.tid, self.op_type, elapsed);
    }
}
