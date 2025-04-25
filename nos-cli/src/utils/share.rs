use std::cmp;
use std::ops::Range;

/// Compute the share of key range to for #`idx` of a total of `n` workers.
pub fn get_share(range: Range<usize>, idx: usize, n: usize) -> Range<usize> {
    let len = range.len();
    let share_len = len / n;
    let remainder = len % n;
    let start = range.start + share_len * idx + cmp::min(idx, remainder);
    let end = start + share_len + if idx < remainder { 1 } else { 0 };
    start..end
}
