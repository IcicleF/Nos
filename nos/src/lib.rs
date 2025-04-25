pub mod ctrl;
pub mod ec;
mod key;
pub mod objstore;
pub mod rpc_types;
pub mod utils;

use std::array;
use std::sync::{atomic::*, Arc, Barrier};
use std::thread;

use anyhow::{Context as _, Result};
use clap::Parser;
use futures::executor::block_on;
use local_ip_address::local_ip;
use rrppcc::{type_alias::*, *};
use uuid::Uuid;

use ctrl::*;
use ec::*;
use objstore::*;
use rpc_types::*;

pub use key::Key;

/// Nos server.
///
/// Note that these arguments are only those used by the executable.
/// Scripts that invoke the executable use many other arguments.
#[derive(Parser, Debug)]
#[command(author, about, long_about = None)]
pub struct Args {
    /// Path to the configuration file.
    #[clap(short, long, default_value = "./config/lab.toml")]
    pub config_file: String,

    /// UUID to identify a run of the cluster.
    #[clap(short, long, default_value = "01234567-89ab-cdef-0123-456789abcdef")]
    pub uuid: Uuid,
}

/// Run the Nos server.
///
/// This function should be called from `main` once and only once.
///
/// # Example
///
/// ```no_run
/// # use {nos::Args, clap::Parser};
/// fn main() -> anyhow::Result<()> {
///     let args = Args::parse();
///     nos::run(args)
/// }
/// ```
pub fn run(args: Args) -> Result<()> {
    ///////////////////////////////// Config /////////////////////////////////
    let cluster = Cluster::load_toml_file(&args.config_file, Role::Server(args.uuid))
        .with_context(|| "failed to load cluster config")?;
    log::info!("loaded cluster config, this is node {}", cluster.rank());

    let ec_config = EcConfig::load_toml_file(&args.config_file);
    let nthreads = NThreads::load_toml_file(&args.config_file)?;
    if cluster.rank() == 0 {
        log::info!("EC config: {:?}", ec_config);
        log::info!("NThreads: {:#?}", nthreads);
    }

    let core_ids = core_affinity::get_core_ids()
        .unwrap()
        .into_iter()
        .filter(|core_id| core_id.id >= cluster.me().cpu())
        .collect::<Vec<_>>();

    // Enable local allocation. (useless on CloudLab?)
    numa_sys::set_localalloc();
    let mut core_id_iter = core_ids.into_iter();

    //////////////////////////////// ObjStore ////////////////////////////////
    // Initiate object store.
    let objstore = <ObjStore<u64>>::new(&cluster, &ec_config);
    log::debug!("created ObjStore");

    // Initialize RPC.
    let nexus = Nexus::new((
        local_ip().expect("failed to get local IPv4 address"),
        cluster.me().port(),
    ));
    log::info!("Nexus working on {:?}", nexus.uri());

    // Spawn worker threads for the ObjStore.
    let mut listener_handles = Vec::with_capacity(nthreads.rpc + nthreads.ec);

    for _ in 0..nthreads.ec {
        let core_id = core_id_iter.next().unwrap();
        let objstore = objstore.clone();
        listener_handles.push(thread::spawn(move || {
            // Bind thread to core
            core_affinity::set_for_current(core_id);
            objstore.encoder()
        }));
    }

    // Two barriers used to sync RPC threads twice.
    let barriers: [_; 2] = array::from_fn(|_| Arc::new(Barrier::new(nthreads.rpc)));
    let rpc_created = Arc::new(AtomicUsize::new(nthreads.rpc));

    // Rpc ID starts from 1.
    for id in 1..=nthreads.rpc {
        let nexus = nexus.clone();
        let objstore = objstore.clone();
        let core_id = core_id_iter.next().unwrap();
        let cluster = cluster.clone();

        let barriers = barriers.clone();
        let rpc_created = rpc_created.clone();

        listener_handles.push(thread::spawn(move || {
            // Bind core
            core_affinity::set_for_current(core_id);

            // Prepare RPC and wait for all RPC threads in the cluster to be ready
            let rpc = {
                let mut rpc = Rpc::new(&nexus, id as RpcId, NIC_NAME, NIC_PHYPORT);
                for rpc_type in ALL_RPC_TYPES {
                    let objstore = objstore.clone();
                    match *rpc_type {
                        ty if (BEGIN_DATA_RPC..=END_DATA_RPC).contains(&ty) => {
                            rpc.set_handler(ty, move |req| {
                                let objstore = objstore.clone();
                                async move { objstore.rpc_handler(req).await }
                            });
                        }
                        ty if (BEGIN_RECOVERY_RPC..=END_RECOVERY_RPC).contains(&ty) => {
                            rpc.set_handler(ty, move |req| {
                                let objstore = objstore.clone();
                                async move { objstore.recovery_rpc_handler(req).await }
                            });
                        }
                        ty if (BEGIN_CONTROL_RPC..=END_CONTROL_RPC).contains(&ty) => {
                            rpc.set_handler(ty, move |req| {
                                objstore.ctrl_rpc_handler(&req);
                                async move {
                                    let mut resp = req.pre_resp_buf();
                                    resp.set_len(0);
                                    resp
                                }
                            });
                        }
                        _ => unreachable!(),
                    };
                }
                rpc
            };

            {
                barriers[0].wait();
                if id == 1 {
                    DistBarrier::wait(&cluster);
                }
                barriers[1].wait();
            }

            // Connect to the RPCs.
            // SAFETY: call only once, no RPC servers have been ran yet.
            block_on(unsafe { objstore.prepare_rpc(&rpc, id as RpcId) });

            // Do not barrier here, since this stops RPC event loop and results in deadlock.
            // Instead, continue to run the RPC server and let main thread barrier.
            rpc_created.fetch_sub(1, Ordering::SeqCst);

            // Run the RPC server
            const LOOP_BATCH: usize = 5000;
            while !objstore.is_halted() {
                for _ in 0..LOOP_BATCH {
                    rpc.progress();
                }
            }
            Ok(())
        }));
    }
    log::debug!("spawned all handler threads");

    // Wait for all RPC threads to be ready.
    while rpc_created.load(Ordering::SeqCst) != 0 {
        std::hint::spin_loop();
    }
    DistBarrier::wait(&cluster);
    if cluster.rank() == 0 {
        log::info!("===== cluster is ready! =====");
    }

    // Join handlers.
    for handle in listener_handles {
        handle.join().unwrap()?;
    }
    Ok(())
}
