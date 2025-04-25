mod distributed;
mod perf;
mod runners;
mod utils;
pub mod workload;

use std::sync::{atomic::*, Arc, Barrier};
use std::time::Duration;
use std::{cmp, thread};

use anyhow::{Context as _, Result};
use clap::Parser;
use core_affinity::CoreId;
use local_ip_address::local_ip;
use rrppcc::*;

#[allow(unused)]
use rand::prelude::*;
use utils::parse_prepare;

use self::runners::*;
use self::workload::Workload;
use nos::ctrl::{Cluster, NThreads, Role};

// For doctest.
#[doc(hidden)]
pub use perf::Perfmon;

/// General-purpose client.
///
/// Note that these arguments are only those used by the executable.
/// Scripts that invoke the executable use many other arguments.
#[derive(Parser, Debug)]
#[command(author, about, long_about = None)]
pub struct Args {
    /// Path to the configuration file.
    #[clap(short, long, default_value = "./config/default.toml")]
    pub config: String,

    /// Path to the workload spec file.
    #[clap(short, long)]
    pub workload: String,

    /// Path to the performance stat output file.
    ///
    /// If set, stdout will only print throughput, and the detailed performance
    /// data will be dumped to the specified file in CSV format at the end.
    /// The actual output file will have the hostname appended to the end of the path
    /// (but before the suffix) to prevent overwriting when running multiple clients.
    ///
    /// However, there is an exception when the path is `/dev/null`, in which case the
    /// dump data will be discarded. This can be used to suppress the stdout to only
    /// print the throughputs.
    #[clap(short, long, default_value = None)]
    pub dump: Option<String>,

    /// Session management port for client-side RPC.
    #[clap(short, long, default_value_t = 31850)]
    pub sm_port: u16,

    /// Cores to bind on.
    #[clap(short, long, default_value = "18-23,42-47")]
    pub bind_core: String,

    /// Execution policy string.
    ///
    /// Valid policy string values:
    /// - `closed`: Closed-loop, with single/multiple coroutines.
    /// - `open:<lambda>`: Open-loop with Poisson(`<lambda>`) request rate.
    /// - `open-measured:<lambda>`: Open-loop with Poisson(`<lambda>`) request rate, with one thread measuring latency.
    /// - `recover:<target>`: Recover the target node.
    #[clap(short, long, default_value = "closed")]
    pub policy: String,

    /// Determine whether to prepare the workload.
    ///
    /// If empty, do no preparation.
    /// Otherwise, the string must be of format `<a>/<n>`. The current thread will split the keyspace into n parts
    /// and prepare the a-th part.
    #[clap(long, default_value = "")]
    pub prepare: String,
}

