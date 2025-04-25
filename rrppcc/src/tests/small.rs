//! Simple tests for small RPCs.

use super::*;
use futures::join;

const HELLO_WORLD: &str = "hello, world!";
const RPC_HELLO: ReqType = 42;

/// Test a single RPC request.
#[test]
fn single_req() {
    let cli_port = next_port();
    let svr_port = next_port();

    let (tx, rx) = mpsc::channel();
    let (tx2, rx2) = mpsc::channel();

    let handle = thread::spawn(move || {
        let nx = Nexus::new(("127.0.0.1", svr_port));
        let mut rpc = Rpc::new(&nx, 2, NIC_NAME, 1);
        rpc.set_handler(RPC_HELLO, |req| async move {
            let mut resp_buf = req.pre_resp_buf();
            unsafe {
                ptr::copy_nonoverlapping(
                    HELLO_WORLD.as_ptr(),
                    resp_buf.as_mut_ptr(),
                    HELLO_WORLD.len(),
                )
            };
            resp_buf.set_len(HELLO_WORLD.len());
            resp_buf
        });

        tx2.send(()).unwrap();
        while let Err(_) = rx.try_recv() {
            rpc.progress();
        }
    });

    let nx = Nexus::new(("127.0.0.1", cli_port));
    let rpc = Rpc::new(&nx, 1, NIC_NAME, 1);

    rx2.recv().unwrap();
    let sess = rpc.create_session(("127.0.0.1", svr_port), 2);
    assert!(block_on(sess.connect()));

    // Prepare buffer.
    let req_buf = rpc.alloc_msgbuf(16);
    let mut resp_buf = rpc.alloc_msgbuf(16);

    // Send request.
    let request = sess.request(RPC_HELLO, &req_buf, &mut resp_buf);
    block_on(request);

    // Validation.
    let payload = {
        let mut payload = Vec::with_capacity(resp_buf.len());
        unsafe {
            ptr::copy_nonoverlapping(resp_buf.as_ptr(), payload.as_mut_ptr(), resp_buf.len());
            payload.set_len(resp_buf.len());
        }
        String::from_utf8(payload).unwrap()
    };
    assert_eq!(payload, HELLO_WORLD);

    tx.send(()).unwrap();
    handle.join().unwrap();
}

/// Test multiple synchronous requests in a session.
#[test]
fn multiple_reqs() {
    let cli_port = next_port();
    let svr_port = next_port();

    let (tx, rx) = mpsc::channel();
    let (tx2, rx2) = mpsc::channel();

    let handle = thread::spawn(move || {
        let nx = Nexus::new(("127.0.0.1", svr_port));
        let mut rpc = Rpc::new(&nx, 2, NIC_NAME, 1);
        rpc.set_handler(RPC_HELLO, |req| async move {
            let mut resp_buf = req.pre_resp_buf();
            assert!(resp_buf.len() == 4080);
            unsafe {
                ptr::copy_nonoverlapping(
                    HELLO_WORLD.as_ptr(),
                    resp_buf.as_mut_ptr(),
                    HELLO_WORLD.len(),
                )
            };
            resp_buf.set_len(HELLO_WORLD.len());
            resp_buf
        });

        tx2.send(()).unwrap();
        while let Err(_) = rx.try_recv() {
            rpc.progress();
        }
    });

    let nx = Nexus::new(("127.0.0.1", cli_port));
    let rpc = Rpc::new(&nx, 1, NIC_NAME, 1);

    rx2.recv().unwrap();
    let sess = rpc.create_session(("127.0.0.1", svr_port), 2);
    assert!(block_on(sess.connect()));

    // Prepare buffer.
    let req_buf = rpc.alloc_msgbuf(16);
    let mut resp_buf = rpc.alloc_msgbuf(16);

    // Send request.
    for _ in 0..100000 {
        let request = sess.request(RPC_HELLO, &req_buf, &mut resp_buf);
        block_on(request);

        // Validation.
        let payload = {
            let mut payload = Vec::with_capacity(resp_buf.len());
            unsafe {
                ptr::copy_nonoverlapping(resp_buf.as_ptr(), payload.as_mut_ptr(), resp_buf.len());
                payload.set_len(resp_buf.len());
            }
            String::from_utf8(payload).unwrap()
        };
        assert_eq!(payload, HELLO_WORLD);
    }

    tx.send(()).unwrap();
    handle.join().unwrap();
}

/// Test multiple concurrent requests (> window size) in a session.
#[test]
fn concurrent_reqs() {
    let cli_port = next_port();
    let svr_port = next_port();

    let (tx, rx) = mpsc::channel();
    let (tx2, rx2) = mpsc::channel();

    let handle = thread::spawn(move || {
        let nx = Nexus::new(("127.0.0.1", svr_port));
        let mut rpc = Rpc::new(&nx, 2, NIC_NAME, 1);
        rpc.set_handler(RPC_HELLO, |req| async move {
            let mut resp_buf = req.pre_resp_buf();
            unsafe {
                ptr::copy_nonoverlapping(
                    HELLO_WORLD.as_ptr(),
                    resp_buf.as_mut_ptr(),
                    HELLO_WORLD.len(),
                )
            };
            resp_buf.set_len(HELLO_WORLD.len());
            resp_buf
        });

        tx2.send(()).unwrap();
        while let Err(_) = rx.try_recv() {
            rpc.progress();
        }
    });

    let nx = Nexus::new(("127.0.0.1", cli_port));
    let rpc = Rpc::new(&nx, 1, NIC_NAME, 1);

    rx2.recv().unwrap();
    let sess = rpc.create_session(("127.0.0.1", svr_port), 2);
    assert!(block_on(sess.connect()));

    // Multiple concurrent buffer & requests.
    const N: usize = 64;
    let req_bufs: [_; N] = array::from_fn(|_| rpc.alloc_msgbuf(16));
    let mut resp_bufs: [_; N] = array::from_fn(|_| rpc.alloc_msgbuf(16));

    // Issue requests.
    let mut requests = Vec::with_capacity(N);
    let mut resp_slice = &mut resp_bufs[..];
    for i in 0..N {
        let (resp, rest) = resp_slice.split_first_mut().unwrap();
        requests.push(sess.request(RPC_HELLO, &req_bufs[i], resp));
        resp_slice = rest;
    }

    // Wait for all requests to complete.
    block_on(join_all(requests));

    // Validation.
    for resp_buf in resp_bufs {
        let payload = {
            let mut payload = Vec::with_capacity(resp_buf.len());
            unsafe {
                ptr::copy_nonoverlapping(resp_buf.as_ptr(), payload.as_mut_ptr(), resp_buf.len());
                payload.set_len(resp_buf.len());
            }
            String::from_utf8(payload).unwrap()
        };
        assert_eq!(payload, HELLO_WORLD);
    }

    tx.send(()).unwrap();
    handle.join().unwrap();
}

