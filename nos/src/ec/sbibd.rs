use crate::ec::EcConfig;

/// Convert a vector of i32 to a vector of bool.
fn vec_to_onehot(v: Vec<i32>, n: i32) -> Vec<bool> {
    let m = n * (n - 1) + 1;
    let mut ret = vec![false; m as usize];
    v.iter().for_each(|x| {
        ret[*x as usize] = true;
    });
    ret
}

/// Normalize x modulo n*(n-1)+1.
fn normalize(x: i32, n: i32) -> i32 {
    let m = n * (n - 1) + 1;
    let x = x % m;
    if x < 0 {
        x + m
    } else {
        x
    }
}

/// Perform a depth-first-search for a (n^2-n+1, n, 1) difference set.
fn dfs(n: i32, ret: &mut Vec<i32>, diff: &mut Vec<bool>, i: i32, j: i32) -> bool {
    let m = n * (n - 1) + 1;

    if i == n {
        true
    } else {
        // Find the next element in the difference set
        for k in j..m {
            let mut d1_diffs = ret.iter().map(|x| normalize(k - *x, n));
            let d2_diffs = ret.iter().map(|x| normalize(*x - k, n)).collect::<Vec<_>>();

            let valid = d1_diffs.all(|x| !d2_diffs.contains(&x));
            if !valid {
                continue;
            }

            let valid = ret.iter().all(|x| {
                !diff[normalize(k - *x, n) as usize] && !diff[normalize(*x - k, n) as usize]
            });
            if valid {
                ret.iter().for_each(|x| {
                    diff[normalize(k - *x, n) as usize] = true;
                    diff[normalize(*x - k, n) as usize] = true;
                });
                ret.push(k);

                if dfs(n, ret, diff, i + 1, k + 1) {
                    return true;
                }

                ret.pop();
                ret.iter().for_each(|x| {
                    diff[normalize(k - *x, n) as usize] = false;
                    diff[normalize(*x - k, n) as usize] = false;
                });
            }
        }
        false
    }
}

/// Find a (n^2-n+1, n, 1) difference set.
fn find_diffset(n: i32) -> Option<Vec<i32>> {
    let m = n * (n - 1) + 1;
    let mut ret: Vec<i32> = vec![0];
    let mut diff = vec![false; m as usize];

    if dfs(n, &mut ret, &mut diff, 1, 1) {
        Some(ret)
    } else {
        None
    }
}

/// Determine whether a number is a prime power.
fn is_prime_power(mut x: i32) -> bool {
    let mut p = 2;
    while x % p != 0 {
        p += 1;
    }
    if x % p == 0 {
        while x % p == 0 {
            x /= p;
        }
        x == 1
    } else {
        false
    }
}

/// Find a (n^2-n+1, n, 1)-SBIBD and guarantee that the i-th block do not contain element i.
/// Based on difference sets.
fn find_sbibd(n: i32) -> Option<Vec<Vec<bool>>> {
    match n {
        n if !is_prime_power(n - 1) => None,
        n => {
            let diffset = find_diffset(n);
            let diffset = diffset?;

            let mut ret = vec![];
            for i in 0..(n * (n - 1) + 1) {
                ret.push(vec_to_onehot(
                    diffset
                        .iter()
                        .map(|x| normalize(*x + i + 1, n))
                        .collect::<Vec<_>>(),
                    n,
                ))
            }
            Some(ret)
        }
    }
}

/// Nos's BIBD-based erasure coding configuration.
pub struct Sbibd {
    /// Number of data chunks to be encoded together for each parity.
    pub k: usize,

    /// Fault tolerance threshold.
    pub p: usize,

    /// Size of the SBIBD.
    size: usize,

    /// SBIBD affinity matrix.
    affinity: Vec<Vec<bool>>,
    /// Backup node list for each primary node.
    backups: Vec<Vec<usize>>,
    /// Primary node list for each backup node.
    primaries: Vec<Vec<usize>>,
    /// Replication source inverse map.
    repl_src_inv: Vec<Vec<usize>>,
}

impl Sbibd {
    /// Create a SBIBD for the given EC configuration.
    pub fn new(ec_config: &EcConfig) -> Sbibd {
        let k = ec_config.k;
        let p = ec_config.p;
        let size = k * k - k + 1;

        let affinity =
            find_sbibd(k as i32).expect("Sbibd: cannot find SBIBD for the given parameters");
        debug_assert_eq!(
            affinity.len(),
            size,
            "Sbibd: searched SBIBD has wrong size {} (expected {})",
            affinity.len(),
            size
        );
        let backups = (0..size)
            .map(|primary| {
                affinity[primary]
                    .iter()
                    .enumerate()
                    .filter(|(_, &x)| x)
                    .map(|(i, _)| i)
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        let primaries = (0..size)
            .map(|backup| {
                affinity
                    .iter()
                    .enumerate()
                    .filter(|(_, x)| x[backup])
                    .map(|(i, _)| i)
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();

        // Construct the replication source inverse map
        let mut repl_src_inv = vec![vec![0; size]; size];
        for backup in 0..size {
            for (i, &primary) in primaries[backup].iter().enumerate() {
                repl_src_inv[backup][primary] = i;
            }
        }

        Self {
            k,
            p,
            size,
            affinity,
            backups,
            primaries,
            repl_src_inv,
        }
    }

    /// Get the size of the SBIBD.
    #[inline(always)]
    pub fn size(&self) -> usize {
        self.size
    }

    /// Check if the given primary node can send a object replica to the given backup node.
    #[inline]
    pub fn is_backup(&self, primary: usize, backup: usize) -> bool {
        self.affinity[primary][backup]
    }

    /// Find all backup nodes for the given primary node.
    #[inline]
    pub fn backups_of(&self, primary: usize) -> &[usize] {
        &self.backups[primary]
    }

    /// Find all primary nodes that can send to the given backup node.
    #[inline]
    pub fn primaries_into(&self, backup: usize) -> &[usize] {
        &self.primaries[backup]
    }

    /// Get replication source index.
    #[inline]
    pub fn repl_src_index(&self, backup: usize, primary: usize) -> usize {
        self.repl_src_inv[backup][primary]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalization() {
        assert_eq!(normalize(0, 3), 0);
        assert_eq!(normalize(1, 3), 1);
        assert_eq!(normalize(2, 3), 2);
        assert_eq!(normalize(3, 3), 3);
        assert_eq!(normalize(-1, 10), 90);
        assert_eq!(normalize(-2, 10), 89);
        assert_eq!(normalize(-3, 10), 88);
    }

    #[test]
    fn existence() {
        assert!(find_diffset(1 + 0).is_some());
        assert!(find_diffset(1 + 1).is_some());
        assert!(find_diffset(1 + 2).is_some());
        assert!(find_diffset(1 + 3).is_some());
        assert!(find_diffset(1 + 4).is_some());
        assert!(find_diffset(1 + 5).is_some());
        assert!(find_diffset(1 + 7).is_some());
        assert!(find_diffset(1 + 8).is_some());
        assert!(find_diffset(1 + 9).is_some());
        assert!(find_diffset(1 + 11).is_some());
    }

    #[test]
    #[ignore = "too slow"]
    fn inexistence() {
        assert!(find_diffset(1 + 6).is_none());
        assert!(find_diffset(1 + 10).is_none());
    }
}