/// Run the Nos client.
///
/// Nos adapts a C/S architecture, where the client is agnostic to the actual
/// implementation of the server. In other words, this client works for every
/// server implementation.
///
/// This function should be called from `main` once and only once.
///
/// # Example
///
/// ```no_run
/// # use {nos_cli::Args, clap::Parser};
/// fn main() -> anyhow::Result<()> {
///     let args = Args::parse();
///     nos_cli::run(args)
/// }
/// ```
pub fn run(args: Args) -> Result<()> {
    ///////////////////////////////// Config /////////////////////////////////
    // Load cluster
    let cluster = Cluster::load_toml_file(&args.config, Role::Client)
        .with_context(|| "failed to load cluster config")?;

    // Load nthreads
    let svr_rpcs = NThreads::load_toml_file(&args.config)?.rpc;

    // Load workload
    let workload =
        Workload::load_toml(&args.workload).with_context(|| "failed to load workload")?;

    let policy = args.policy.parse::<RunnerPolicy>()?;
    if let RunnerPolicy::Recovery(target) = policy {
        let faults = workload.faults.clone();
        assert!(
            faults.map_or(false, |faults| faults.contains(&target)),
            "invalid recovery target: {}",
            target
        );
        log::info!(
            "specified recovery target {}, workload details will be ignored!",
            target,
        );
    } else {
        log::info!("loaded workload: {}", workload.name);
    }

    let prepare = parse_prepare(&args.prepare)?;

    // Randomize key space
    // const NUM_CODING_GROUPS: usize = 1;
    // let mut key_space = (0..(workload.count * NUM_CODING_GROUPS)).collect::<Vec<_>>();
    // key_space.shuffle(&mut rand::thread_rng());
    // key_space.truncate(workload.count);
    // key_space.sort_unstable();
    // let key_space = Arc::new(key_space);

    ////////////////////////////////// Run ///////////////////////////////////
    // Initialize RPC
    let nexus = Nexus::new((
        local_ip().expect("failed to get local IPv4 address"),
        args.sm_port,
    ));

    // Prepare for core binding.
    let cores = {
        let cores = utils::parse_core_list(&args.bind_core)?;
        match core_affinity::get_core_ids() {
            Some(available_cores) => cores
                .into_iter()
                .filter(|&core_id| available_cores.contains(&CoreId { id: core_id }))
                .collect::<Vec<_>>(),
            None => Vec::new(),
        }
    };
    let nthreads = cmp::min(svr_rpcs, cores.len());
    if nthreads == 0 {
        anyhow::bail!("no available cores");
    }
    let mut core_iter = cores.into_iter();

    // Prepare perfmon.
    let perfmon = Arc::new(Perfmon::new(nthreads));
    let fin = Arc::new(AtomicUsize::new(nthreads));

    // Run workload.
    let barrier = Arc::new(Barrier::new(nthreads));
    let worker_handles = (0..nthreads)
        .map(|tid| {
            let core_id = core_iter.next().unwrap();
            let fin = fin.clone();

            let cluster = cluster.clone();
            let workload = workload.clone();

            let nexus = nexus.clone();
            let perfmon = perfmon.clone();
            let barrier = barrier.clone();

            thread::spawn(move || {
                // Bind thread to core.
                core_affinity::set_for_current(CoreId { id: core_id });

                // Run the workload and notify the main thread when it finishes.
                runners::run_workload_thread::<u64>(
                    (tid, nthreads),
                    cluster,
                    (workload, prepare, policy),
                    nexus,
                    perfmon,
                    barrier,
                );
                fin.fetch_sub(1, Ordering::Relaxed);
            })
        })
        .collect::<Vec<_>>();

    // Wait for all threads to finish.
    const PERF_REPORT_INTERVAL_MS: u64 = 500;
    let mut stats = Vec::with_capacity(64);

    // Setup SIGTERM handler.
    let term = Arc::new(AtomicBool::new(false));
    signal_hook::flag::register(signal_hook::consts::SIGTERM, Arc::clone(&term))?;

    // Wait for all threads to finish or a SIGTERM input.
    while fin.load(Ordering::Relaxed) > 0 && !term.load(Ordering::Relaxed) {
        // Print performance report
        let stat = perfmon.stat();
        if !stat.is_meaningless() {
            if args.dump.is_some() {
                println!("thpt = {}", stat.throughput);
            } else {
                println!("{}", stat.dump()?);
            }
            stats.push(stat);
        }

        // Sleep for a period of time.
        // Use `busy_sleep` because cores are all busy, and raw `sleep` may be very inaccurate.
        thread::sleep(Duration::from_millis(PERF_REPORT_INTERVAL_MS));
    }

    // Print final stat.
    if workload.duration != 0 {
        let stat = perfmon.stat();
        if args.dump.is_some() {
            println!("thpt = {}", stat.throughput);
        } else {
            println!("{}", stat.dump()?);
        }
        if !stat.is_meaningless() {
            stats.push(stat);
        }
    }

    // Dump stats.
    if let Some(ref dump_file) = args.dump {
        // Append hostname to the end
        let dump_file = match dump_file.as_str() {
            "/dev/null" => dump_file.clone().into(),
            _ => utils::append_hostname(dump_file),
        };

        log::info!("dumping stats to file: {}", dump_file.display());
        if stats.is_empty() {
            log::warn!("no stats to dump!");
        } else {
            let mut wtr = csv::Writer::from_path(dump_file)?;
            for stat in stats {
                wtr.serialize(stat)?;
            }
            wtr.flush()?;
        }
    }

    // Join handles.
    for handle in worker_handles {
        handle.join().unwrap();
    }
    Ok(())
}
