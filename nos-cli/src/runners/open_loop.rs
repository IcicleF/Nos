//! Open-loop worker.

use std::cell::RefCell;
use std::collections::VecDeque;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;
use std::{mem, ptr};

use futures::FutureExt;
use quanta::Instant;
use rand::prelude::*;
use rand_distr::Exp;
use rrppcc::{type_alias::*, *};

use nos::ctrl::Cluster;
use nos::objstore::{ObjMeta, MAX_VALUE_SIZE};
use nos::{rpc_types, Key};

use crate::distributed;
use crate::workload::{KvRequest, Workload, WorkloadGenerator};

struct BufPair {
    req_buf: MsgBuf,
    resp_buf: MsgBuf,
}

/// An awaitable object that handles request execution and resource freeing.
struct OpenLoopRequest<'a> {
    req: Request<'a>,

    ll: &'a RefCell<VecDeque<Box<BufPair>>>,
    bufs_ptr: *mut BufPair,
}

impl<'a> OpenLoopRequest<'a> {
    /// Create a new request object.
    pub fn new<'r, K>(
        session: &'a Session<'r>,
        request: KvRequest,
        bufs: Box<BufPair>,
        ll: &'a RefCell<VecDeque<Box<BufPair>>>,
    ) -> Self
    where
        K: Key,
    {
        let bufs = Box::leak(bufs);
        let bufs_ptr = bufs as *mut BufPair;

        let req = match request {
            KvRequest::Read(idx) => {
                let key = K::from(idx as u64);
                unsafe {
                    let hdr = ObjMeta::new(key, 0, 0);
                    let hdr_pos = bufs.req_buf.as_ptr() as *mut ObjMeta<K>;
                    ptr::write(hdr_pos, hdr);
                }
                bufs.req_buf.set_len(<ObjMeta<K>>::len());
                bufs.resp_buf.set_len(distributed::buf_len::<K>());
                session.request(rpc_types::RPC_DATA_READ, &bufs.req_buf, &mut bufs.resp_buf)
            }
            KvRequest::Write(idx, value) => {
                let key = K::from(idx as u64);
                let value_len = value.len().min(MAX_VALUE_SIZE);
                unsafe {
                    let hdr = ObjMeta::new(key, value_len, 0);
                    let hdr_pos = bufs.req_buf.as_ptr() as *mut ObjMeta<K>;
                    ptr::write(hdr_pos, hdr);
                    let value_pos = bufs.req_buf.as_mut_ptr().add(<ObjMeta<K>>::len());
                    ptr::copy_nonoverlapping(value.as_ptr(), value_pos, value_len);
                }
                bufs.req_buf.set_len(<ObjMeta<K>>::len() + value_len);
                bufs.resp_buf.set_len(mem::size_of::<u64>());
                session.request(rpc_types::RPC_DATA_WRITE, &bufs.req_buf, &mut bufs.resp_buf)
            }
        };
        Self { req, ll, bufs_ptr }
    }
}

impl<'a> Future for OpenLoopRequest<'a> {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        let res = self.req.poll_unpin(cx);
        if res.is_ready() {
            let bufs = unsafe { Box::from_raw(self.bufs_ptr as *mut BufPair) };
            self.ll.borrow_mut().push_back(bufs);
        }
        res
    }
}

/// Per-thread open-loop client.
/// Dispatch requests to the servers at a fixed rate (Poisson process).
///
/// Prior to calling this function, the dataset must be already prepared.
#[tokio::main(flavor = "current_thread")]
pub async fn run_open_loop<'a, K>(
    (tid, nthreads): (usize, usize),
    cluster: Cluster,
    workload: Workload,
    rpc: &'a Rpc,
    sessions: &'a [SessId],
    lambda: f64,
) where
    K: Key,
{
    // Get the sessions.
    let sessions = sessions
        .iter()
        .map(|&sess_id| rpc.get_session(sess_id).unwrap())
        .collect::<Vec<_>>();

    // Poisson process: interval between events ~ Exp(lambda).
    let mut rng = rand::thread_rng();
    let exp = Exp::new(lambda).unwrap();

    // Prepare workload.
    let wg = WorkloadGenerator::new(&workload);
    let wl_duration = Duration::from_secs(workload.duration);

    // Open-loop worker requires more sophisticated buffer & Future management.
    let buf_queue = RefCell::new(VecDeque::new());
    let mut requests: Vec<OpenLoopRequest<'_>> = Vec::with_capacity(8);

    // Run workload.
    let mut trace_iter = wg.trace_iter(tid, nthreads);
    let start_time = Instant::now();
    let mut next_req_time = start_time;
    loop {
        // Get the timestamp only once.
        let now = Instant::now();
        if now.duration_since(start_time) >= wl_duration {
            break;
        }

        // Poll existing requests.
        let mut cx = Context::from_waker(futures::task::noop_waker_ref());
        requests.retain_mut(|req| req.poll_unpin(&mut cx).is_pending());

        // Generate a new request if time has come.
        if now >= next_req_time {
            let (req, session) = if let Some(ref mut it) = trace_iter {
                let Some(record) = it.next() else { break };
                let key = K::from(record.key);
                let req = match record.is_set {
                    true => KvRequest::Write(
                        record.key as usize,
                        vec![0; record.len].into_boxed_slice(),
                    ),
                    false => KvRequest::Read(record.key as usize),
                };
                let session = &sessions[(key % cluster.size() as u64) as usize];
                (req, session)
            } else {
                // Avoid generating a key that maps to a dead server.
                let (req, key) = {
                    let mut req = wg.generate();
                    let mut key = K::from(req.key_idx() as u64);
                    let mut primary = (key % cluster.size() as u64) as usize;
                    while !cluster[primary].is_alive() {
                        req = wg.generate();
                        key = K::from(req.key_idx() as u64);
                        primary = (key % cluster.size() as u64) as usize;
                    }
                    (req, key)
                };
                let session = &sessions[(key % cluster.size() as u64) as usize];
                (req, session)
            };

            let node = buf_queue.borrow_mut().pop_front().unwrap_or_else(|| {
                let len = distributed::buf_len::<K>();
                Box::new(BufPair {
                    req_buf: rpc.alloc_msgbuf(len),
                    resp_buf: rpc.alloc_msgbuf(len),
                })
            });
            let request = OpenLoopRequest::new::<K>(session, req, node, &buf_queue);
            requests.push(request);

            let nanos = exp.sample(&mut rng) as u64;
            next_req_time += Duration::from_nanos(nanos);
        }
    }
}
