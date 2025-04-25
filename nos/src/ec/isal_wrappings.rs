//! Wrappers around ISA-L functions.

use std::mem::MaybeUninit;
use std::ptr;

use crate::utils::*;

static ZEROES: [u8; 1 << 20] = [0; 1 << 20];
const ALIGNMENT: usize = 64;

/// XOR all the slices in `src` into `dst`.
#[inline]
pub fn xor_compute(dst: &mut [MaybeUninit<u8>], src: &[&[u8]]) {
    debug_assert!(
        !src.is_empty(),
        "xor_compute: source array must not be empty"
    );

    let len = src[0].len();
    debug_assert!(
        src.iter().all(|chunk| chunk.len() == len),
        "xor_compute: source/destination chunks must have the same length"
    );
    debug_assert!(dst.len() >= len, "xor_compute: destination slice too small");

    // ISA-L only accepts input array with length > 2
    if src.len() == 1 {
        unsafe { ptr::copy_nonoverlapping(src[0].as_ptr(), dst.as_mut_ptr() as _, len) };
        return;
    }

    let mut buffers = Vec::new();
    let mut array = src
        .iter()
        .map(|slice| {
            if slice.as_ptr() as usize % ALIGNMENT != 0 {
                let buf = vec![0; len + ALIGNMENT];
                buffers.push(buf);

                let buf = buffers.last_mut().unwrap();
                let buf_ptr = make_aligned_mut_to(buf.as_mut_ptr(), ALIGNMENT);
                unsafe { ptr::copy_nonoverlapping(slice.as_ptr(), buf_ptr, len) };
                buf_ptr
            } else {
                slice.as_ptr() as *mut u8
            }
        })
        .collect::<Vec<_>>();

    // Make necessary copies to avoid unalignment errors
    let (dst_ptr, buf) = if dst.as_mut_ptr() as usize % ALIGNMENT != 0 {
        let mut buf = <Vec<u8>>::with_capacity(len + ALIGNMENT);
        (
            make_aligned_mut_to(buf.spare_capacity_mut().as_mut_ptr() as *mut _, ALIGNMENT),
            Some(buf),
        )
    } else {
        (dst.as_mut_ptr() as *mut u8, None)
    };

    array.push(dst_ptr);

    // SAFETY: FFI.
    let ret =
        unsafe { isal_sys::xor_gen((src.len() + 1) as i32, len as i32, array.as_mut_ptr() as _) };
    if ret != 0 {
        panic!("isa-l: xor_compute failed with error code {}", ret);
    }

    if buf.is_some() {
        unsafe { ptr::copy_nonoverlapping(dst_ptr, dst.as_mut_ptr() as _, len) };
    }
}

/// Check that the slices XOR to zero.
#[inline]
pub fn xor_check(src: &[&[u8]]) {
    if src.is_empty() {
        return;
    }

    let len = src[0].len();
    debug_assert!(
        src.iter().all(|chunk| chunk.len() == len),
        "xor_check: all chunks must have the same lengths"
    );

    let mut array = src
        .iter()
        .map(|slice| slice.as_ptr() as *mut u8)
        .collect::<Vec<_>>();

    // SAFETY: FFI.
    let ret = unsafe { isal_sys::xor_check(src.len() as i32, len as i32, array.as_mut_ptr() as _) };
    if ret != 0 {
        panic!("isa-l: xor_check failed with error code {}", ret);
    }
}

/// Update a slice with the XOR of itself and another slice.
#[inline]
pub fn xor_update(dst: &mut [u8], src: &[u8]) {
    assert!(
        dst.len() >= src.len(),
        "xor_update: target slice length {} smaller than source slice length {}",
        dst.len(),
        src.len()
    );

    // Check source pointer.
    let src_holder = if src.as_ptr() as usize % ALIGNMENT != 0 {
        let mut buf = vec![0; src.len() + ALIGNMENT];
        let aligned_ptr = make_aligned_mut_to(buf.as_mut_ptr(), ALIGNMENT);
        unsafe { ptr::copy_nonoverlapping(src.as_ptr(), aligned_ptr, src.len()) };
        Some(buf)
    } else {
        None
    };
    let src_ptr = match src_holder {
        Some(ref buf) => make_aligned_to(buf.as_ptr(), ALIGNMENT),
        None => src.as_ptr(),
    };

    // Check destination pointer.
    let mut dst_holder = if dst.as_ptr() as usize % ALIGNMENT != 0 {
        Some(vec![0; dst.len() + ALIGNMENT])
    } else {
        None
    };
    let dst_ptr = match dst_holder {
        Some(ref mut buf) => make_aligned_mut_to(buf.as_mut_ptr(), ALIGNMENT),
        None => dst.as_mut_ptr(),
    };

    let mut array = vec![src_ptr, dst_ptr, dst_ptr];
    let ret = unsafe { isal_sys::xor_gen(3, src.len() as i32, array.as_mut_ptr() as _) };
    if ret != 0 {
        panic!("isa-l: xor_update failed with error code {}", ret);
    }

    if dst_holder.is_some() {
        unsafe {
            ptr::copy_nonoverlapping(dst_ptr, dst.as_mut_ptr(), src.len());
        }
    }
}

