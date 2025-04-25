#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

extern "C" {
    pub fn crc16_t10dif(init_crc: u16, buf: *const ::std::os::raw::c_uchar, len: u64) -> u16;
}
extern "C" {
    pub fn crc16_t10dif_copy(init_crc: u16, dst: *mut u8, src: *mut u8, len: u64) -> u16;
}
extern "C" {
    pub fn crc32_ieee(init_crc: u32, buf: *const ::std::os::raw::c_uchar, len: u64) -> u32;
}
extern "C" {
    pub fn crc32_gzip_refl(init_crc: u32, buf: *const ::std::os::raw::c_uchar, len: u64) -> u32;
}
extern "C" {
    pub fn crc32_iscsi(
        buffer: *mut ::std::os::raw::c_uchar,
        len: ::std::os::raw::c_int,
        init_crc: ::std::os::raw::c_uint,
    ) -> ::std::os::raw::c_uint;
}
extern "C" {
    pub fn crc32_iscsi_base(
        buffer: *mut ::std::os::raw::c_uchar,
        len: ::std::os::raw::c_int,
        crc_init: ::std::os::raw::c_uint,
    ) -> ::std::os::raw::c_uint;
}
extern "C" {
    pub fn crc16_t10dif_base(seed: u16, buf: *mut u8, len: u64) -> u16;
}
extern "C" {
    pub fn crc16_t10dif_copy_base(init_crc: u16, dst: *mut u8, src: *mut u8, len: u64) -> u16;
}
extern "C" {
    pub fn crc32_ieee_base(seed: u32, buf: *mut u8, len: u64) -> u32;
}
extern "C" {
    pub fn crc32_gzip_refl_base(seed: u32, buf: *mut u8, len: u64) -> u32;
}
extern "C" {
    pub fn crc64_ecma_refl(init_crc: u64, buf: *const ::std::os::raw::c_uchar, len: u64) -> u64;
}
extern "C" {
    pub fn crc64_ecma_norm(init_crc: u64, buf: *const ::std::os::raw::c_uchar, len: u64) -> u64;
}
extern "C" {
    pub fn crc64_iso_refl(init_crc: u64, buf: *const ::std::os::raw::c_uchar, len: u64) -> u64;
}
extern "C" {
    pub fn crc64_iso_norm(init_crc: u64, buf: *const ::std::os::raw::c_uchar, len: u64) -> u64;
}
extern "C" {
    pub fn crc64_jones_refl(init_crc: u64, buf: *const ::std::os::raw::c_uchar, len: u64) -> u64;
}
extern "C" {
    pub fn crc64_jones_norm(init_crc: u64, buf: *const ::std::os::raw::c_uchar, len: u64) -> u64;
}
extern "C" {
    pub fn crc64_ecma_refl_by8(init_crc: u64, buf: *const ::std::os::raw::c_uchar, len: u64)
        -> u64;
}
extern "C" {
    pub fn crc64_ecma_norm_by8(init_crc: u64, buf: *const ::std::os::raw::c_uchar, len: u64)
        -> u64;
}
extern "C" {
    pub fn crc64_ecma_refl_base(
        init_crc: u64,
        buf: *const ::std::os::raw::c_uchar,
        len: u64,
    ) -> u64;
}
extern "C" {
    pub fn crc64_ecma_norm_base(
        init_crc: u64,
        buf: *const ::std::os::raw::c_uchar,
        len: u64,
    ) -> u64;
}
extern "C" {
    pub fn crc64_iso_refl_by8(init_crc: u64, buf: *const ::std::os::raw::c_uchar, len: u64) -> u64;
}
extern "C" {
    pub fn crc64_iso_norm_by8(init_crc: u64, buf: *const ::std::os::raw::c_uchar, len: u64) -> u64;
}
extern "C" {
    pub fn crc64_iso_refl_base(init_crc: u64, buf: *const ::std::os::raw::c_uchar, len: u64)
        -> u64;
}
extern "C" {
    pub fn crc64_iso_norm_base(init_crc: u64, buf: *const ::std::os::raw::c_uchar, len: u64)
        -> u64;
}
extern "C" {
    pub fn crc64_jones_refl_by8(
        init_crc: u64,
        buf: *const ::std::os::raw::c_uchar,
        len: u64,
    ) -> u64;
}
extern "C" {
    pub fn crc64_jones_norm_by8(
        init_crc: u64,
        buf: *const ::std::os::raw::c_uchar,
        len: u64,
    ) -> u64;
}
extern "C" {
    pub fn crc64_jones_refl_base(
        init_crc: u64,
        buf: *const ::std::os::raw::c_uchar,
        len: u64,
    ) -> u64;
}
extern "C" {
    pub fn crc64_jones_norm_base(
        init_crc: u64,
        buf: *const ::std::os::raw::c_uchar,
        len: u64,
    ) -> u64;
}
extern "C" {
    pub fn gf_vect_mul_sse(
        len: ::std::os::raw::c_int,
        gftbl: *mut ::std::os::raw::c_uchar,
        src: *mut ::std::os::raw::c_void,
        dest: *mut ::std::os::raw::c_void,
    ) -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn gf_vect_mul_avx(
        len: ::std::os::raw::c_int,
        gftbl: *mut ::std::os::raw::c_uchar,
        src: *mut ::std::os::raw::c_void,
        dest: *mut ::std::os::raw::c_void,
    ) -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn gf_vect_mul(
        len: ::std::os::raw::c_int,
        gftbl: *mut ::std::os::raw::c_uchar,
        src: *mut ::std::os::raw::c_void,
        dest: *mut ::std::os::raw::c_void,
    ) -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn gf_vect_mul_init(c: ::std::os::raw::c_uchar, gftbl: *mut ::std::os::raw::c_uchar);
}
extern "C" {
    pub fn gf_vect_mul_base(
        len: ::std::os::raw::c_int,
        a: *mut ::std::os::raw::c_uchar,
        src: *mut ::std::os::raw::c_uchar,
        dest: *mut ::std::os::raw::c_uchar,
    ) -> ::std::os::raw::c_int;
}
extern "C" {
    /// Initialize tables for fast Erasure Code encode and decode.
    ///
    /// Generates the expanded tables needed for fast encode or decode for erasure
    /// codes on blocks of data. 32bytes is generated for each input coefficient.
    ///
    /// # Arguments
    ///
    /// * `k`      - The number of vector sources or rows in the generator matrix
    ///              for coding.
    /// * `rows`   - The number of output vectors to concurrently encode/decode.
    /// * `a`      - Pointer to array of input coefficients for coding.
    /// * `gftbls` - Pointer to start of space for concatenated output tables
    ///              generated from input coefficients. Must be of size `32*k*rows`.
    pub fn ec_init_tables(
        k: ::std::os::raw::c_int,
        rows: ::std::os::raw::c_int,
        a: *mut ::std::os::raw::c_uchar,
        gftbls: *mut ::std::os::raw::c_uchar,
    );
}
extern "C" {
    pub fn ec_encode_data(
        len: ::std::os::raw::c_int,
        k: ::std::os::raw::c_int,
        rows: ::std::os::raw::c_int,
        gftbls: *mut ::std::os::raw::c_uchar,
        data: *mut *mut ::std::os::raw::c_uchar,
        coding: *mut *mut ::std::os::raw::c_uchar,
    );
}
extern "C" {
    pub fn ec_encode_data_base(
        len: ::std::os::raw::c_int,
        srcs: ::std::os::raw::c_int,
        dests: ::std::os::raw::c_int,
        v: *mut ::std::os::raw::c_uchar,
        src: *mut *mut ::std::os::raw::c_uchar,
        dest: *mut *mut ::std::os::raw::c_uchar,
    );
}
extern "C" {
    /// Generate update for encode or decode of erasure codes from single source, runs appropriate version.
    ///
    /// Given one source data block, update one or multiple blocks of encoded data as
    /// specified by a matrix of GF(2^8) coefficients. When given a suitable set of
    /// coefficients, this function will perform the fast generation or decoding of
    /// Reed-Solomon type erasure codes from one input source at a time.
    ///
    /// This function determines what instruction sets are enabled and selects the
    /// appropriate version at runtime.
    ///
    /// # Arguments
    ///
    /// * `len`    - Length of each block of data (vector) of source or dest data.
    /// * `k`      - The number of vector sources or rows in the generator matrix
    ///              for coding.
    /// * `rows`   - The number of output vectors to concurrently encode/decode.
    /// * `vec_i`  - The vector index corresponding to the single input source.
    /// * `g_tbls` - Pointer to array of input tables generated from coding
    ///              coefficients in ec_init_tables(). Must be of size `32*k*rows`.
    /// * `data`   - Pointer to single input source used to update output parity.
    /// * `coding` - Array of pointers to coded output buffers.
    pub fn ec_encode_data_update(
        len: ::std::os::raw::c_int,
        k: ::std::os::raw::c_int,
        rows: ::std::os::raw::c_int,
        vec_i: ::std::os::raw::c_int,
        g_tbls: *mut ::std::os::raw::c_uchar,
        data: *mut ::std::os::raw::c_uchar,
        coding: *mut *mut ::std::os::raw::c_uchar,
    );
}
extern "C" {
    pub fn ec_encode_data_update_base(
        len: ::std::os::raw::c_int,
        k: ::std::os::raw::c_int,
        rows: ::std::os::raw::c_int,
        vec_i: ::std::os::raw::c_int,
        v: *mut ::std::os::raw::c_uchar,
        data: *mut ::std::os::raw::c_uchar,
        dest: *mut *mut ::std::os::raw::c_uchar,
    );
}
extern "C" {
    pub fn gf_vect_dot_prod_base(
        len: ::std::os::raw::c_int,
        vlen: ::std::os::raw::c_int,
        gftbls: *mut ::std::os::raw::c_uchar,
        src: *mut *mut ::std::os::raw::c_uchar,
        dest: *mut ::std::os::raw::c_uchar,
    );
}
extern "C" {
    pub fn gf_vect_dot_prod(
        len: ::std::os::raw::c_int,
        vlen: ::std::os::raw::c_int,
        gftbls: *mut ::std::os::raw::c_uchar,
        src: *mut *mut ::std::os::raw::c_uchar,
        dest: *mut ::std::os::raw::c_uchar,
    );
}
extern "C" {
    pub fn gf_vect_mad(
        len: ::std::os::raw::c_int,
        vec: ::std::os::raw::c_int,
        vec_i: ::std::os::raw::c_int,
        gftbls: *mut ::std::os::raw::c_uchar,
        src: *mut ::std::os::raw::c_uchar,
        dest: *mut ::std::os::raw::c_uchar,
    );
}
extern "C" {
    pub fn gf_vect_mad_base(
        len: ::std::os::raw::c_int,
        vec: ::std::os::raw::c_int,
        vec_i: ::std::os::raw::c_int,
        v: *mut ::std::os::raw::c_uchar,
        src: *mut ::std::os::raw::c_uchar,
        dest: *mut ::std::os::raw::c_uchar,
    );
}
extern "C" {
    pub fn ec_encode_data_sse(
        len: ::std::os::raw::c_int,
        k: ::std::os::raw::c_int,
        rows: ::std::os::raw::c_int,
        gftbls: *mut ::std::os::raw::c_uchar,
        data: *mut *mut ::std::os::raw::c_uchar,
        coding: *mut *mut ::std::os::raw::c_uchar,
    );
}
extern "C" {
    pub fn ec_encode_data_avx(
        len: ::std::os::raw::c_int,
        k: ::std::os::raw::c_int,
        rows: ::std::os::raw::c_int,
        gftbls: *mut ::std::os::raw::c_uchar,
        data: *mut *mut ::std::os::raw::c_uchar,
        coding: *mut *mut ::std::os::raw::c_uchar,
    );
}
extern "C" {
    pub fn ec_encode_data_avx2(
        len: ::std::os::raw::c_int,
        k: ::std::os::raw::c_int,
        rows: ::std::os::raw::c_int,
        gftbls: *mut ::std::os::raw::c_uchar,
        data: *mut *mut ::std::os::raw::c_uchar,
        coding: *mut *mut ::std::os::raw::c_uchar,
    );
}
extern "C" {
    pub fn ec_encode_data_update_sse(
        len: ::std::os::raw::c_int,
        k: ::std::os::raw::c_int,
        rows: ::std::os::raw::c_int,
        vec_i: ::std::os::raw::c_int,
        g_tbls: *mut ::std::os::raw::c_uchar,
        data: *mut ::std::os::raw::c_uchar,
        coding: *mut *mut ::std::os::raw::c_uchar,
    );
}
extern "C" {
    pub fn ec_encode_data_update_avx(
        len: ::std::os::raw::c_int,
        k: ::std::os::raw::c_int,
        rows: ::std::os::raw::c_int,
        vec_i: ::std::os::raw::c_int,
        g_tbls: *mut ::std::os::raw::c_uchar,
        data: *mut ::std::os::raw::c_uchar,
        coding: *mut *mut ::std::os::raw::c_uchar,
    );
}
extern "C" {
    pub fn ec_encode_data_update_avx2(
        len: ::std::os::raw::c_int,
        k: ::std::os::raw::c_int,
        rows: ::std::os::raw::c_int,
        vec_i: ::std::os::raw::c_int,
        g_tbls: *mut ::std::os::raw::c_uchar,
        data: *mut ::std::os::raw::c_uchar,
        coding: *mut *mut ::std::os::raw::c_uchar,
    );
}
extern "C" {
    pub fn gf_vect_dot_prod_sse(
        len: ::std::os::raw::c_int,
        vlen: ::std::os::raw::c_int,
        gftbls: *mut ::std::os::raw::c_uchar,
        src: *mut *mut ::std::os::raw::c_uchar,
        dest: *mut ::std::os::raw::c_uchar,
    );
}
extern "C" {
    pub fn gf_vect_dot_prod_avx(
        len: ::std::os::raw::c_int,
        vlen: ::std::os::raw::c_int,
        gftbls: *mut ::std::os::raw::c_uchar,
        src: *mut *mut ::std::os::raw::c_uchar,
        dest: *mut ::std::os::raw::c_uchar,
    );
}
extern "C" {
    pub fn gf_vect_dot_prod_avx2(
        len: ::std::os::raw::c_int,
        vlen: ::std::os::raw::c_int,
        gftbls: *mut ::std::os::raw::c_uchar,
        src: *mut *mut ::std::os::raw::c_uchar,
        dest: *mut ::std::os::raw::c_uchar,
    );
}
extern "C" {
    pub fn gf_2vect_dot_prod_sse(
        len: ::std::os::raw::c_int,
        vlen: ::std::os::raw::c_int,
        gftbls: *mut ::std::os::raw::c_uchar,
        src: *mut *mut ::std::os::raw::c_uchar,
        dest: *mut *mut ::std::os::raw::c_uchar,
    );
}
extern "C" {
    pub fn gf_2vect_dot_prod_avx(
        len: ::std::os::raw::c_int,
        vlen: ::std::os::raw::c_int,
        gftbls: *mut ::std::os::raw::c_uchar,
        src: *mut *mut ::std::os::raw::c_uchar,
        dest: *mut *mut ::std::os::raw::c_uchar,
    );
}
extern "C" {
    pub fn gf_2vect_dot_prod_avx2(
        len: ::std::os::raw::c_int,
        vlen: ::std::os::raw::c_int,
        gftbls: *mut ::std::os::raw::c_uchar,
        src: *mut *mut ::std::os::raw::c_uchar,
        dest: *mut *mut ::std::os::raw::c_uchar,
    );
}
extern "C" {
    pub fn gf_3vect_dot_prod_sse(
        len: ::std::os::raw::c_int,
        vlen: ::std::os::raw::c_int,
        gftbls: *mut ::std::os::raw::c_uchar,
        src: *mut *mut ::std::os::raw::c_uchar,
        dest: *mut *mut ::std::os::raw::c_uchar,
    );
}
extern "C" {
    pub fn gf_3vect_dot_prod_avx(
        len: ::std::os::raw::c_int,
        vlen: ::std::os::raw::c_int,
        gftbls: *mut ::std::os::raw::c_uchar,
        src: *mut *mut ::std::os::raw::c_uchar,
        dest: *mut *mut ::std::os::raw::c_uchar,
    );
}
extern "C" {
    pub fn gf_3vect_dot_prod_avx2(
        len: ::std::os::raw::c_int,
        vlen: ::std::os::raw::c_int,
        gftbls: *mut ::std::os::raw::c_uchar,
        src: *mut *mut ::std::os::raw::c_uchar,
        dest: *mut *mut ::std::os::raw::c_uchar,
    );
}
extern "C" {
    pub fn gf_4vect_dot_prod_sse(
        len: ::std::os::raw::c_int,
        vlen: ::std::os::raw::c_int,
        gftbls: *mut ::std::os::raw::c_uchar,
        src: *mut *mut ::std::os::raw::c_uchar,
        dest: *mut *mut ::std::os::raw::c_uchar,
    );
}
extern "C" {
    pub fn gf_4vect_dot_prod_avx(
        len: ::std::os::raw::c_int,
        vlen: ::std::os::raw::c_int,
        gftbls: *mut ::std::os::raw::c_uchar,
        src: *mut *mut ::std::os::raw::c_uchar,
        dest: *mut *mut ::std::os::raw::c_uchar,
    );
}
extern "C" {
    pub fn gf_4vect_dot_prod_avx2(
        len: ::std::os::raw::c_int,
        vlen: ::std::os::raw::c_int,
        gftbls: *mut ::std::os::raw::c_uchar,
        src: *mut *mut ::std::os::raw::c_uchar,
        dest: *mut *mut ::std::os::raw::c_uchar,
    );
}
extern "C" {
    pub fn gf_5vect_dot_prod_sse(
        len: ::std::os::raw::c_int,
        vlen: ::std::os::raw::c_int,
        gftbls: *mut ::std::os::raw::c_uchar,
        src: *mut *mut ::std::os::raw::c_uchar,
        dest: *mut *mut ::std::os::raw::c_uchar,
    );
}
extern "C" {
    pub fn gf_5vect_dot_prod_avx(
        len: ::std::os::raw::c_int,
        vlen: ::std::os::raw::c_int,
        gftbls: *mut ::std::os::raw::c_uchar,
        src: *mut *mut ::std::os::raw::c_uchar,
        dest: *mut *mut ::std::os::raw::c_uchar,
    );
}
extern "C" {
    pub fn gf_5vect_dot_prod_avx2(
        len: ::std::os::raw::c_int,
        vlen: ::std::os::raw::c_int,
        gftbls: *mut ::std::os::raw::c_uchar,
        src: *mut *mut ::std::os::raw::c_uchar,
        dest: *mut *mut ::std::os::raw::c_uchar,
    );
}
extern "C" {
    pub fn gf_6vect_dot_prod_sse(
        len: ::std::os::raw::c_int,
        vlen: ::std::os::raw::c_int,
        gftbls: *mut ::std::os::raw::c_uchar,
        src: *mut *mut ::std::os::raw::c_uchar,
        dest: *mut *mut ::std::os::raw::c_uchar,
    );
}
extern "C" {
    pub fn gf_6vect_dot_prod_avx(
        len: ::std::os::raw::c_int,
        vlen: ::std::os::raw::c_int,
        gftbls: *mut ::std::os::raw::c_uchar,
        src: *mut *mut ::std::os::raw::c_uchar,
        dest: *mut *mut ::std::os::raw::c_uchar,
    );
}
extern "C" {
    pub fn gf_6vect_dot_prod_avx2(
        len: ::std::os::raw::c_int,
        vlen: ::std::os::raw::c_int,
        gftbls: *mut ::std::os::raw::c_uchar,
        src: *mut *mut ::std::os::raw::c_uchar,
        dest: *mut *mut ::std::os::raw::c_uchar,
    );
}
extern "C" {
    pub fn gf_vect_mad_sse(
        len: ::std::os::raw::c_int,
        vec: ::std::os::raw::c_int,
        vec_i: ::std::os::raw::c_int,
        gftbls: *mut ::std::os::raw::c_uchar,
        src: *mut ::std::os::raw::c_uchar,
        dest: *mut ::std::os::raw::c_uchar,
    );
}
extern "C" {
    pub fn gf_vect_mad_avx(
        len: ::std::os::raw::c_int,
        vec: ::std::os::raw::c_int,
        vec_i: ::std::os::raw::c_int,
        gftbls: *mut ::std::os::raw::c_uchar,
        src: *mut ::std::os::raw::c_uchar,
        dest: *mut ::std::os::raw::c_uchar,
    );
}
extern "C" {
    pub fn gf_vect_mad_avx2(
        len: ::std::os::raw::c_int,
        vec: ::std::os::raw::c_int,
        vec_i: ::std::os::raw::c_int,
        gftbls: *mut ::std::os::raw::c_uchar,
        src: *mut ::std::os::raw::c_uchar,
        dest: *mut ::std::os::raw::c_uchar,
    );
}
extern "C" {
    pub fn gf_2vect_mad_sse(
        len: ::std::os::raw::c_int,
        vec: ::std::os::raw::c_int,
        vec_i: ::std::os::raw::c_int,
        gftbls: *mut ::std::os::raw::c_uchar,
        src: *mut ::std::os::raw::c_uchar,
        dest: *mut *mut ::std::os::raw::c_uchar,
    );
}
extern "C" {
    pub fn gf_2vect_mad_avx(
        len: ::std::os::raw::c_int,
        vec: ::std::os::raw::c_int,
        vec_i: ::std::os::raw::c_int,
        gftbls: *mut ::std::os::raw::c_uchar,
        src: *mut ::std::os::raw::c_uchar,
        dest: *mut *mut ::std::os::raw::c_uchar,
    );
}
extern "C" {
    pub fn gf_2vect_mad_avx2(
        len: ::std::os::raw::c_int,
        vec: ::std::os::raw::c_int,
        vec_i: ::std::os::raw::c_int,
        gftbls: *mut ::std::os::raw::c_uchar,
        src: *mut ::std::os::raw::c_uchar,
        dest: *mut *mut ::std::os::raw::c_uchar,
    );
}
extern "C" {
    pub fn gf_3vect_mad_sse(
        len: ::std::os::raw::c_int,
        vec: ::std::os::raw::c_int,
        vec_i: ::std::os::raw::c_int,
        gftbls: *mut ::std::os::raw::c_uchar,
        src: *mut ::std::os::raw::c_uchar,
        dest: *mut *mut ::std::os::raw::c_uchar,
    );
}
extern "C" {
    pub fn gf_3vect_mad_avx(
        len: ::std::os::raw::c_int,
        vec: ::std::os::raw::c_int,
        vec_i: ::std::os::raw::c_int,
        gftbls: *mut ::std::os::raw::c_uchar,
        src: *mut ::std::os::raw::c_uchar,
        dest: *mut *mut ::std::os::raw::c_uchar,
    );
}
extern "C" {
    pub fn gf_3vect_mad_avx2(
        len: ::std::os::raw::c_int,
        vec: ::std::os::raw::c_int,
        vec_i: ::std::os::raw::c_int,
        gftbls: *mut ::std::os::raw::c_uchar,
        src: *mut ::std::os::raw::c_uchar,
        dest: *mut *mut ::std::os::raw::c_uchar,
    );
}
extern "C" {
    pub fn gf_4vect_mad_sse(
        len: ::std::os::raw::c_int,
        vec: ::std::os::raw::c_int,
        vec_i: ::std::os::raw::c_int,
        gftbls: *mut ::std::os::raw::c_uchar,
        src: *mut ::std::os::raw::c_uchar,
        dest: *mut *mut ::std::os::raw::c_uchar,
    );
}
extern "C" {
    pub fn gf_4vect_mad_avx(
        len: ::std::os::raw::c_int,
        vec: ::std::os::raw::c_int,
        vec_i: ::std::os::raw::c_int,
        gftbls: *mut ::std::os::raw::c_uchar,
        src: *mut ::std::os::raw::c_uchar,
        dest: *mut *mut ::std::os::raw::c_uchar,
    );
}
extern "C" {
    pub fn gf_4vect_mad_avx2(
        len: ::std::os::raw::c_int,
        vec: ::std::os::raw::c_int,
        vec_i: ::std::os::raw::c_int,
        gftbls: *mut ::std::os::raw::c_uchar,
        src: *mut ::std::os::raw::c_uchar,
        dest: *mut *mut ::std::os::raw::c_uchar,
    );
}
extern "C" {
    pub fn gf_5vect_mad_sse(
        len: ::std::os::raw::c_int,
        vec: ::std::os::raw::c_int,
        vec_i: ::std::os::raw::c_int,
        gftbls: *mut ::std::os::raw::c_uchar,
        src: *mut ::std::os::raw::c_uchar,
        dest: *mut *mut ::std::os::raw::c_uchar,
    );
}
extern "C" {
    pub fn gf_5vect_mad_avx(
        len: ::std::os::raw::c_int,
        vec: ::std::os::raw::c_int,
        vec_i: ::std::os::raw::c_int,
        gftbls: *mut ::std::os::raw::c_uchar,
        src: *mut ::std::os::raw::c_uchar,
        dest: *mut *mut ::std::os::raw::c_uchar,
    );
}
extern "C" {
    pub fn gf_5vect_mad_avx2(
        len: ::std::os::raw::c_int,
        vec: ::std::os::raw::c_int,
        vec_i: ::std::os::raw::c_int,
        gftbls: *mut ::std::os::raw::c_uchar,
        src: *mut ::std::os::raw::c_uchar,
        dest: *mut *mut ::std::os::raw::c_uchar,
    );
}
extern "C" {
    pub fn gf_6vect_mad_sse(
        len: ::std::os::raw::c_int,
        vec: ::std::os::raw::c_int,
        vec_i: ::std::os::raw::c_int,
        gftbls: *mut ::std::os::raw::c_uchar,
        src: *mut ::std::os::raw::c_uchar,
        dest: *mut *mut ::std::os::raw::c_uchar,
    );
}
extern "C" {
    pub fn gf_6vect_mad_avx(
        len: ::std::os::raw::c_int,
        vec: ::std::os::raw::c_int,
        vec_i: ::std::os::raw::c_int,
        gftbls: *mut ::std::os::raw::c_uchar,
        src: *mut ::std::os::raw::c_uchar,
        dest: *mut *mut ::std::os::raw::c_uchar,
    );
}
extern "C" {
    pub fn gf_6vect_mad_avx2(
        len: ::std::os::raw::c_int,
        vec: ::std::os::raw::c_int,
        vec_i: ::std::os::raw::c_int,
        gftbls: *mut ::std::os::raw::c_uchar,
        src: *mut ::std::os::raw::c_uchar,
        dest: *mut *mut ::std::os::raw::c_uchar,
    );
}
extern "C" {
    pub fn gf_mul(
        a: ::std::os::raw::c_uchar,
        b: ::std::os::raw::c_uchar,
    ) -> ::std::os::raw::c_uchar;
}
extern "C" {
    pub fn gf_inv(a: ::std::os::raw::c_uchar) -> ::std::os::raw::c_uchar;
}
extern "C" {
    pub fn gf_gen_rs_matrix(
        a: *mut ::std::os::raw::c_uchar,
        m: ::std::os::raw::c_int,
        k: ::std::os::raw::c_int,
    );
}
extern "C" {
    /// Generate a Cauchy matrix of coefficients to be used for encoding.
    ///
    /// Cauchy matrix example of encoding coefficients where high portion of matrix
    /// is identity matrix I and lower portion is constructed as 1/(i + j) | i != j,
    /// i:{0,k-1} j:{k,m-1}.  Any sub-matrix of a Cauchy matrix should be invertable.
    ///
    /// # Arguments
    ///
    /// * `a` - An `m * k` 2D-in-1D array to hold coefficients.
    /// * `m` - Number of rows in matrix corresponding to srcs + parity.
    /// * `k` - Number of columns in matrix corresponding to srcs.
    pub fn gf_gen_cauchy1_matrix(
        a: *mut ::std::os::raw::c_uchar,
        m: ::std::os::raw::c_int,
        k: ::std::os::raw::c_int,
    );
}
extern "C" {
    /// Invert a matrix in GF(2^8).
    ///
    /// Attempts to construct an n x n inverse of the input matrix. Returns non-zero
    /// if singular. Will always destroy input matrix in process.
    ///
    /// # Arguments
    ///
    /// * `in`  - Input matrix, destroyed by invert process.
    /// * `out` - Output matrix such that `in * out = I`, where I is the identity matrix.
    /// * `n`   - Size of matrix (`n * n`).
    pub fn gf_invert_matrix(
        in_: *mut ::std::os::raw::c_uchar,
        out: *mut ::std::os::raw::c_uchar,
        n: ::std::os::raw::c_int,
    ) -> ::std::os::raw::c_int;
}

