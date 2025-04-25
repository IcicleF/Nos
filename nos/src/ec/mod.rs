//! Erasure coding functionalities.
//!
//! Low-level erasure coding and XOR functionalities are provided by Intel ISA-L.
//! Also, this module has SBIBD implementation for Nos's use.

use std::fmt;
use std::path::Path;
use std::ptr;

pub mod isal_wrappings;
use isal_wrappings as isal;

mod sbibd;
pub use sbibd::*;

/// Conventional erasure coding configuration.
#[derive(Clone)]
pub struct EcConfig {
    /// Number of data chunks to be encoded together for each parity.
    pub k: usize,

    /// Fault tolerance threshold.
    pub p: usize,

    /// Encode matrix.
    pub encode_matrix: Vec<u8>,

    /// Encode GF tables.
    pub encode_gf_tbls: Vec<u8>,
}

impl fmt::Debug for EcConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EcConfig")
            .field("k", &self.k)
            .field("p", &self.p)
            .finish()
    }
}

impl EcConfig {
    /// Create a new EC configuration.
    pub fn new(k: usize, p: usize) -> Self {
        assert_ne!(k, 0, "EcConfig: parameter `k` must be non-zero");
        if k <= p {
            log::warn!(
                "EcConfig: parameter `k` must be greater than `p`, you must not be running Nos!"
            );
        }
        if p == 0 {
            return Self {
                k,
                p,
                encode_matrix: Vec::new(),
                encode_gf_tbls: Vec::new(),
            };
        }

        let m = k + p;
        let mut encode_matrix = vec![0; m * k];
        let mut encode_gf_tbls = vec![0; 32 * k * p];

        // SAFETY: FFI.
        unsafe {
            isal_sys::gf_gen_cauchy1_matrix(encode_matrix.as_mut_ptr(), m as i32, k as i32);
            isal_sys::ec_init_tables(
                k as i32,
                p as i32,
                encode_matrix.as_mut_ptr().add(k * k),
                encode_gf_tbls.as_mut_ptr(),
            );
        }
        Self {
            k,
            p,
            encode_matrix,
            encode_gf_tbls,
        }
    }

    /// Load TOML configuration.
    ///
    /// The TOML file must have a `ec` table with `k` and `p` fields.
    /// Note that other irrelevant fields are ignored.
    ///
    /// ## TOML Example
    ///
    /// ```toml
    /// [ec]
    /// k = 4
    /// p = 2
    /// ```
    pub fn load_toml(toml_str: &str) -> Self {
        let toml_val: toml::Value =
            toml::from_str(toml_str).expect("EcConfig: failed to parse TOML config");
        let config = toml_val["ec"]
            .as_table()
            .expect("EcConfig: no EC configuration found in the given TOML config");
        EcConfig::new(
            config["k"].as_integer().unwrap() as usize,
            config["p"].as_integer().unwrap() as usize,
        )
    }

    /// Load the configuration from a TOML file.
    pub fn load_toml_file(path: impl AsRef<Path>) -> Self {
        let config = std::fs::read_to_string(path).expect("EcConfig: failed to read TOML file");
        Self::load_toml(&config)
    }

    /// Get the stripe width.
    #[inline]
    pub fn width(&self) -> usize {
        self.k + self.p
    }

    /// Encode the data chunks into parity chunks.
    pub fn encode(&self, data: &[&[u8]], parity: &mut [&mut [u8]]) {
        debug_assert!(
            data.len() == self.k && parity.len() == self.p,
            "EC encode: invalid data/parity chunk counts: {:?}, expected {:?}",
            (data.len(), parity.len()),
            (self.k, self.p)
        );

        if self.p == 0 {
            return;
        }
        let len = data[0].len();
        debug_assert!(
            data.iter().all(|chunk| chunk.len() == len)
                && parity.iter().all(|chunk| chunk.len() == len),
            "EC encode: data/parity chunks must have the same length"
        );

        let data_ptrs = data.iter().map(|chunk| chunk.as_ptr()).collect::<Vec<_>>();
        let parity_ptrs = parity
            .iter_mut()
            .map(|chunk| chunk.as_mut_ptr())
            .collect::<Vec<_>>();

        isal::ec_encode(&self.encode_gf_tbls, &data_ptrs, &parity_ptrs, len);
    }