/// Perform EC encode.
#[inline]
pub fn ec_encode(gftbls: &[u8], data: &[*const u8], parity: &[*mut u8], len: usize) {
    debug_assert_eq!(
        gftbls.len(),
        32 * data.len() * parity.len(),
        "ec_encode: invalid gftbls length"
    );

    // SAFETY: FFI.
    unsafe {
        isal_sys::ec_encode_data_avx(
            len as _,
            data.len() as _,
            parity.len() as _,
            gftbls.as_ptr() as _,
            data.as_ptr() as _,
            parity.as_ptr() as _,
        )
    };
}

/// Perform one-time EC table initialization and encode.
/// This is useful when decoding data.
#[inline]
pub fn ec_onetime_encode(matrix: &[u8], data: &[*const u8], parity: &[*mut u8], len: usize) {
    debug_assert_eq!(
        matrix.len(),
        data.len() * parity.len(),
        "ec_encode: invalid gftbls length"
    );

    let mut gftbls = vec![0; 32 * data.len() * parity.len()];

    // SAFETY: FFI.
    unsafe {
        let k = data.len();
        let p = parity.len();

        isal_sys::ec_init_tables(k as _, p as _, matrix.as_ptr() as _, gftbls.as_mut_ptr());

        isal_sys::ec_encode_data_avx(
            len as _,
            k as _,
            p as _,
            gftbls.as_mut_ptr(),
            data.as_ptr() as _,
            parity.as_ptr() as _,
        );
    }
}

/// Perform EC update.
#[inline]
pub fn ec_update(
    gftbls: &[u8],
    data_num: usize,
    parity: &[*mut u8],
    len: usize,
    data_idx: usize,
    data: &[u8],
) {
    debug_assert_eq!(
        gftbls.len(),
        32 * data_num * parity.len(),
        "ec_encode: invalid gftbls length"
    );
    debug_assert!(
        data_idx < data_num,
        "ec_update: invalid data_idx {}",
        data_idx
    );

    // SAFETY: FFI.
    unsafe {
        isal_sys::ec_encode_data_update_avx(
            len as _,
            data_num as _,
            parity.len() as _,
            data_idx as _,
            gftbls.as_ptr() as _,
            data.as_ptr() as _,
            parity.as_ptr() as _,
        )
    };
}

/// Construct a decode matrix for lost data chunks with the given encode matrix and indices.
#[inline]
pub fn ec_make_decode_matrix(
    k: usize,
    p: usize,
    encode_matrix: &[u8],
    indices: &[usize],
) -> Vec<u8> {
    let m = k + p;
    debug_assert_eq!(
        encode_matrix.len(),
        m * k,
        "ec_make_decode_matrix: invalid encode matrix length"
    );
    debug_assert_eq!(
        k,
        indices.len(),
        "ec_make_decode_matrix: invalid indices array length"
    );
    debug_assert!(
        indices.iter().all(|&idx| idx < m),
        "ec_make_decode_matrix: invalid indices"
    );
    debug_assert!(
        (0..(k - 1)).all(|i| indices[i] < indices[i + 1]),
        "ec_make_decode_matrix: indice array unsorted"
    );

    // Construct b (matrix that encoded remaining frags) by removing erased rows
    let mut b = vec![0; k * k];
    for (i, &idx) in indices.iter().enumerate() {
        // SAFETY: inbound.
        unsafe {
            ptr::copy_nonoverlapping(
                encode_matrix.as_ptr().add(idx * k),
                b.as_mut_ptr().add(i * k),
                k,
            )
        };
    }

    // SAFETY: FFI.
    let mut inv = vec![0; k * k];
    let ret = unsafe { isal_sys::gf_invert_matrix(b.as_mut_ptr(), inv.as_mut_ptr(), k as _) };
    if ret != 0 {
        panic!("isa-l: gf_invert_matrix failed");
    }
    inv
}