pub const IGZIP_DIST_TABLE_SIZE: ::std::os::raw::c_uint = 2;
pub const IGZIP_DECODE_OFFSET: ::std::os::raw::c_uint = 0;
pub const IGZIP_LEN_TABLE_SIZE: ::std::os::raw::c_uint = 256;
pub const IGZIP_LIT_TABLE_SIZE: ::std::os::raw::c_uint = 257;

pub mod isal_zstate_state {
    pub type Type = ::std::os::raw::c_uint;

    pub const ZSTATE_NEW_HDR: Type = 0;
    pub const ZSTATE_HDR: Type = 1;
    pub const ZSTATE_CREATE_HDR: Type = 2;
    pub const ZSTATE_BODY: Type = 3;
    pub const ZSTATE_FLUSH_READ_BUFFER: Type = 4;
    pub const ZSTATE_FLUSH_ICF_BUFFER: Type = 5;
    pub const ZSTATE_TYPE0_HDR: Type = 6;
    pub const ZSTATE_TYPE0_BODY: Type = 7;
    pub const ZSTATE_SYNC_FLUSH: Type = 8;
    pub const ZSTATE_FLUSH_WRITE_BUFFER: Type = 9;
    pub const ZSTATE_TRL: Type = 10;
    pub const ZSTATE_END: Type = 11;
    pub const ZSTATE_TMP_NEW_HDR: Type = 12;
    pub const ZSTATE_TMP_HDR: Type = 13;
    pub const ZSTATE_TMP_CREATE_HDR: Type = 14;
    pub const ZSTATE_TMP_BODY: Type = 15;
    pub const ZSTATE_TMP_FLUSH_READ_BUFFER: Type = 16;
    pub const ZSTATE_TMP_FLUSH_ICF_BUFFER: Type = 17;
    pub const ZSTATE_TMP_TYPE0_HDR: Type = 18;
    pub const ZSTATE_TMP_TYPE0_BODY: Type = 19;
    pub const ZSTATE_TMP_SYNC_FLUSH: Type = 20;
    pub const ZSTATE_TMP_FLUSH_WRITE_BUFFER: Type = 21;
    pub const ZSTATE_TMP_TRL: Type = 22;
    pub const ZSTATE_TMP_END: Type = 23;
}

