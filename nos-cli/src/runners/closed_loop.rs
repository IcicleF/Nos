//! Closed-loop worker.

use std::ops::Range;
use std::sync::Arc;
use std::time::Duration;

use quanta::Instant;
use rrppcc::{type_alias::*, *};
use tokio::task;

#[allow(unused)]
use rand::prelude::*;

use nos::ctrl::Cluster;
use nos::Key;

use crate::perf::*;
use crate::workload::{KvRequest, Workload, WorkloadGenerator};
use crate::{distributed, utils};

/// Client that runs normal workloads.
async fn run_normal<K>(
    tid: usize,
    (coro_id, num_coros): (usize, usize),
    cluster: Cluster,
    workload: Workload,
    rpc: &Rpc,
    sessions: Vec<SessId>,
    perfmon: Arc<Perfmon>,
) where
    K: Key,
{
    // Get the sessions.
    let sessions = sessions
        .into_iter()
        .map(|sess_id| rpc.get_session(sess_id).unwrap())
        .collect::<Vec<_>>();

    // Prepare workload.
    let wg = WorkloadGenerator::new(&workload);
    let wl_duration = Duration::from_secs(workload.duration);

    let mut req_buf = rpc.alloc_msgbuf(distributed::buf_len::<K>());
    let mut resp_buf = rpc.alloc_msgbuf(distributed::buf_len::<K>());

    // Run workload.
    let mut trace_iter = wg.trace_iter(coro_id, num_coros);
    let start_time = Instant::now();
    'timer_loop: while start_time.elapsed() <= wl_duration {
        const WORKLOAD_BATCH: usize = 64;
        for _ in 0..WORKLOAD_BATCH {
            let req = if let Some(ref mut it) = trace_iter {
                let Some(record) = it.next() else {
                    break 'timer_loop;
                };
                if record.is_set {
                    KvRequest::Write(record.key as usize, vec![0; record.len].into_boxed_slice())
                } else {
                    KvRequest::Read(record.key as usize)
                }
            } else {
                wg.generate()
            };

            match req {
                KvRequest::Read(idx) => {
                    let key = K::from(idx as u64);
                    let primary = (key % cluster.size() as u64) as usize;

                    let op = Op::new(&perfmon, tid, OpType::Get);
                    let resp = if cluster[primary].is_alive() {
                        distributed::read(&sessions[primary], &mut req_buf, &mut resp_buf, key)
                            .await
                    } else {
                        let mut failover = (primary + 1) % cluster.size();
                        while failover != primary && !cluster[failover].is_alive() {
                            failover = (failover + 1) % cluster.size();
                        }
                        debug_assert!(cluster[failover].is_alive(), "no alive server");

                        // Read request sent to a non-primary automatically becomes a degraded read
                        distributed::read(&sessions[failover], &mut req_buf, &mut resp_buf, key)
                            .await
                    };
                    if cfg!(debug_assertions) && resp.is_none() {
                        log::warn!("key {} not found", key);
                    }
                    drop(op);
                }
                KvRequest::Write(idx, value) => {
                    let key = K::from(idx as u64);
                    let primary = (key % cluster.size() as u64) as usize;

                    let op = Op::new(&perfmon, tid, OpType::Put);
                    debug_assert!(cluster[primary].is_alive(), "degraded write not supported");
                    distributed::write(
                        &sessions[primary],
                        &mut req_buf,
                        &mut resp_buf,
                        key,
                        &value,
                    )
                    .await;
                    drop(op);
                }
            }
        }
    }
}

/// Client coroutine that runs recovery workloads.
async fn run_recovery_coro<K>(
    tid: usize,
    (share, target, duration): (Range<usize>, usize, Duration),
    cluster: Cluster,
    (rpc, sessions): (&'static Rpc, Vec<SessId>),
    perfmon: Arc<Perfmon>,
) where
    K: Key,
{
    // Get the sessions.
    let sessions = sessions
        .into_iter()
        .map(|sess_id| rpc.get_session(sess_id).unwrap())
        .collect::<Vec<_>>();

    // Run workload.
    let mut req_buf = rpc.alloc_msgbuf(distributed::buf_len::<K>());
    let mut resp_buf = rpc.alloc_msgbuf(distributed::buf_len::<K>());

    let start_time = Instant::now();
    while start_time.elapsed() <= duration {
        let share = share.clone();
        for key in share.filter(|k| (*k) % cluster.size() == target) {
            // Move to next alive server
            let mut failover = key % cluster.size();
            while !cluster[failover].is_alive() {
                failover = (failover + 1) % cluster.size();
            }

            let key = K::from(key as u64);

            // Read from the failover server
            let op = Op::new(&perfmon, tid, OpType::Get);
            distributed::read(&sessions[failover], &mut req_buf, &mut resp_buf, key).await;
            drop(op);
        }
    }
}

/// Per-thread closed-loop client.
/// Dispatch requests to the servers without stopping, but won't send another request
/// when an existing one is pending.
#[allow(clippy::too_many_arguments)]
#[tokio::main(flavor = "current_thread")]
pub async fn run_closed_loop<K>(
    (tid, nthreads): (usize, usize),
    cluster: Cluster,
    workload: Workload,
    num_coros: usize,
    recover: Option<(usize, Range<usize>)>,
    rpc: &'static Rpc,
    sessions: Vec<SessId>,
    perfmon: Arc<Perfmon>,
) where
    K: Key,
{
    // Run workload in a `LocalSet`.
    let ls = task::LocalSet::new();
    for i in 0..num_coros {
        let cluster = cluster.clone();
        let perfmon = perfmon.clone();

        if let Some((target, share)) = recover.clone() {
            // Recovery.
            let share = utils::get_share(share.clone(), i, num_coros);
            ls.spawn_local(run_recovery_coro::<K>(
                tid,
                (share, target, Duration::from_secs(workload.duration)),
                cluster,
                (rpc, sessions.clone()),
                perfmon,
            ));
        } else {
            // Normal.
            let workload = workload.clone();
            ls.spawn_local(run_normal::<K>(
                tid,
                (tid * num_coros + i, nthreads * num_coros),
                cluster,
                workload,
                rpc,
                sessions.clone(),
                perfmon,
            ));
        }
    }
    ls.await;
}
