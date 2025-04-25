//! Workload runners.

pub mod closed_loop;
pub mod open_loop;
pub mod prepare;

use std::mem;
use std::str::FromStr;
use std::sync::{Arc, Barrier};

use futures::executor::block_on;
use nos::ctrl::{Cluster, NIC_NAME, NIC_PHYPORT};
use nos::Key;
use rrppcc::{type_alias::*, *};

use crate::perf::*;
use crate::utils::{self, Prepare};
use crate::workload::Workload;

/// Workload execution policy.
#[derive(Debug, Clone, Copy)]
pub enum RunnerPolicy {
    /// All threads run in closed-loop, with coroutines multiplexing every thread.
    /// Should be used for maximum throughput.
    ClosedLoop,

    /// All threads run in open-loop.
    ///
    /// Parameter: lambda (unit: nanoseconds^{-1}).
    OpenLoop(f64),

    /// All but one thread run in open-loop.
    /// The remaining thread is closed-loop and measures common-case latency.
    ///
    /// Parameter: lambda (unit: nanoseconds^{-1}).
    OpenLoopMeasured(f64),

    /// Recover the target node.
    ///
    /// Parameter: target node ID.
    Recovery(usize),

    /// All but one thread run in open-loop.
    /// The remaining thread is closed-loop and measures degraded read latency.
    ///
    /// Parameter: target node ID + lambda (unit: nanoseconds^{-1}).
    OpenLoopDegraded(usize, f64),
}

/// Valid policy string values:
/// - `closed`: Closed-loop, with single/multiple coroutines.
/// - `open:<lambda>`: Open-loop with Poisson(<lambda>) request rate.
/// - `open-measured:<lambda>`: Open-loop with Poisson(<lambda>) request rate, with one thread measuring latency.
/// - `recover:<target>`: Recover the target node.
impl FromStr for RunnerPolicy {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut parts = s.split(':');
        let policy = parts.next().unwrap();

        Ok(match policy {
            "open" => {
                let arg = parts
                    .next()
                    .ok_or_else(|| anyhow::anyhow!("missing request rate"))?;
                let theta = arg.parse::<f64>()?;
                RunnerPolicy::OpenLoop(1.0 / theta)
            }
            "open-measured" => {
                let arg = parts
                    .next()
                    .ok_or_else(|| anyhow::anyhow!("missing request rate"))?;
                let theta = arg.parse::<f64>()?;
                RunnerPolicy::OpenLoopMeasured(1.0 / theta)
            }
            "recover" => {
                let arg = parts
                    .next()
                    .ok_or_else(|| anyhow::anyhow!("missing recovery target"))?;
                RunnerPolicy::Recovery(arg.parse()?)
            }
            "open-degraded" => {
                let mut args = parts
                    .next()
                    .ok_or_else(|| anyhow::anyhow!("missing ID / request rate"))?
                    .split(',');
                let target = args
                    .next()
                    .ok_or_else(|| anyhow::anyhow!("missing ID"))?
                    .parse()?;
                let theta = args
                    .next()
                    .ok_or_else(|| anyhow::anyhow!("missing ID"))?
                    .parse::<f64>()?;
                RunnerPolicy::OpenLoopDegraded(target, 1.0 / theta)
            }
            _ => RunnerPolicy::ClosedLoop,
        })
    }
}

