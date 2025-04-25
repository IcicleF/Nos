use criterion::{criterion_group, criterion_main, Criterion};

use futures::executor::block_on;
use rrppcc::{type_alias::ReqType, *};
use std::{ptr, sync::mpsc, thread};

const LOCALHOST: &'static str = "127.0.0.1";
const NIC_NAME: &'static str = "mlx5_0";

pub fn benchmark_idle(c: &mut Criterion) {
    const PORT: u16 = 31850;

    let nx = Nexus::new((LOCALHOST, PORT));
    let rpc = Rpc::new(&nx, 1, NIC_NAME, 1);

    // Benchmark idle event-loop latency.
    c.bench_function("idle-eventloop", |b| b.iter(|| rpc.progress()));
}

pub fn benchmark_sync(c: &mut Criterion) {
    const CLI_PORT: u16 = 31850;
    const SVR_PORT: u16 = 31851;

    const RPC_SMALL: ReqType = 42;
    const SMALL_RPC_LEN: usize = 8;
    const RPC_LARGE: ReqType = 43;
    const LARGE_RPC_LEN: usize = 4 << 10;

    let (tx, rx) = mpsc::channel();
    let (tx2, rx2) = mpsc::channel();

    let handle = thread::spawn(move || {
        let nx = Nexus::new((LOCALHOST, SVR_PORT));
        let mut rpc = Rpc::new(&nx, 2, NIC_NAME, 1);

        rpc.set_handler(RPC_SMALL, |req| async move {
            let mut resp_buf = req.pre_resp_buf();
            unsafe { ptr::write_bytes(resp_buf.as_mut_ptr(), 1, SMALL_RPC_LEN) };
            resp_buf.set_len(SMALL_RPC_LEN);
            resp_buf
        });
        rpc.set_handler(RPC_LARGE, |req| async move {
            let mut resp_buf = req.rpc().alloc_msgbuf(LARGE_RPC_LEN);
            unsafe { ptr::write_bytes(resp_buf.as_mut_ptr(), 1, LARGE_RPC_LEN) };
            resp_buf
        });

        tx2.send(()).unwrap();
        while let Err(_) = rx.try_recv() {
            rpc.progress();
        }
    });

    let nx = Nexus::new((LOCALHOST, CLI_PORT));
    let rpc = Rpc::new(&nx, 1, NIC_NAME, 1);

    rx2.recv().unwrap();
    let sess = rpc.create_session((LOCALHOST, SVR_PORT), 2);
    assert!(block_on(sess.connect()));

    // Prepare small buffer.
    let mut req_buf = rpc.alloc_msgbuf(SMALL_RPC_LEN);
    let mut resp_buf = rpc.alloc_msgbuf(SMALL_RPC_LEN);
    unsafe { ptr::write_bytes(req_buf.as_mut_ptr(), 1, SMALL_RPC_LEN) };

    // Benchmark synchronous pingpong-only RPC performance.
    c.bench_function("sync-pingpong", |b| {
        b.iter(|| {
            let request = sess.request(RPC_SMALL, &req_buf, &mut resp_buf);
            block_on(request);
        })
    });

    // Prepare large buffer.
    let mut req_buf = rpc.alloc_msgbuf(LARGE_RPC_LEN);
    let mut resp_buf = rpc.alloc_msgbuf(LARGE_RPC_LEN);
    unsafe { ptr::write_bytes(req_buf.as_mut_ptr(), 1, LARGE_RPC_LEN) };

    // Benchmark synchronous large RPC performance.
    c.bench_function("sync-pingpong-large", |b| {
        b.iter(|| {
            let request = sess.request(RPC_SMALL, &req_buf, &mut resp_buf);
            block_on(request);
        })
    });

    tx.send(()).unwrap();
    handle.join().unwrap();
}

criterion_group!(benches, benchmark_idle, benchmark_sync);
criterion_main!(benches);
