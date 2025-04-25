use std::{mem, ptr, slice};

use nos::{
    objstore::{ObjMeta, MAX_VALUE_SIZE},
    rpc_types, Key,
};
use rrppcc::*;

/// Get a safe MsgBuf length to allocate.
pub const fn buf_len<K>() -> usize
where
    K: Key,
{
    <ObjMeta<K>>::len() + MAX_VALUE_SIZE
}

/// Perform distributed write with RPC.
///
/// # Contract
///
/// - The target session must point to the primary node of the key. If not, bad
///   things will happen, but the system should not crash.
#[inline]
pub async fn write<'a, K>(
    sess: &'a Session<'a>,
    req_buf: &'a mut MsgBuf,
    resp_buf: &'a mut MsgBuf,
    key: K,
    mut value: &[u8],
) where
    K: Key,
{
    if value.len() > MAX_VALUE_SIZE {
        // panic!(
        //     "value to be written too large: {} > {}",
        //     value.len(),
        //     MAX_VALUE_SIZE
        // );
        value = &value[..MAX_VALUE_SIZE];
    }
    let value = value;

    let hdr = ObjMeta::new(key, value.len(), 0);
    unsafe {
        let hdr_pos = req_buf.as_mut_ptr() as *mut ObjMeta<K>;
        ptr::write_volatile(hdr_pos, hdr);

        let value_pos = req_buf.as_mut_ptr().add(<ObjMeta<K>>::len());
        ptr::copy_nonoverlapping(value.as_ptr(), value_pos, value.len());
    }
    req_buf.set_len(<ObjMeta<K>>::len() + value.len());
    resp_buf.set_len(mem::size_of::<u64>());

    sess.request(rpc_types::RPC_DATA_WRITE, req_buf, resp_buf)
        .await;

    let retval = unsafe { *(resp_buf.as_ptr() as *mut u64) };
    debug_assert_eq!(0, retval, "write failed with retval: {}", retval);
}

/// Perform distributed write with RPC.
#[inline]
pub async fn read<'a, K>(
    sess: &'a Session<'a>,
    req_buf: &'a mut MsgBuf,
    resp_buf: &'a mut MsgBuf,
    key: K,
) -> Option<&'a [u8]>
where
    K: Key,
{
    let hdr = ObjMeta::new(key, 0, 0);
    unsafe {
        let hdr_pos = req_buf.as_ptr() as *mut ObjMeta<K>;
        ptr::write_volatile(hdr_pos, hdr);
    }
    req_buf.set_len(<ObjMeta<K>>::len());
    resp_buf.set_len(buf_len::<K>());

    sess.request(rpc_types::RPC_DATA_READ, req_buf, resp_buf)
        .await;

    let retval = unsafe { *(resp_buf.as_mut_ptr() as *mut u64) };
    match retval {
        0 => None,
        len => unsafe {
            let value_pos = resp_buf.as_mut_ptr().add(mem::size_of::<u64>());
            let len = len as usize;
            debug_assert!(
                len <= MAX_VALUE_SIZE,
                "read value too large: {} > {}",
                len,
                MAX_VALUE_SIZE
            );
            Some(slice::from_raw_parts(value_pos, len))
        },
    }
}

/// Halt the RPC server at the target session.
///
/// # Contract
///
/// - Due to server-side designs, this RPC might not return. However it by now
///   worked well.
/// - Do not send any RPCs to the target node after this RPC.
#[allow(dead_code)]
pub async fn halt<'a>(sess: &'a Session<'a>, req_buf: &'a mut MsgBuf, resp_buf: &'a mut MsgBuf) {
    req_buf.set_len(8);
    resp_buf.set_len(8);

    sess.request(rpc_types::RPC_HALT, req_buf, resp_buf).await;
}

/// Make the target node think a specific node has died.
pub async fn inject_failure<'a>(
    sess: &'a Session<'a>,
    req_buf: &'a mut MsgBuf,
    resp_buf: &'a mut MsgBuf,
    node: usize,
) {
    // Request format:
    // - [0B, 8B): target node ID
    req_buf.set_len(mem::size_of::<u64>());
    resp_buf.set_len(8);
    unsafe { ptr::write(req_buf.as_mut_ptr() as *mut u64, node as u64) };

    sess.request(rpc_types::RPC_MAKE_FAILED, req_buf, resp_buf)
        .await;
}

/// Make the target node think a specific node has become alive.
#[allow(dead_code)]
pub async fn inject_recovery<'a>(
    sess: &'a Session<'a>,
    req_buf: &'a mut MsgBuf,
    resp_buf: &'a mut MsgBuf,
    node: usize,
) {
    // Request format:
    // - [0B, 8B): target node ID
    req_buf.set_len(mem::size_of::<u64>());
    resp_buf.set_len(8);
    unsafe { ptr::write(req_buf.as_mut_ptr() as *mut u64, node as u64) };

    sess.request(rpc_types::RPC_MAKE_ALIVE, req_buf, resp_buf)
        .await;
}