    /// Compute the update of parity chunks given a data chunk update.
    pub fn update(&self, data: &[u8], index: usize, parity: &mut [&mut [u8]]) {
        if self.p == 0 {
            return;
        }

        debug_assert!(
            parity.len() == self.p,
            "EC update: invalid parity chunk number: {}, expected {}",
            parity.len(),
            self.p
        );
        let len = data.len();
        debug_assert!(
            parity.iter().all(|chunk| chunk.len() == len),
            "EC update: data/parity chunks must have the same length"
        );

        let parity_ptrs = parity
            .iter_mut()
            .map(|chunk| chunk.as_mut_ptr())
            .collect::<Vec<_>>();

        isal::ec_update(&self.encode_gf_tbls, self.k, &parity_ptrs, len, index, data);
    }

    /// Recover lost data chunks from provided data and parity chunks.
    ///
    /// Input format:
    /// - `data`: An array of `k` data chunks. Lost chunks will be filled in.
    /// - `parity`: An array of needed parity chunks.
    /// - `indices`: A sorted array of `k` indices indicating the index of available chunk.
    pub fn recover(&self, data: &mut [&mut [u8]], parity: &[&[u8]], indices: &[usize]) {
        debug_assert!(
            self.p != 0,
            "EC recover: cannot recover with no parity configured"
        );
        debug_assert!(
            !data.is_empty(),
            "EC recover: cannot recover with no input chunks"
        );
        debug_assert!(
            !parity.is_empty(),
            "EC recover: cannot recover with no output chunks"
        );
        debug_assert_eq!(
            data.len(),
            self.k,
            "EC recover: data chunk count (LHS) mismatch with config (RHS)",
        );
        debug_assert!(
            parity.len() <= self.p,
            "EC recover: required recovery chunk {} exceeds capacity {}",
            parity.len(),
            self.p
        );
        assert_eq!(
            indices.len(),
            self.k,
            "EC recover: input chunk count (LHS) mismatch with config (RHS)",
        );
        assert!(
            indices.iter().all(|&idx| idx < self.width()),
            "EC recover: invalid chunk index"
        );

        let nerrs = indices.iter().filter(|&&idx| idx >= self.k).count();
        assert_eq!(
            nerrs,
            parity.len(),
            "EC recover: lost chunk count (LHS) mismatch with parity chunk count (RHS)"
        );

        let lost_data = (0..self.k)
            .filter(|&idx| !indices.contains(&idx))
            .collect::<Vec<_>>();
        debug_assert_eq!(
            lost_data.len(),
            nerrs,
            "EC recover: lost chunk count mismatch: {:?} vs {} parities provided",
            indices,
            nerrs
        );

        let len = data[0].len();
        debug_assert!(
            data.iter().all(|chunk| chunk.len() == len)
                && parity.iter().all(|chunk| chunk.len() == len),
            "EC recover: data/parity chunks must have the same length"
        );

        // Construct decode matrix
        let inv_matrix = isal::ec_make_decode_matrix(self.k, self.p, &self.encode_matrix, indices);
        let mut decode_matrix = vec![0; self.k * nerrs];
        for (i, &idx) in lost_data.iter().enumerate() {
            unsafe {
                ptr::copy_nonoverlapping(
                    inv_matrix.as_ptr().add(idx * self.k),
                    decode_matrix.as_mut_ptr().add(i * self.k),
                    self.k,
                );
            }
        }

        // Pack chunk pointers
        let mut src_ptrs = vec![ptr::null(); self.k];
        let mut dst_ptrs = vec![ptr::null_mut(); nerrs];
        let mut parity_idx = 0;
        for i in 0..self.k {
            src_ptrs[i] = if indices[i] < self.k {
                data[indices[i]].as_ptr()
            } else {
                let parity_ptr = parity[parity_idx].as_ptr();
                parity_idx += 1;
                parity_ptr
            };
        }
        for i in 0..nerrs {
            dst_ptrs[i] = data[lost_data[i]].as_mut_ptr();
        }

        // Recover
        isal::ec_onetime_encode(&decode_matrix, &src_ptrs, &dst_ptrs, len);
    }
}

impl Default for EcConfig {
    /// Default EC configuration with (k, p) = (4, 2).
    fn default() -> Self {
        // SBIBD definitely exists for k = 4.
        EcConfig::new(4, 2)
    }
}