/// The workload runner.
/// This function should be run as a separate thread, possibly with a little logic before/after it.
pub fn run_workload_thread<K>(
    (tid, nthreads): (usize, usize),
    cluster: Cluster,
    (workload, prepare, policy): (Workload, Option<Prepare>, RunnerPolicy),
    nexus: Arc<Nexus>,
    perfmon: Arc<Perfmon>,
    barrier: Arc<Barrier>,
) where
    K: Key,
{
    // Prepare RPC.
    let rpc = Rpc::new(&nexus, tid as RpcId, NIC_NAME, NIC_PHYPORT);
    let sessions = {
        let mut sessions = Vec::with_capacity(cluster.size());
        for svr in cluster.peers() {
            let sess = rpc.create_session(svr.uri(), (tid + 1) as RpcId);
            assert!(block_on(sess.connect()), "failed to connect to {:?}", svr);
            sessions.push(sess);
        }
        sessions
    };
    log::trace!("tid {}: connected to all servers", tid);

    // Prepare initial records if needed.
    let keyspace = workload.count;
    if let Some(ref prep) = prepare {
        let my_share = utils::get_share(0..keyspace, prep.i, prep.n);
        let share = utils::get_share(my_share, tid, nthreads);
        block_on(prepare::prepare::<K>(
            &cluster,
            &workload,
            share,
            &rpc,
            &sessions,
            tid == 0,
        ));
    }
    if barrier.wait().is_leader() && prepare.is_some() {
        let prep = prepare.as_ref().unwrap();
        log::info!("all records prepared ({}/{})", prep.i, prep.n);
    }

    // Inject failures if needed.
    if tid == 0 {
        if let Some(ref faults) = workload.faults {
            block_on(prepare::inject_failure(&cluster, faults, &rpc, &sessions));
        }
    }
    barrier.wait();

    // Convert sessions into IDs to prevent unsafely transmuting them to `'static`.
    let sessions = sessions
        .into_iter()
        .map(|sess| sess.id())
        .collect::<Vec<_>>();

    // Run the workload.
    {
        // Number of coroutines used in closed-loop multi-coroutine policy.
        const NUM_COROS: usize = 32;

        // SAFETY: the spawned futures are awaited before `rpc` and `sessions` are destroyed,
        // so casting them to `'static` should be safe.
        let rpc: &'static Rpc = unsafe { mem::transmute(rpc.as_ref().get_ref()) };
        match policy {
            RunnerPolicy::ClosedLoop => {
                closed_loop::run_closed_loop::<K>(
                    (tid, nthreads),
                    cluster,
                    workload,
                    NUM_COROS,
                    None,
                    rpc,
                    sessions,
                    perfmon,
                );
            }
            RunnerPolicy::OpenLoop(lambda) => {
                open_loop::run_open_loop::<K>(
                    (tid, nthreads),
                    cluster,
                    workload,
                    rpc,
                    &sessions,
                    lambda,
                );
            }
            RunnerPolicy::OpenLoopMeasured(lambda) => {
                if tid == 0 {
                    closed_loop::run_closed_loop::<K>(
                        (tid, nthreads),
                        cluster,
                        workload,
                        1,
                        None,
                        rpc,
                        sessions,
                        perfmon,
                    );
                } else {
                    open_loop::run_open_loop::<K>(
                        (tid, nthreads),
                        cluster,
                        workload,
                        rpc,
                        &sessions,
                        lambda,
                    );
                }
            }
            RunnerPolicy::OpenLoopDegraded(target, lambda) => {
                if tid == 0 {
                    closed_loop::run_closed_loop::<K>(
                        (tid, nthreads),
                        cluster,
                        workload,
                        1,
                        Some((target, 0..keyspace)),
                        rpc,
                        sessions,
                        perfmon,
                    );
                } else {
                    open_loop::run_open_loop::<K>(
                        (tid, nthreads),
                        cluster,
                        workload,
                        rpc,
                        &sessions,
                        lambda,
                    );
                }
            }
            RunnerPolicy::Recovery(target) => {
                closed_loop::run_closed_loop::<K>(
                    (tid, nthreads),
                    cluster,
                    workload,
                    NUM_COROS,
                    Some((target, utils::get_share(0..keyspace, tid, nthreads))),
                    rpc,
                    sessions,
                    perfmon,
                );
            }
        };
    }

    // Wait for all threads to finish, consumes the 2nd barrier
    if barrier.wait().is_leader() {
        log::info!("workload finished");
    }
}
