use rand::prelude::*;

use crate::ctrl::Cluster;
use crate::Key;

/// Determine how many encodees needs to be recursively recovered to construct
/// the desired object by the given parity encodee list.
#[inline(always)]
pub fn count_recursive<K>(key: K, parity: &[K], cluster: &Cluster) -> usize
where
    K: Key,
{
    parity
        .iter()
        .filter(|k| {
            **k != key && {
                let primary = (**k % cluster.size() as u64) as usize;
                !cluster[primary].is_alive()
            }
        })
        .count()
}

/// Pick the best parity for a degraded read.
///
/// **FIXME:** introduce versioning.
#[inline]
pub fn pick_best_parity<'a, K>(
    key: K,
    parities: &'a [(usize, Vec<K>)],
    cluster: &Cluster,
) -> (usize, &'a Vec<K>)
where
    K: Key,
{
    assert!(!parities.is_empty());
    let mut cands = parities
        .iter()
        .map(|(i, parity)| (*i, parity, count_recursive(key, parity, cluster)))
        .collect::<Vec<_>>();
    cands.sort_by_key(|(_, _, count)| *count);

    // Return a random choice of the best parity
    let min_count = cands[0].2;
    let n = cands
        .iter()
        .position(|(_, _, count)| *count != min_count)
        .unwrap_or(cands.len());
    let idx = thread_rng().gen_range(0..n);
    (cands[idx].0, cands[idx].1)
}
