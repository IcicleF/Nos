//! Dataset preparer.

use std::ops::Range;
use std::sync::atomic::*;
use std::time::Duration;

use nos::ctrl::Cluster;
use nos::Key;
use quanta::Instant;
use rrppcc::*;

use crate::distributed;
use crate::workload::{Workload, WorkloadGenerator};

static PREPARE_CNT: AtomicUsize = AtomicUsize::new(0);
const PREPARE_UPDATE_INTERVAL: usize = 1000;
const PREPARE_REPORT_INTERVAL: Duration = Duration::from_secs(2);

/// Prepare the dataset.
pub async fn prepare<'a, 'r, K>(
    cluster: &'a Cluster,
    workload: &'a Workload,
    keyspace: Range<usize>,
    rpc: &'r Rpc,
    sessions: &'a [Session<'r>],
    print: bool,
) where
    K: Key,
{
    // Prepare buffers.
    let mut req_buf = rpc.alloc_msgbuf(distributed::buf_len::<K>());
    let mut resp_buf = rpc.alloc_msgbuf(distributed::buf_len::<K>());
    let value = vec![1u8; workload.value_size];

    let wg = WorkloadGenerator::new(workload);
    let mut cnt = 0;
    let mut time_since_last_report = Instant::now();

    for idx in keyspace {
        let key = K::from(idx as u64);
        let len = wg.value_len(key);

        let primary = (key % cluster.size() as u64) as usize;
        distributed::write(
            &sessions[primary],
            &mut req_buf,
            &mut resp_buf,
            key,
            &value[..len],
        )
        .await;

        cnt += 1;
        if cnt == PREPARE_UPDATE_INTERVAL {
            let report_cnt = PREPARE_CNT.fetch_add(cnt, Ordering::Relaxed);
            if time_since_last_report.elapsed() >= PREPARE_REPORT_INTERVAL {
                log::info!("prepared {} records", report_cnt + cnt);
                time_since_last_report = Instant::now();
            }
            cnt = 0;
        }
    }

    if cnt > 0 && print {
        let report_cnt = PREPARE_CNT.fetch_add(cnt, Ordering::Relaxed);
        log::info!("prepared {} records", report_cnt + cnt);
    }
}

/// Inject failures.
pub async fn inject_failure<'a, 'r>(
    cluster: &'a Cluster,
    faults: &'a [usize],
    rpc: &'r Rpc,
    sessions: &'a [Session<'r>],
) {
    // Prepare buffers.
    let mut req_buf = rpc.alloc_msgbuf(16);
    let mut resp_buf = rpc.alloc_msgbuf(16);

    // Tell me that these nodes are recognized as failed.
    for j in faults {
        cluster[*j].make_failed();
    }

    // Tell all servers that these nodes are recognized as failed.
    for sess in sessions {
        for j in faults {
            distributed::inject_failure(sess, &mut req_buf, &mut resp_buf, *j).await;
        }
    }

    log::info!("injected failures to {:?}", faults);
}