/// Test nested small RPC requests.
#[test]
fn nested() {
    let cli_port = next_port();
    let svr_port_1 = next_port();
    let svr_port_2 = next_port();

    // cli ---> svr_1 ---> svr_2
    let stop_flag = Arc::new(AtomicBool::new(false));
    let (tx_rdy2, rx_rdy2) = mpsc::channel();
    let (tx_rdy1, rx_rdy1) = mpsc::channel();

    let svr2_handle = thread::spawn({
        let stop_flag = stop_flag.clone();
        move || {
            let nx = Nexus::new(("127.0.0.1", svr_port_2));
            let mut rpc = Rpc::new(&nx, 3, NIC_NAME, 1);
            rpc.set_handler(RPC_HELLO, |req| async move {
                let mut resp_buf = req.pre_resp_buf();
                unsafe {
                    ptr::copy_nonoverlapping(
                        HELLO_WORLD.as_ptr(),
                        resp_buf.as_mut_ptr(),
                        HELLO_WORLD.len(),
                    )
                };
                resp_buf.set_len(HELLO_WORLD.len());
                resp_buf
            });

            tx_rdy2.send(()).unwrap();
            while !stop_flag.load(Ordering::SeqCst) {
                rpc.progress();
            }
        }
    });

    let svr1_handle = thread::spawn({
        let stop_flag = stop_flag.clone();
        move || {
            let nx = Nexus::new(("127.0.0.1", svr_port_1));
            let mut rpc = Rpc::new(&nx, 2, NIC_NAME, 1);
            rpc.set_handler(RPC_HELLO, |req| async move {
                let nest_send_buf = req.rpc().alloc_msgbuf(16);
                let mut nest_resp_buf1 = req.rpc().alloc_msgbuf(16);
                let mut nest_resp_buf2 = req.rpc().alloc_msgbuf(16);

                let sess = req.rpc().get_session(0).unwrap();
                let nest_req1 = sess.request(RPC_HELLO, &nest_send_buf, &mut nest_resp_buf1);
                let nest_req2 = sess.request(RPC_HELLO, &nest_send_buf, &mut nest_resp_buf2);
                join!(nest_req1, nest_req2);

                let mut resp_buf = req.pre_resp_buf();
                resp_buf.set_len(nest_resp_buf1.len() + nest_resp_buf2.len());
                unsafe {
                    ptr::copy_nonoverlapping(
                        nest_resp_buf1.as_ptr(),
                        resp_buf.as_mut_ptr(),
                        nest_resp_buf1.len(),
                    );
                    ptr::copy_nonoverlapping(
                        nest_resp_buf2.as_ptr(),
                        resp_buf.as_mut_ptr().add(nest_resp_buf1.len()),
                        nest_resp_buf2.len(),
                    );
                }
                resp_buf
            });

            // Connect to svr_2.
            rx_rdy2.recv().unwrap();

            let sess = rpc.create_session(("127.0.0.1", svr_port_2), 3);
            assert!(block_on(sess.connect()));
            assert!(sess.id() == 0);

            tx_rdy1.send(()).unwrap();
            while !stop_flag.load(Ordering::SeqCst) {
                rpc.progress();
            }
        }
    });

    let nx = Nexus::new(("127.0.0.1", cli_port));
    let rpc = Rpc::new(&nx, 1, NIC_NAME, 1);

    // Connect to svr_1.
    rx_rdy1.recv().unwrap();

    let sess = rpc.create_session(("127.0.0.1", svr_port_1), 2);
    assert!(block_on(sess.connect()));

    // Prepare buffer.
    let req_buf = rpc.alloc_msgbuf(16);
    let mut resp_buf = rpc.alloc_msgbuf(32);

    // Send request.
    let expected = HELLO_WORLD.to_owned() + HELLO_WORLD;
    for _ in 0..100000 {
        unsafe { ptr::write_bytes(resp_buf.as_mut_ptr(), 0, 16) };

        let request = sess.request(RPC_HELLO, &req_buf, &mut resp_buf);
        block_on(request);

        // Validation.
        let payload = {
            let mut payload = Vec::with_capacity(resp_buf.len());
            unsafe {
                ptr::copy_nonoverlapping(resp_buf.as_ptr(), payload.as_mut_ptr(), resp_buf.len());
                payload.set_len(resp_buf.len());
            }
            String::from_utf8(payload).unwrap()
        };
        assert_eq!(payload, expected);
    }

    stop_flag.store(true, Ordering::SeqCst);
    svr1_handle.join().unwrap();
    svr2_handle.join().unwrap();
}