/// Perform PQ encode.
#[inline]
#[allow(unused_assignments)]
pub fn pq_encode(data: &[&[u8]], parity: &mut [&mut [u8]]) {
    assert!(parity.len() <= 2);
    if parity.is_empty() {
        return;
    }

    let len = data.len();
    let roundup_len = len.next_multiple_of(ALIGNMENT);

    let mut data_holders = Vec::with_capacity(data.len());
    let mut array = data
        .iter()
        .map(|slice| {
            if slice.as_ptr() as usize % ALIGNMENT == 0 {
                slice.as_ptr() as *mut u8 as *mut std::ffi::c_void
            } else {
                data_holders.push(vec![0; roundup_len + ALIGNMENT]);
                let buf = data_holders.last_mut().unwrap();
                let data_ptr = make_aligned_mut_to(buf.as_mut_ptr(), ALIGNMENT);
                unsafe { ptr::copy_nonoverlapping(slice.as_ptr(), data_ptr, slice.len()) };
                data_ptr as *mut std::ffi::c_void
            }
        })
        .collect::<Vec<_>>();
    array.push(parity[0].as_mut_ptr() as *mut std::ffi::c_void);

    let mut p_holder = vec![0u8; roundup_len + ALIGNMENT];
    let mut q_holder = vec![0u8; roundup_len + ALIGNMENT];
    if parity[0].as_ptr() as usize % ALIGNMENT == 0 {
        array.push(parity[0].as_mut_ptr() as *mut std::ffi::c_void);
    } else {
        let p_ptr = make_aligned_mut_to(p_holder.as_mut_ptr(), ALIGNMENT);
        unsafe { ptr::copy_nonoverlapping(parity[0].as_ptr(), p_ptr, parity[0].len()) };
        array.push(p_ptr as *mut std::ffi::c_void);
    }

    if parity.len() == 2 {
        if parity[1].as_ptr() as usize % ALIGNMENT == 0 {
            array.push(parity[1].as_mut_ptr() as *mut std::ffi::c_void);
        } else {
            let q_ptr = make_aligned_mut_to(q_holder.as_mut_ptr(), ALIGNMENT);
            unsafe { ptr::copy_nonoverlapping(parity[1].as_ptr(), q_ptr, parity[1].len()) };
            array.push(q_ptr as *mut std::ffi::c_void);
        }
    } else {
        array.push(make_aligned_mut_to(q_holder.as_mut_ptr(), ALIGNMENT) as *mut std::ffi::c_void);
    }

    // SAFETY: FFI.
    let ret = unsafe {
        isal_sys::pq_gen(
            (data.len() + 2) as i32,
            roundup_len as i32,
            array.as_mut_ptr(),
        )
    };
    if ret != 0 {
        panic!("isa-l: pq_gen failed");
    }
}

/// Perform PQ update.
#[inline]
pub fn pq_update(data: &[u8], index: usize, k: usize, parity: &mut [&mut [u8]]) {
    assert!(parity.len() == 2);

    let len = data.len();
    let roundup_len = len.next_multiple_of(ALIGNMENT);

    let mut data_holder = vec![0u8; roundup_len + ALIGNMENT];
    let data_ptr = make_aligned_mut_to(data_holder.as_mut_ptr(), ALIGNMENT);
    unsafe { ptr::copy_nonoverlapping(data.as_ptr(), data_ptr, len) };

    let mut p_holder = vec![0u8; roundup_len + ALIGNMENT];
    let p_ptr = if parity[0].as_ptr() as usize % ALIGNMENT == 0 {
        parity[0].as_mut_ptr()
    } else {
        make_aligned_mut_to(p_holder.as_mut_ptr(), ALIGNMENT)
    };
    let mut q_holder = vec![0u8; roundup_len + ALIGNMENT];
    let q_ptr = if parity[1].as_ptr() as usize % ALIGNMENT == 0 {
        parity[1].as_mut_ptr()
    } else {
        make_aligned_mut_to(q_holder.as_mut_ptr(), ALIGNMENT)
    };

    let zeroes_ptr = make_aligned_to(ZEROES.as_ptr(), ALIGNMENT);
    let mut array = vec![zeroes_ptr as *mut std::ffi::c_void; k + 2];
    array[index] = data_ptr as *mut u8 as _;
    array[k] = p_ptr as _;
    array[k + 1] = q_ptr as _;

    // SAFETY: FFI.
    let ret = unsafe { isal_sys::pq_gen((k + 2) as i32, roundup_len as i32, array.as_mut_ptr()) };
    if ret != 0 {
        panic!("isa-l: pq_gen failed");
    }

    if p_ptr != parity[0].as_mut_ptr() {
        parity[0].copy_from_slice(&p_holder[..len]);
    }
    if q_ptr != parity[1].as_mut_ptr() {
        parity[1].copy_from_slice(&q_holder[..len]);
    }
}