pub mod isal_block_state {
    pub type Type = ::std::os::raw::c_uint;

    pub const ISAL_BLOCK_NEW_HDR: Type = 0;
    pub const ISAL_BLOCK_HDR: Type = 1;
    pub const ISAL_BLOCK_TYPE0: Type = 2;
    pub const ISAL_BLOCK_CODED: Type = 3;
    pub const ISAL_BLOCK_INPUT_DONE: Type = 4;
    pub const ISAL_BLOCK_FINISH: Type = 5;
    pub const ISAL_GZIP_EXTRA_LEN: Type = 6;
    pub const ISAL_GZIP_EXTRA: Type = 7;
    pub const ISAL_GZIP_NAME: Type = 8;
    pub const ISAL_GZIP_COMMENT: Type = 9;
    pub const ISAL_GZIP_HCRC: Type = 10;
    pub const ISAL_ZLIB_DICT: Type = 11;
    pub const ISAL_CHECKSUM_CHECK: Type = 12;
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct isal_huff_histogram {
    pub lit_len_histogram: [u64; 286usize],
    pub dist_histogram: [u64; 30usize],
    pub hash_table: [u16; 8192usize],
}
#[test]
fn bindgen_test_layout_isal_huff_histogram() {
    const UNINIT: ::std::mem::MaybeUninit<isal_huff_histogram> = ::std::mem::MaybeUninit::uninit();
    let ptr = UNINIT.as_ptr();
    assert_eq!(
        ::std::mem::size_of::<isal_huff_histogram>(),
        18912usize,
        concat!("Size of: ", stringify!(isal_huff_histogram))
    );
    assert_eq!(
        ::std::mem::align_of::<isal_huff_histogram>(),
        8usize,
        concat!("Alignment of ", stringify!(isal_huff_histogram))
    );
    assert_eq!(
        unsafe { ::std::ptr::addr_of!((*ptr).lit_len_histogram) as usize - ptr as usize },
        0usize,
        concat!(
            "Offset of field: ",
            stringify!(isal_huff_histogram),
            "::",
            stringify!(lit_len_histogram)
        )
    );
    assert_eq!(
        unsafe { ::std::ptr::addr_of!((*ptr).dist_histogram) as usize - ptr as usize },
        2288usize,
        concat!(
            "Offset of field: ",
            stringify!(isal_huff_histogram),
            "::",
            stringify!(dist_histogram)
        )
    );
    assert_eq!(
        unsafe { ::std::ptr::addr_of!((*ptr).hash_table) as usize - ptr as usize },
        2528usize,
        concat!(
            "Offset of field: ",
            stringify!(isal_huff_histogram),
            "::",
            stringify!(hash_table)
        )
    );
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct isal_mod_hist {
    pub d_hist: [u32; 30usize],
    pub ll_hist: [u32; 513usize],
}
#[test]
fn bindgen_test_layout_isal_mod_hist() {
    const UNINIT: ::std::mem::MaybeUninit<isal_mod_hist> = ::std::mem::MaybeUninit::uninit();
    let ptr = UNINIT.as_ptr();
    assert_eq!(
        ::std::mem::size_of::<isal_mod_hist>(),
        2172usize,
        concat!("Size of: ", stringify!(isal_mod_hist))
    );
    assert_eq!(
        ::std::mem::align_of::<isal_mod_hist>(),
        4usize,
        concat!("Alignment of ", stringify!(isal_mod_hist))
    );
    assert_eq!(
        unsafe { ::std::ptr::addr_of!((*ptr).d_hist) as usize - ptr as usize },
        0usize,
        concat!(
            "Offset of field: ",
            stringify!(isal_mod_hist),
            "::",
            stringify!(d_hist)
        )
    );
    assert_eq!(
        unsafe { ::std::ptr::addr_of!((*ptr).ll_hist) as usize - ptr as usize },
        120usize,
        concat!(
            "Offset of field: ",
            stringify!(isal_mod_hist),
            "::",
            stringify!(ll_hist)
        )
    );
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct BitBuf2 {
    pub m_bits: u64,
    pub m_bit_count: u32,
    pub m_out_buf: *mut u8,
    pub m_out_end: *mut u8,
    pub m_out_start: *mut u8,
}
#[test]
fn bindgen_test_layout_BitBuf2() {
    const UNINIT: ::std::mem::MaybeUninit<BitBuf2> = ::std::mem::MaybeUninit::uninit();
    let ptr = UNINIT.as_ptr();
    assert_eq!(
        ::std::mem::size_of::<BitBuf2>(),
        40usize,
        concat!("Size of: ", stringify!(BitBuf2))
    );
    assert_eq!(
        ::std::mem::align_of::<BitBuf2>(),
        8usize,
        concat!("Alignment of ", stringify!(BitBuf2))
    );
    assert_eq!(
        unsafe { ::std::ptr::addr_of!((*ptr).m_bits) as usize - ptr as usize },
        0usize,
        concat!(
            "Offset of field: ",
            stringify!(BitBuf2),
            "::",
            stringify!(m_bits)
        )
    );
    assert_eq!(
        unsafe { ::std::ptr::addr_of!((*ptr).m_bit_count) as usize - ptr as usize },
        8usize,
        concat!(
            "Offset of field: ",
            stringify!(BitBuf2),
            "::",
            stringify!(m_bit_count)
        )
    );
    assert_eq!(
        unsafe { ::std::ptr::addr_of!((*ptr).m_out_buf) as usize - ptr as usize },
        16usize,
        concat!(
            "Offset of field: ",
            stringify!(BitBuf2),
            "::",
            stringify!(m_out_buf)
        )
    );
    assert_eq!(
        unsafe { ::std::ptr::addr_of!((*ptr).m_out_end) as usize - ptr as usize },
        24usize,
        concat!(
            "Offset of field: ",
            stringify!(BitBuf2),
            "::",
            stringify!(m_out_end)
        )
    );
    assert_eq!(
        unsafe { ::std::ptr::addr_of!((*ptr).m_out_start) as usize - ptr as usize },
        32usize,
        concat!(
            "Offset of field: ",
            stringify!(BitBuf2),
            "::",
            stringify!(m_out_start)
        )
    );
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct isal_zlib_header {
    pub info: u32,
    pub level: u32,
    pub dict_id: u32,
    pub dict_flag: u32,
}
#[test]
fn bindgen_test_layout_isal_zlib_header() {
    const UNINIT: ::std::mem::MaybeUninit<isal_zlib_header> = ::std::mem::MaybeUninit::uninit();
    let ptr = UNINIT.as_ptr();
    assert_eq!(
        ::std::mem::size_of::<isal_zlib_header>(),
        16usize,
        concat!("Size of: ", stringify!(isal_zlib_header))
    );
    assert_eq!(
        ::std::mem::align_of::<isal_zlib_header>(),
        4usize,
        concat!("Alignment of ", stringify!(isal_zlib_header))
    );
    assert_eq!(
        unsafe { ::std::ptr::addr_of!((*ptr).info) as usize - ptr as usize },
        0usize,
        concat!(
            "Offset of field: ",
            stringify!(isal_zlib_header),
            "::",
            stringify!(info)
        )
    );
    assert_eq!(
        unsafe { ::std::ptr::addr_of!((*ptr).level) as usize - ptr as usize },
        4usize,
        concat!(
            "Offset of field: ",
            stringify!(isal_zlib_header),
            "::",
            stringify!(level)
        )
    );
    assert_eq!(
        unsafe { ::std::ptr::addr_of!((*ptr).dict_id) as usize - ptr as usize },
        8usize,
        concat!(
            "Offset of field: ",
            stringify!(isal_zlib_header),
            "::",
            stringify!(dict_id)
        )
    );
    assert_eq!(
        unsafe { ::std::ptr::addr_of!((*ptr).dict_flag) as usize - ptr as usize },
        12usize,
        concat!(
            "Offset of field: ",
            stringify!(isal_zlib_header),
            "::",
            stringify!(dict_flag)
        )
    );
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct isal_gzip_header {
    pub text: u32,
    pub time: u32,
    pub xflags: u32,
    pub os: u32,
    pub extra: *mut u8,
    pub extra_buf_len: u32,
    pub extra_len: u32,
    pub name: *mut ::std::os::raw::c_char,
    pub name_buf_len: u32,
    pub comment: *mut ::std::os::raw::c_char,
    pub comment_buf_len: u32,
    pub hcrc: u32,
    pub flags: u32,
}
#[test]
fn bindgen_test_layout_isal_gzip_header() {
    const UNINIT: ::std::mem::MaybeUninit<isal_gzip_header> = ::std::mem::MaybeUninit::uninit();
    let ptr = UNINIT.as_ptr();
    assert_eq!(
        ::std::mem::size_of::<isal_gzip_header>(),
        72usize,
        concat!("Size of: ", stringify!(isal_gzip_header))
    );
    assert_eq!(
        ::std::mem::align_of::<isal_gzip_header>(),
        8usize,
        concat!("Alignment of ", stringify!(isal_gzip_header))
    );
    assert_eq!(
        unsafe { ::std::ptr::addr_of!((*ptr).text) as usize - ptr as usize },
        0usize,
        concat!(
            "Offset of field: ",
            stringify!(isal_gzip_header),
            "::",
            stringify!(text)
        )
    );
    assert_eq!(
        unsafe { ::std::ptr::addr_of!((*ptr).time) as usize - ptr as usize },
        4usize,
        concat!(
            "Offset of field: ",
            stringify!(isal_gzip_header),
            "::",
            stringify!(time)
        )
    );
    assert_eq!(
        unsafe { ::std::ptr::addr_of!((*ptr).xflags) as usize - ptr as usize },
        8usize,
        concat!(
            "Offset of field: ",
            stringify!(isal_gzip_header),
            "::",
            stringify!(xflags)
        )
    );
    assert_eq!(
        unsafe { ::std::ptr::addr_of!((*ptr).os) as usize - ptr as usize },
        12usize,
        concat!(
            "Offset of field: ",
            stringify!(isal_gzip_header),
            "::",
            stringify!(os)
        )
    );
    assert_eq!(
        unsafe { ::std::ptr::addr_of!((*ptr).extra) as usize - ptr as usize },
        16usize,
        concat!(
            "Offset of field: ",
            stringify!(isal_gzip_header),
            "::",
            stringify!(extra)
        )
    );
    assert_eq!(
        unsafe { ::std::ptr::addr_of!((*ptr).extra_buf_len) as usize - ptr as usize },
        24usize,
        concat!(
            "Offset of field: ",
            stringify!(isal_gzip_header),
            "::",
            stringify!(extra_buf_len)
        )
    );
    assert_eq!(
        unsafe { ::std::ptr::addr_of!((*ptr).extra_len) as usize - ptr as usize },
        28usize,
        concat!(
            "Offset of field: ",
            stringify!(isal_gzip_header),
            "::",
            stringify!(extra_len)
        )
    );
    assert_eq!(
        unsafe { ::std::ptr::addr_of!((*ptr).name) as usize - ptr as usize },
        32usize,
        concat!(
            "Offset of field: ",
            stringify!(isal_gzip_header),
            "::",
            stringify!(name)
        )
    );
    assert_eq!(
        unsafe { ::std::ptr::addr_of!((*ptr).name_buf_len) as usize - ptr as usize },
        40usize,
        concat!(
            "Offset of field: ",
            stringify!(isal_gzip_header),
            "::",
            stringify!(name_buf_len)
        )
    );
    assert_eq!(
        unsafe { ::std::ptr::addr_of!((*ptr).comment) as usize - ptr as usize },
        48usize,
        concat!(
            "Offset of field: ",
            stringify!(isal_gzip_header),
            "::",
            stringify!(comment)
        )
    );
    assert_eq!(
        unsafe { ::std::ptr::addr_of!((*ptr).comment_buf_len) as usize - ptr as usize },
        56usize,
        concat!(
            "Offset of field: ",
            stringify!(isal_gzip_header),
            "::",
            stringify!(comment_buf_len)
        )
    );
    assert_eq!(
        unsafe { ::std::ptr::addr_of!((*ptr).hcrc) as usize - ptr as usize },
        60usize,
        concat!(
            "Offset of field: ",
            stringify!(isal_gzip_header),
            "::",
            stringify!(hcrc)
        )
    );
    assert_eq!(
        unsafe { ::std::ptr::addr_of!((*ptr).flags) as usize - ptr as usize },
        64usize,
        concat!(
            "Offset of field: ",
            stringify!(isal_gzip_header),
            "::",
            stringify!(flags)
        )
    );
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct isal_zstate {
    pub total_in_start: u32,
    pub block_next: u32,
    pub block_end: u32,
    pub dist_mask: u32,
    pub hash_mask: u32,
    pub state: isal_zstate_state::Type,
    pub bitbuf: BitBuf2,
    pub crc: u32,
    pub has_wrap_hdr: u8,
    pub has_eob_hdr: u8,
    pub has_eob: u8,
    pub has_hist: u8,
    pub has_level_buf_init: u16,
    pub count: u32,
    pub tmp_out_buff: [u8; 16usize],
    pub tmp_out_start: u32,
    pub tmp_out_end: u32,
    pub b_bytes_valid: u32,
    pub b_bytes_processed: u32,
    pub buffer: [u8; 65824usize],
    pub head: [u16; 8192usize],
}
#[test]
fn bindgen_test_layout_isal_zstate() {
    const UNINIT: ::std::mem::MaybeUninit<isal_zstate> = ::std::mem::MaybeUninit::uninit();
    let ptr = UNINIT.as_ptr();
    assert_eq!(
        ::std::mem::size_of::<isal_zstate>(),
        82320usize,
        concat!("Size of: ", stringify!(isal_zstate))
    );
    assert_eq!(
        ::std::mem::align_of::<isal_zstate>(),
        8usize,
        concat!("Alignment of ", stringify!(isal_zstate))
    );
    assert_eq!(
        unsafe { ::std::ptr::addr_of!((*ptr).total_in_start) as usize - ptr as usize },
        0usize,
        concat!(
            "Offset of field: ",
            stringify!(isal_zstate),
            "::",
            stringify!(total_in_start)
        )
    );
    assert_eq!(
        unsafe { ::std::ptr::addr_of!((*ptr).block_next) as usize - ptr as usize },
        4usize,
        concat!(
            "Offset of field: ",
            stringify!(isal_zstate),
            "::",
            stringify!(block_next)
        )
    );
    assert_eq!(
        unsafe { ::std::ptr::addr_of!((*ptr).block_end) as usize - ptr as usize },
        8usize,
        concat!(
            "Offset of field: ",
            stringify!(isal_zstate),
            "::",
            stringify!(block_end)
        )
    );
    assert_eq!(
        unsafe { ::std::ptr::addr_of!((*ptr).dist_mask) as usize - ptr as usize },
        12usize,
        concat!(
            "Offset of field: ",
            stringify!(isal_zstate),
            "::",
            stringify!(dist_mask)
        )
    );
    assert_eq!(
        unsafe { ::std::ptr::addr_of!((*ptr).hash_mask) as usize - ptr as usize },
        16usize,
        concat!(
            "Offset of field: ",
            stringify!(isal_zstate),
            "::",
            stringify!(hash_mask)
        )
    );
    assert_eq!(
        unsafe { ::std::ptr::addr_of!((*ptr).state) as usize - ptr as usize },
        20usize,
        concat!(
            "Offset of field: ",
            stringify!(isal_zstate),
            "::",
            stringify!(state)
        )
    );
    assert_eq!(
        unsafe { ::std::ptr::addr_of!((*ptr).bitbuf) as usize - ptr as usize },
        24usize,
        concat!(
            "Offset of field: ",
            stringify!(isal_zstate),
            "::",
            stringify!(bitbuf)
        )
    );
    assert_eq!(
        unsafe { ::std::ptr::addr_of!((*ptr).crc) as usize - ptr as usize },
        64usize,
        concat!(
            "Offset of field: ",
            stringify!(isal_zstate),
            "::",
            stringify!(crc)
        )
    );
    assert_eq!(
        unsafe { ::std::ptr::addr_of!((*ptr).has_wrap_hdr) as usize - ptr as usize },
        68usize,
        concat!(
            "Offset of field: ",
            stringify!(isal_zstate),
            "::",
            stringify!(has_wrap_hdr)
        )
    );
    assert_eq!(
        unsafe { ::std::ptr::addr_of!((*ptr).has_eob_hdr) as usize - ptr as usize },
        69usize,
        concat!(
            "Offset of field: ",
            stringify!(isal_zstate),
            "::",
            stringify!(has_eob_hdr)
        )
    );
    assert_eq!(
        unsafe { ::std::ptr::addr_of!((*ptr).has_eob) as usize - ptr as usize },
        70usize,
        concat!(
            "Offset of field: ",
            stringify!(isal_zstate),
            "::",
            stringify!(has_eob)
        )
    );
    assert_eq!(
        unsafe { ::std::ptr::addr_of!((*ptr).has_hist) as usize - ptr as usize },
        71usize,
        concat!(
            "Offset of field: ",
            stringify!(isal_zstate),
            "::",
            stringify!(has_hist)
        )
    );
    assert_eq!(
        unsafe { ::std::ptr::addr_of!((*ptr).has_level_buf_init) as usize - ptr as usize },
        72usize,
        concat!(
            "Offset of field: ",
            stringify!(isal_zstate),
            "::",
            stringify!(has_level_buf_init)
        )
    );
    assert_eq!(
        unsafe { ::std::ptr::addr_of!((*ptr).count) as usize - ptr as usize },
        76usize,
        concat!(
            "Offset of field: ",
            stringify!(isal_zstate),
            "::",
            stringify!(count)
        )
    );
    assert_eq!(
        unsafe { ::std::ptr::addr_of!((*ptr).tmp_out_buff) as usize - ptr as usize },
        80usize,
        concat!(
            "Offset of field: ",
            stringify!(isal_zstate),
            "::",
            stringify!(tmp_out_buff)
        )
    );
    assert_eq!(
        unsafe { ::std::ptr::addr_of!((*ptr).tmp_out_start) as usize - ptr as usize },
        96usize,
        concat!(
            "Offset of field: ",
            stringify!(isal_zstate),
            "::",
            stringify!(tmp_out_start)
        )
    );
    assert_eq!(
        unsafe { ::std::ptr::addr_of!((*ptr).tmp_out_end) as usize - ptr as usize },
        100usize,
        concat!(
            "Offset of field: ",
            stringify!(isal_zstate),
            "::",
            stringify!(tmp_out_end)
        )
    );
    assert_eq!(
        unsafe { ::std::ptr::addr_of!((*ptr).b_bytes_valid) as usize - ptr as usize },
        104usize,
        concat!(
            "Offset of field: ",
            stringify!(isal_zstate),
            "::",
            stringify!(b_bytes_valid)
        )
    );
    assert_eq!(
        unsafe { ::std::ptr::addr_of!((*ptr).b_bytes_processed) as usize - ptr as usize },
        108usize,
        concat!(
            "Offset of field: ",
            stringify!(isal_zstate),
            "::",
            stringify!(b_bytes_processed)
        )
    );
    assert_eq!(
        unsafe { ::std::ptr::addr_of!((*ptr).buffer) as usize - ptr as usize },
        112usize,
        concat!(
            "Offset of field: ",
            stringify!(isal_zstate),
            "::",
            stringify!(buffer)
        )
    );
    assert_eq!(
        unsafe { ::std::ptr::addr_of!((*ptr).head) as usize - ptr as usize },
        65936usize,
        concat!(
            "Offset of field: ",
            stringify!(isal_zstate),
            "::",
            stringify!(head)
        )
    );
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct isal_hufftables {
    pub deflate_hdr: [u8; 328usize],
    pub deflate_hdr_count: u32,
    pub deflate_hdr_extra_bits: u32,
    pub dist_table: [u32; 2usize],
    pub len_table: [u32; 256usize],
    pub lit_table: [u16; 257usize],
    pub lit_table_sizes: [u8; 257usize],
    pub dcodes: [u16; 30usize],
    pub dcodes_sizes: [u8; 30usize],
}
#[test]
fn bindgen_test_layout_isal_hufftables() {
    const UNINIT: ::std::mem::MaybeUninit<isal_hufftables> = ::std::mem::MaybeUninit::uninit();
    let ptr = UNINIT.as_ptr();
    assert_eq!(
        ::std::mem::size_of::<isal_hufftables>(),
        2232usize,
        concat!("Size of: ", stringify!(isal_hufftables))
    );
    assert_eq!(
        ::std::mem::align_of::<isal_hufftables>(),
        4usize,
        concat!("Alignment of ", stringify!(isal_hufftables))
    );
    assert_eq!(
        unsafe { ::std::ptr::addr_of!((*ptr).deflate_hdr) as usize - ptr as usize },
        0usize,
        concat!(
            "Offset of field: ",
            stringify!(isal_hufftables),
            "::",
            stringify!(deflate_hdr)
        )
    );
    assert_eq!(
        unsafe { ::std::ptr::addr_of!((*ptr).deflate_hdr_count) as usize - ptr as usize },
        328usize,
        concat!(
            "Offset of field: ",
            stringify!(isal_hufftables),
            "::",
            stringify!(deflate_hdr_count)
        )
    );
    assert_eq!(
        unsafe { ::std::ptr::addr_of!((*ptr).deflate_hdr_extra_bits) as usize - ptr as usize },
        332usize,
        concat!(
            "Offset of field: ",
            stringify!(isal_hufftables),
            "::",
            stringify!(deflate_hdr_extra_bits)
        )
    );
    assert_eq!(
        unsafe { ::std::ptr::addr_of!((*ptr).dist_table) as usize - ptr as usize },
        336usize,
        concat!(
            "Offset of field: ",
            stringify!(isal_hufftables),
            "::",
            stringify!(dist_table)
        )
    );
    assert_eq!(
        unsafe { ::std::ptr::addr_of!((*ptr).len_table) as usize - ptr as usize },
        344usize,
        concat!(
            "Offset of field: ",
            stringify!(isal_hufftables),
            "::",
            stringify!(len_table)
        )
    );
    assert_eq!(
        unsafe { ::std::ptr::addr_of!((*ptr).lit_table) as usize - ptr as usize },
        1368usize,
        concat!(
            "Offset of field: ",
            stringify!(isal_hufftables),
            "::",
            stringify!(lit_table)
        )
    );
    assert_eq!(
        unsafe { ::std::ptr::addr_of!((*ptr).lit_table_sizes) as usize - ptr as usize },
        1882usize,
        concat!(
            "Offset of field: ",
            stringify!(isal_hufftables),
            "::",
            stringify!(lit_table_sizes)
        )
    );
    assert_eq!(
        unsafe { ::std::ptr::addr_of!((*ptr).dcodes) as usize - ptr as usize },
        2140usize,
        concat!(
            "Offset of field: ",
            stringify!(isal_hufftables),
            "::",
            stringify!(dcodes)
        )
    );
    assert_eq!(
        unsafe { ::std::ptr::addr_of!((*ptr).dcodes_sizes) as usize - ptr as usize },
        2200usize,
        concat!(
            "Offset of field: ",
            stringify!(isal_hufftables),
            "::",
            stringify!(dcodes_sizes)
        )
    );
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct isal_zstream {
    pub next_in: *mut u8,
    pub avail_in: u32,
    pub total_in: u32,
    pub next_out: *mut u8,
    pub avail_out: u32,
    pub total_out: u32,
    pub hufftables: *mut isal_hufftables,
    pub level: u32,
    pub level_buf_size: u32,
    pub level_buf: *mut u8,
    pub end_of_stream: u16,
    pub flush: u16,
    pub gzip_flag: u16,
    pub hist_bits: u16,
    pub internal_state: isal_zstate,
}
#[test]
fn bindgen_test_layout_isal_zstream() {
    const UNINIT: ::std::mem::MaybeUninit<isal_zstream> = ::std::mem::MaybeUninit::uninit();
    let ptr = UNINIT.as_ptr();
    assert_eq!(
        ::std::mem::size_of::<isal_zstream>(),
        82384usize,
        concat!("Size of: ", stringify!(isal_zstream))
    );
    assert_eq!(
        ::std::mem::align_of::<isal_zstream>(),
        8usize,
        concat!("Alignment of ", stringify!(isal_zstream))
    );
    assert_eq!(
        unsafe { ::std::ptr::addr_of!((*ptr).next_in) as usize - ptr as usize },
        0usize,
        concat!(
            "Offset of field: ",
            stringify!(isal_zstream),
            "::",
            stringify!(next_in)
        )
    );
    assert_eq!(
        unsafe { ::std::ptr::addr_of!((*ptr).avail_in) as usize - ptr as usize },
        8usize,
        concat!(
            "Offset of field: ",
            stringify!(isal_zstream),
            "::",
            stringify!(avail_in)
        )
    );
    assert_eq!(
        unsafe { ::std::ptr::addr_of!((*ptr).total_in) as usize - ptr as usize },
        12usize,
        concat!(
            "Offset of field: ",
            stringify!(isal_zstream),
            "::",
            stringify!(total_in)
        )
    );
    assert_eq!(
        unsafe { ::std::ptr::addr_of!((*ptr).next_out) as usize - ptr as usize },
        16usize,
        concat!(
            "Offset of field: ",
            stringify!(isal_zstream),
            "::",
            stringify!(next_out)
        )
    );
    assert_eq!(
        unsafe { ::std::ptr::addr_of!((*ptr).avail_out) as usize - ptr as usize },
        24usize,
        concat!(
            "Offset of field: ",
            stringify!(isal_zstream),
            "::",
            stringify!(avail_out)
        )
    );
    assert_eq!(
        unsafe { ::std::ptr::addr_of!((*ptr).total_out) as usize - ptr as usize },
        28usize,
        concat!(
            "Offset of field: ",
            stringify!(isal_zstream),
            "::",
            stringify!(total_out)
        )
    );
    assert_eq!(
        unsafe { ::std::ptr::addr_of!((*ptr).hufftables) as usize - ptr as usize },
        32usize,
        concat!(
            "Offset of field: ",
            stringify!(isal_zstream),
            "::",
            stringify!(hufftables)
        )
    );
    assert_eq!(
        unsafe { ::std::ptr::addr_of!((*ptr).level) as usize - ptr as usize },
        40usize,
        concat!(
            "Offset of field: ",
            stringify!(isal_zstream),
            "::",
            stringify!(level)
        )
    );
    assert_eq!(
        unsafe { ::std::ptr::addr_of!((*ptr).level_buf_size) as usize - ptr as usize },
        44usize,
        concat!(
            "Offset of field: ",
            stringify!(isal_zstream),
            "::",
            stringify!(level_buf_size)
        )
    );
    assert_eq!(
        unsafe { ::std::ptr::addr_of!((*ptr).level_buf) as usize - ptr as usize },
        48usize,
        concat!(
            "Offset of field: ",
            stringify!(isal_zstream),
            "::",
            stringify!(level_buf)
        )
    );
    assert_eq!(
        unsafe { ::std::ptr::addr_of!((*ptr).end_of_stream) as usize - ptr as usize },
        56usize,
        concat!(
            "Offset of field: ",
            stringify!(isal_zstream),
            "::",
            stringify!(end_of_stream)
        )
    );
    assert_eq!(
        unsafe { ::std::ptr::addr_of!((*ptr).flush) as usize - ptr as usize },
        58usize,
        concat!(
            "Offset of field: ",
            stringify!(isal_zstream),
            "::",
            stringify!(flush)
        )
    );
    assert_eq!(
        unsafe { ::std::ptr::addr_of!((*ptr).gzip_flag) as usize - ptr as usize },
        60usize,
        concat!(
            "Offset of field: ",
            stringify!(isal_zstream),
            "::",
            stringify!(gzip_flag)
        )
    );
    assert_eq!(
        unsafe { ::std::ptr::addr_of!((*ptr).hist_bits) as usize - ptr as usize },
        62usize,
        concat!(
            "Offset of field: ",
            stringify!(isal_zstream),
            "::",
            stringify!(hist_bits)
        )
    );
    assert_eq!(
        unsafe { ::std::ptr::addr_of!((*ptr).internal_state) as usize - ptr as usize },
        64usize,
        concat!(
            "Offset of field: ",
            stringify!(isal_zstream),
            "::",
            stringify!(internal_state)
        )
    );
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct inflate_huff_code_large {
    pub short_code_lookup: [u32; 4096usize],
    pub long_code_lookup: [u16; 1264usize],
}

#[test]
fn bindgen_test_layout_inflate_huff_code_large() {
    const UNINIT: ::std::mem::MaybeUninit<inflate_huff_code_large> =
        ::std::mem::MaybeUninit::uninit();
    let ptr = UNINIT.as_ptr();
    assert_eq!(
        ::std::mem::size_of::<inflate_huff_code_large>(),
        18912usize,
        concat!("Size of: ", stringify!(inflate_huff_code_large))
    );
    assert_eq!(
        ::std::mem::align_of::<inflate_huff_code_large>(),
        4usize,
        concat!("Alignment of ", stringify!(inflate_huff_code_large))
    );
    assert_eq!(
        unsafe { ::std::ptr::addr_of!((*ptr).short_code_lookup) as usize - ptr as usize },
        0usize,
        concat!(
            "Offset of field: ",
            stringify!(inflate_huff_code_large),
            "::",
            stringify!(short_code_lookup)
        )
    );
    assert_eq!(
        unsafe { ::std::ptr::addr_of!((*ptr).long_code_lookup) as usize - ptr as usize },
        16384usize,
        concat!(
            "Offset of field: ",
            stringify!(inflate_huff_code_large),
            "::",
            stringify!(long_code_lookup)
        )
    );
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct inflate_huff_code_small {
    pub short_code_lookup: [u16; 1024usize],
    pub long_code_lookup: [u16; 80usize],
}

#[test]
fn bindgen_test_layout_inflate_huff_code_small() {
    const UNINIT: ::std::mem::MaybeUninit<inflate_huff_code_small> =
        ::std::mem::MaybeUninit::uninit();
    let ptr = UNINIT.as_ptr();
    assert_eq!(
        ::std::mem::size_of::<inflate_huff_code_small>(),
        2208usize,
        concat!("Size of: ", stringify!(inflate_huff_code_small))
    );
    assert_eq!(
        ::std::mem::align_of::<inflate_huff_code_small>(),
        2usize,
        concat!("Alignment of ", stringify!(inflate_huff_code_small))
    );
    assert_eq!(
        unsafe { ::std::ptr::addr_of!((*ptr).short_code_lookup) as usize - ptr as usize },
        0usize,
        concat!(
            "Offset of field: ",
            stringify!(inflate_huff_code_small),
            "::",
            stringify!(short_code_lookup)
        )
    );
    assert_eq!(
        unsafe { ::std::ptr::addr_of!((*ptr).long_code_lookup) as usize - ptr as usize },
        2048usize,
        concat!(
            "Offset of field: ",
            stringify!(inflate_huff_code_small),
            "::",
            stringify!(long_code_lookup)
        )
    );
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct inflate_state {
    pub next_out: *mut u8,
    pub avail_out: u32,
    pub total_out: u32,
    pub next_in: *mut u8,
    pub read_in: u64,
    pub avail_in: u32,
    pub read_in_length: i32,
    pub lit_huff_code: inflate_huff_code_large,
    pub dist_huff_code: inflate_huff_code_small,
    pub block_state: isal_block_state::Type,
    pub dict_length: u32,
    pub bfinal: u32,
    pub crc_flag: u32,
    pub crc: u32,
    pub hist_bits: u32,
    pub __bindgen_anon_1: inflate_state__bindgen_ty_1,
    pub write_overflow_lits: i32,
    pub write_overflow_len: i32,
    pub copy_overflow_length: i32,
    pub copy_overflow_distance: i32,
    pub wrapper_flag: i16,
    pub tmp_in_size: i16,
    pub tmp_out_valid: i32,
    pub tmp_out_processed: i32,
    pub tmp_in_buffer: [u8; 328usize],
    pub tmp_out_buffer: [u8; 65824usize],
}

#[repr(C)]
#[derive(Copy, Clone)]
pub union inflate_state__bindgen_ty_1 {
    pub type0_block_len: i32,
    pub count: i32,
    pub dict_id: u32,
}

#[test]
fn bindgen_test_layout_inflate_state__bindgen_ty_1() {
    const UNINIT: ::std::mem::MaybeUninit<inflate_state__bindgen_ty_1> =
        ::std::mem::MaybeUninit::uninit();
    let ptr = UNINIT.as_ptr();
    assert_eq!(
        ::std::mem::size_of::<inflate_state__bindgen_ty_1>(),
        4usize,
        concat!("Size of: ", stringify!(inflate_state__bindgen_ty_1))
    );
    assert_eq!(
        ::std::mem::align_of::<inflate_state__bindgen_ty_1>(),
        4usize,
        concat!("Alignment of ", stringify!(inflate_state__bindgen_ty_1))
    );
    assert_eq!(
        unsafe { ::std::ptr::addr_of!((*ptr).type0_block_len) as usize - ptr as usize },
        0usize,
        concat!(
            "Offset of field: ",
            stringify!(inflate_state__bindgen_ty_1),
            "::",
            stringify!(type0_block_len)
        )
    );
    assert_eq!(
        unsafe { ::std::ptr::addr_of!((*ptr).count) as usize - ptr as usize },
        0usize,
        concat!(
            "Offset of field: ",
            stringify!(inflate_state__bindgen_ty_1),
            "::",
            stringify!(count)
        )
    );
    assert_eq!(
        unsafe { ::std::ptr::addr_of!((*ptr).dict_id) as usize - ptr as usize },
        0usize,
        concat!(
            "Offset of field: ",
            stringify!(inflate_state__bindgen_ty_1),
            "::",
            stringify!(dict_id)
        )
    );
}
#[test]
fn bindgen_test_layout_inflate_state() {
    const UNINIT: ::std::mem::MaybeUninit<inflate_state> = ::std::mem::MaybeUninit::uninit();
    let ptr = UNINIT.as_ptr();
    assert_eq!(
        ::std::mem::size_of::<inflate_state>(),
        87368usize,
        concat!("Size of: ", stringify!(inflate_state))
    );
    assert_eq!(
        ::std::mem::align_of::<inflate_state>(),
        8usize,
        concat!("Alignment of ", stringify!(inflate_state))
    );
    assert_eq!(
        unsafe { ::std::ptr::addr_of!((*ptr).next_out) as usize - ptr as usize },
        0usize,
        concat!(
            "Offset of field: ",
            stringify!(inflate_state),
            "::",
            stringify!(next_out)
        )
    );
    assert_eq!(
        unsafe { ::std::ptr::addr_of!((*ptr).avail_out) as usize - ptr as usize },
        8usize,
        concat!(
            "Offset of field: ",
            stringify!(inflate_state),
            "::",
            stringify!(avail_out)
        )
    );
    assert_eq!(
        unsafe { ::std::ptr::addr_of!((*ptr).total_out) as usize - ptr as usize },
        12usize,
        concat!(
            "Offset of field: ",
            stringify!(inflate_state),
            "::",
            stringify!(total_out)
        )
    );
    assert_eq!(
        unsafe { ::std::ptr::addr_of!((*ptr).next_in) as usize - ptr as usize },
        16usize,
        concat!(
            "Offset of field: ",
            stringify!(inflate_state),
            "::",
            stringify!(next_in)
        )
    );
    assert_eq!(
        unsafe { ::std::ptr::addr_of!((*ptr).read_in) as usize - ptr as usize },
        24usize,
        concat!(
            "Offset of field: ",
            stringify!(inflate_state),
            "::",
            stringify!(read_in)
        )
    );
    assert_eq!(
        unsafe { ::std::ptr::addr_of!((*ptr).avail_in) as usize - ptr as usize },
        32usize,
        concat!(
            "Offset of field: ",
            stringify!(inflate_state),
            "::",
            stringify!(avail_in)
        )
    );
    assert_eq!(
        unsafe { ::std::ptr::addr_of!((*ptr).read_in_length) as usize - ptr as usize },
        36usize,
        concat!(
            "Offset of field: ",
            stringify!(inflate_state),
            "::",
            stringify!(read_in_length)
        )
    );
    assert_eq!(
        unsafe { ::std::ptr::addr_of!((*ptr).lit_huff_code) as usize - ptr as usize },
        40usize,
        concat!(
            "Offset of field: ",
            stringify!(inflate_state),
            "::",
            stringify!(lit_huff_code)
        )
    );
    assert_eq!(
        unsafe { ::std::ptr::addr_of!((*ptr).dist_huff_code) as usize - ptr as usize },
        18952usize,
        concat!(
            "Offset of field: ",
            stringify!(inflate_state),
            "::",
            stringify!(dist_huff_code)
        )
    );
    assert_eq!(
        unsafe { ::std::ptr::addr_of!((*ptr).block_state) as usize - ptr as usize },
        21160usize,
        concat!(
            "Offset of field: ",
            stringify!(inflate_state),
            "::",
            stringify!(block_state)
        )
    );
    assert_eq!(
        unsafe { ::std::ptr::addr_of!((*ptr).dict_length) as usize - ptr as usize },
        21164usize,
        concat!(
            "Offset of field: ",
            stringify!(inflate_state),
            "::",
            stringify!(dict_length)
        )
    );
    assert_eq!(
        unsafe { ::std::ptr::addr_of!((*ptr).bfinal) as usize - ptr as usize },
        21168usize,
        concat!(
            "Offset of field: ",
            stringify!(inflate_state),
            "::",
            stringify!(bfinal)
        )
    );
    assert_eq!(
        unsafe { ::std::ptr::addr_of!((*ptr).crc_flag) as usize - ptr as usize },
        21172usize,
        concat!(
            "Offset of field: ",
            stringify!(inflate_state),
            "::",
            stringify!(crc_flag)
        )
    );
    assert_eq!(
        unsafe { ::std::ptr::addr_of!((*ptr).crc) as usize - ptr as usize },
        21176usize,
        concat!(
            "Offset of field: ",
            stringify!(inflate_state),
            "::",
            stringify!(crc)
        )
    );
    assert_eq!(
        unsafe { ::std::ptr::addr_of!((*ptr).hist_bits) as usize - ptr as usize },
        21180usize,
        concat!(
            "Offset of field: ",
            stringify!(inflate_state),
            "::",
            stringify!(hist_bits)
        )
    );
    assert_eq!(
        unsafe { ::std::ptr::addr_of!((*ptr).write_overflow_lits) as usize - ptr as usize },
        21188usize,
        concat!(
            "Offset of field: ",
            stringify!(inflate_state),
            "::",
            stringify!(write_overflow_lits)
        )
    );
    assert_eq!(
        unsafe { ::std::ptr::addr_of!((*ptr).write_overflow_len) as usize - ptr as usize },
        21192usize,
        concat!(
            "Offset of field: ",
            stringify!(inflate_state),
            "::",
            stringify!(write_overflow_len)
        )
    );
    assert_eq!(
        unsafe { ::std::ptr::addr_of!((*ptr).copy_overflow_length) as usize - ptr as usize },
        21196usize,
        concat!(
            "Offset of field: ",
            stringify!(inflate_state),
            "::",
            stringify!(copy_overflow_length)
        )
    );
    assert_eq!(
        unsafe { ::std::ptr::addr_of!((*ptr).copy_overflow_distance) as usize - ptr as usize },
        21200usize,
        concat!(
            "Offset of field: ",
            stringify!(inflate_state),
            "::",
            stringify!(copy_overflow_distance)
        )
    );
    assert_eq!(
        unsafe { ::std::ptr::addr_of!((*ptr).wrapper_flag) as usize - ptr as usize },
        21204usize,
        concat!(
            "Offset of field: ",
            stringify!(inflate_state),
            "::",
            stringify!(wrapper_flag)
        )
    );
    assert_eq!(
        unsafe { ::std::ptr::addr_of!((*ptr).tmp_in_size) as usize - ptr as usize },
        21206usize,
        concat!(
            "Offset of field: ",
            stringify!(inflate_state),
            "::",
            stringify!(tmp_in_size)
        )
    );
    assert_eq!(
        unsafe { ::std::ptr::addr_of!((*ptr).tmp_out_valid) as usize - ptr as usize },
        21208usize,
        concat!(
            "Offset of field: ",
            stringify!(inflate_state),
            "::",
            stringify!(tmp_out_valid)
        )
    );
    assert_eq!(
        unsafe { ::std::ptr::addr_of!((*ptr).tmp_out_processed) as usize - ptr as usize },
        21212usize,
        concat!(
            "Offset of field: ",
            stringify!(inflate_state),
            "::",
            stringify!(tmp_out_processed)
        )
    );
    assert_eq!(
        unsafe { ::std::ptr::addr_of!((*ptr).tmp_in_buffer) as usize - ptr as usize },
        21216usize,
        concat!(
            "Offset of field: ",
            stringify!(inflate_state),
            "::",
            stringify!(tmp_in_buffer)
        )
    );
    assert_eq!(
        unsafe { ::std::ptr::addr_of!((*ptr).tmp_out_buffer) as usize - ptr as usize },
        21544usize,
        concat!(
            "Offset of field: ",
            stringify!(inflate_state),
            "::",
            stringify!(tmp_out_buffer)
        )
    );
}
extern "C" {
    pub fn isal_update_histogram(
        in_stream: *mut u8,
        length: ::std::os::raw::c_int,
        histogram: *mut isal_huff_histogram,
    );
}
extern "C" {
    pub fn isal_create_hufftables(
        hufftables: *mut isal_hufftables,
        histogram: *mut isal_huff_histogram,
    ) -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn isal_create_hufftables_subset(
        hufftables: *mut isal_hufftables,
        histogram: *mut isal_huff_histogram,
    ) -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn isal_deflate_init(stream: *mut isal_zstream);
}
extern "C" {
    pub fn isal_deflate_reset(stream: *mut isal_zstream);
}
extern "C" {
    pub fn isal_gzip_header_init(gz_hdr: *mut isal_gzip_header);
}
extern "C" {
    pub fn isal_write_gzip_header(stream: *mut isal_zstream, gz_hdr: *mut isal_gzip_header) -> u32;
}
extern "C" {
    pub fn isal_write_zlib_header(stream: *mut isal_zstream, z_hdr: *mut isal_zlib_header) -> u32;
}
extern "C" {
    pub fn isal_deflate_set_hufftables(
        stream: *mut isal_zstream,
        hufftables: *mut isal_hufftables,
        type_: ::std::os::raw::c_int,
    ) -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn isal_deflate_stateless_init(stream: *mut isal_zstream);
}
extern "C" {
    pub fn isal_deflate_set_dict(
        stream: *mut isal_zstream,
        dict: *mut u8,
        dict_len: u32,
    ) -> ::std::os::raw::c_int;
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct isal_dict {
    pub params: u32,
    pub level: u32,
    pub hist_size: u32,
    pub hash_size: u32,
    pub history: [u8; 32768usize],
    pub hashtable: [u16; 32768usize],
}
#[test]
fn bindgen_test_layout_isal_dict() {
    const UNINIT: ::std::mem::MaybeUninit<isal_dict> = ::std::mem::MaybeUninit::uninit();
    let ptr = UNINIT.as_ptr();
    assert_eq!(
        ::std::mem::size_of::<isal_dict>(),
        98320usize,
        concat!("Size of: ", stringify!(isal_dict))
    );
    assert_eq!(
        ::std::mem::align_of::<isal_dict>(),
        4usize,
        concat!("Alignment of ", stringify!(isal_dict))
    );
    assert_eq!(
        unsafe { ::std::ptr::addr_of!((*ptr).params) as usize - ptr as usize },
        0usize,
        concat!(
            "Offset of field: ",
            stringify!(isal_dict),
            "::",
            stringify!(params)
        )
    );
    assert_eq!(
        unsafe { ::std::ptr::addr_of!((*ptr).level) as usize - ptr as usize },
        4usize,
        concat!(
            "Offset of field: ",
            stringify!(isal_dict),
            "::",
            stringify!(level)
        )
    );
    assert_eq!(
        unsafe { ::std::ptr::addr_of!((*ptr).hist_size) as usize - ptr as usize },
        8usize,
        concat!(
            "Offset of field: ",
            stringify!(isal_dict),
            "::",
            stringify!(hist_size)
        )
    );
    assert_eq!(
        unsafe { ::std::ptr::addr_of!((*ptr).hash_size) as usize - ptr as usize },
        12usize,
        concat!(
            "Offset of field: ",
            stringify!(isal_dict),
            "::",
            stringify!(hash_size)
        )
    );
    assert_eq!(
        unsafe { ::std::ptr::addr_of!((*ptr).history) as usize - ptr as usize },
        16usize,
        concat!(
            "Offset of field: ",
            stringify!(isal_dict),
            "::",
            stringify!(history)
        )
    );
    assert_eq!(
        unsafe { ::std::ptr::addr_of!((*ptr).hashtable) as usize - ptr as usize },
        32784usize,
        concat!(
            "Offset of field: ",
            stringify!(isal_dict),
            "::",
            stringify!(hashtable)
        )
    );
}
extern "C" {
    pub fn isal_deflate_process_dict(
        stream: *mut isal_zstream,
        dict_str: *mut isal_dict,
        dict: *mut u8,
        dict_len: u32,
    ) -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn isal_deflate_reset_dict(
        stream: *mut isal_zstream,
        dict_str: *mut isal_dict,
    ) -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn isal_deflate(stream: *mut isal_zstream) -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn isal_deflate_stateless(stream: *mut isal_zstream) -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn isal_inflate_init(state: *mut inflate_state);
}
extern "C" {
    pub fn isal_inflate_reset(state: *mut inflate_state);
}
extern "C" {
    pub fn isal_inflate_set_dict(
        state: *mut inflate_state,
        dict: *mut u8,
        dict_len: u32,
    ) -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn isal_read_gzip_header(
        state: *mut inflate_state,
        gz_hdr: *mut isal_gzip_header,
    ) -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn isal_read_zlib_header(
        state: *mut inflate_state,
        zlib_hdr: *mut isal_zlib_header,
    ) -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn isal_inflate(state: *mut inflate_state) -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn isal_inflate_stateless(state: *mut inflate_state) -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn isal_adler32(init: u32, buf: *const ::std::os::raw::c_uchar, len: u64) -> u32;
}

#[repr(C)]
#[repr(align(16))]
#[derive(Debug, Copy, Clone)]
pub struct max_align_t {
    pub __clang_max_align_nonce1: ::std::os::raw::c_longlong,
    pub __bindgen_padding_0: u64,
    pub __clang_max_align_nonce2: u128,
}
#[test]
fn bindgen_test_layout_max_align_t() {
    const UNINIT: ::std::mem::MaybeUninit<max_align_t> = ::std::mem::MaybeUninit::uninit();
    let ptr = UNINIT.as_ptr();
    assert_eq!(
        ::std::mem::size_of::<max_align_t>(),
        32usize,
        concat!("Size of: ", stringify!(max_align_t))
    );
    assert_eq!(
        ::std::mem::align_of::<max_align_t>(),
        16usize,
        concat!("Alignment of ", stringify!(max_align_t))
    );
    assert_eq!(
        unsafe { ::std::ptr::addr_of!((*ptr).__clang_max_align_nonce1) as usize - ptr as usize },
        0usize,
        concat!(
            "Offset of field: ",
            stringify!(max_align_t),
            "::",
            stringify!(__clang_max_align_nonce1)
        )
    );
    assert_eq!(
        unsafe { ::std::ptr::addr_of!((*ptr).__clang_max_align_nonce2) as usize - ptr as usize },
        16usize,
        concat!(
            "Offset of field: ",
            stringify!(max_align_t),
            "::",
            stringify!(__clang_max_align_nonce2)
        )
    );
}
extern "C" {
    pub fn isal_zero_detect(mem: *mut ::std::os::raw::c_void, len: usize) -> ::std::os::raw::c_int;
}
extern "C" {
    /// Generate XOR parity vector from N sources, runs appropriate version.
    /// Returns 0 on success, other on failure.
    ///
    /// This function determines what instruction sets are enabled and
    /// selects the appropriate version at runtime.
    ///
    /// # Arguments
    ///
    /// * `vects` - Number of source+dest vectors in array. Must be > 2.
    /// * `len`   - Length of each vector in bytes.
    /// * `array` - Array of pointers to source and dest. For XOR the dest is
    ///             the last pointer. ie `array[vects-1]`. Src and dest
    ///             pointers must be aligned to 32B.
    pub fn xor_gen(
        vects: ::std::os::raw::c_int,
        len: ::std::os::raw::c_int,
        array: *mut *mut ::std::os::raw::c_void,
    ) -> ::std::os::raw::c_int;
}
extern "C" {
    /// Checks that array has XOR parity sum of 0 across all vectors, runs appropriate version.
    /// Returns 0 on success, other on failure.
    ///
    /// This function determines what instruction sets are enabled and
    /// selects the appropriate version at runtime.
    ///
    /// # Arguments
    ///
    /// * `vects` - Number of vectors in array. Must be > 1.
    /// * `len`   - Length of each vector in bytes.
    /// * `array` - Array of pointers to vectors. Src and dest pointers
    ///             must be aligned to 16B.
    pub fn xor_check(
        vects: ::std::os::raw::c_int,
        len: ::std::os::raw::c_int,
        array: *mut *mut ::std::os::raw::c_void,
    ) -> ::std::os::raw::c_int;
}
extern "C" {
    /// Generate P+Q parity vectors from N sources, runs appropriate version.
    /// Returns 0 on success, other on failure.
    ///
    /// This function determines what instruction sets are enabled and
    /// selects the appropriate version at runtime.
    ///
    /// # Arguments
    ///
    /// * `vects` - Number of source+dest vectors in array. Must be > 3.
    /// * `len`   - Length of each vector in bytes. Must be 32B aligned.
    /// * `array` - Array of pointers to source and dest. For P+Q the dest
    ///             is the last two pointers. ie `array[vects-2]`,
    ///             `array[vects-1]`.  P and Q parity vectors are
    ///             written to these last two pointers. Src and dest
    ///             pointers must be aligned to 32B.
    pub fn pq_gen(
        vects: ::std::os::raw::c_int,
        len: ::std::os::raw::c_int,
        array: *mut *mut ::std::os::raw::c_void,
    ) -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn pq_check(
        vects: ::std::os::raw::c_int,
        len: ::std::os::raw::c_int,
        array: *mut *mut ::std::os::raw::c_void,
    ) -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn xor_gen_sse(
        vects: ::std::os::raw::c_int,
        len: ::std::os::raw::c_int,
        array: *mut *mut ::std::os::raw::c_void,
    ) -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn xor_gen_avx(
        vects: ::std::os::raw::c_int,
        len: ::std::os::raw::c_int,
        array: *mut *mut ::std::os::raw::c_void,
    ) -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn xor_check_sse(
        vects: ::std::os::raw::c_int,
        len: ::std::os::raw::c_int,
        array: *mut *mut ::std::os::raw::c_void,
    ) -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn pq_gen_sse(
        vects: ::std::os::raw::c_int,
        len: ::std::os::raw::c_int,
        array: *mut *mut ::std::os::raw::c_void,
    ) -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn pq_gen_avx(
        vects: ::std::os::raw::c_int,
        len: ::std::os::raw::c_int,
        array: *mut *mut ::std::os::raw::c_void,
    ) -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn pq_gen_avx2(
        vects: ::std::os::raw::c_int,
        len: ::std::os::raw::c_int,
        array: *mut *mut ::std::os::raw::c_void,
    ) -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn pq_check_sse(
        vects: ::std::os::raw::c_int,
        len: ::std::os::raw::c_int,
        array: *mut *mut ::std::os::raw::c_void,
    ) -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn pq_gen_base(
        vects: ::std::os::raw::c_int,
        len: ::std::os::raw::c_int,
        array: *mut *mut ::std::os::raw::c_void,
    ) -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn xor_gen_base(
        vects: ::std::os::raw::c_int,
        len: ::std::os::raw::c_int,
        array: *mut *mut ::std::os::raw::c_void,
    ) -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn xor_check_base(
        vects: ::std::os::raw::c_int,
        len: ::std::os::raw::c_int,
        array: *mut *mut ::std::os::raw::c_void,
    ) -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn pq_check_base(
        vects: ::std::os::raw::c_int,
        len: ::std::os::raw::c_int,
        array: *mut *mut ::std::os::raw::c_void,
    ) -> ::std::os::raw::c_int;
}
