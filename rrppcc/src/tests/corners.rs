//! Corner-case tests.

use super::*;

const RPC_HELLO: ReqType = 42;
const RPC_NOMSG: ReqType = 99;
const HELLO_WORLD: &str = "hello, world!";

/// Test if zero-sized requests & responses can be correctly handled.
#[test]
fn zero_sized() {
    let cli_port = next_port();
    let svr_port = next_port();

    let (tx, rx) = mpsc::channel();
    let (tx2, rx2) = mpsc::channel();

    let handle = thread::spawn(move || {
        let nx = Nexus::new(("127.0.0.1", svr_port));
        let mut rpc = Rpc::new(&nx, 2, NIC_NAME, 1);
        rpc.set_handler(RPC_NOMSG, |req| async move {
            let mut resp_buf = req.pre_resp_buf();
            resp_buf.set_len(0);
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
    let req_buf = rpc.alloc_msgbuf(0);
    let mut resp_buf = rpc.alloc_msgbuf(1);

    // Send request.
    let request = sess.request(RPC_NOMSG, &req_buf, &mut resp_buf);
    block_on(request);

    // Validation.
    assert_eq!(resp_buf.len(), 0);

    tx.send(()).unwrap();
    handle.join().unwrap();
}

/// Test if multiple requests can share the same `MsgBuf`.
#[test]
fn shared_req() {
    let cli_port = next_port();
    let svr_port = next_port();

    let (tx, rx) = mpsc::channel();
    let (tx2, rx2) = mpsc::channel();

    let handle = thread::spawn(move || {
        let nx = Nexus::new(("127.0.0.1", svr_port));
        let mut rpc = Rpc::new(&nx, 2, NIC_NAME, 1);
        rpc.set_handler(RPC_HELLO, |req| async move {
            let mut resp_buf = req.pre_resp_buf();
            let req_buf = req.req_buf();
            unsafe {
                ptr::copy_nonoverlapping(req_buf.as_ptr(), resp_buf.as_mut_ptr(), req_buf.len())
            };
            resp_buf.set_len(req_buf.len());
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

    // Single shared request.
    let magic_byte = rand::random::<u8>();
    let mut req_buf = rpc.alloc_msgbuf(8);
    unsafe { ptr::write_bytes(req_buf.as_mut_ptr(), magic_byte, req_buf.len()) };

    // Multiple response buffers.
    const N: usize = 64;
    let mut resp_bufs: [_; N] = array::from_fn(|_| rpc.alloc_msgbuf(16));

    // Issue requests.
    let mut requests = Vec::with_capacity(N);
    let mut resp_slice = &mut resp_bufs[..];
    for _ in 0..N {
        let (resp, rest) = resp_slice.split_first_mut().unwrap();
        requests.push(sess.request(RPC_HELLO, &req_buf, resp));
        resp_slice = rest;
    }

    // Wait for all requests to complete.
    block_on(join_all(requests));

    // Validation.
    for resp_buf in resp_bufs {
        let payload = unsafe { resp_buf.as_slice() };
        assert_eq!(payload, &[magic_byte; 8]);
    }

    tx.send(()).unwrap();
    handle.join().unwrap();
}

/// Test multiple concurrent requests (> window size) in a session.
#[test]
fn aborting_reqs() {
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

    // Abort some requests.
    const ABORT_START: usize = 5;
    const ABORT_END: usize = 31;

    let mut filtered_reqs = Vec::new();
    for (i, req) in requests.into_iter().enumerate() {
        if i >= ABORT_START && i < ABORT_END {
            req.abort();
        } else {
            filtered_reqs.push(req);
        }
    }

    // Wait for all requests to complete.
    block_on(join_all(filtered_reqs));

    // Validation.
    for (i, resp_buf) in resp_bufs.into_iter().enumerate() {
        if (ABORT_START..ABORT_END).contains(&i) {
            continue;
        }

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

#[test]
#[should_panic(expected = "RequestHandle not dropped after handler finishes")]
fn moving_req_handle() {
    let cli_port = next_port();
    let svr_port = next_port();

    let (tx, rx) = mpsc::channel();
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
            std::mem::forget(req);

            resp_buf.set_len(HELLO_WORLD.len());
            resp_buf
        });

        tx.send(()).unwrap();
        loop {
            rpc.progress();
        }
    });

    let nx = Nexus::new(("127.0.0.1", cli_port));
    let rpc = Rpc::new(&nx, 1, NIC_NAME, 1);

    rx.recv().unwrap();
    let sess = rpc.create_session(("127.0.0.1", svr_port), 2);
    assert!(block_on(sess.connect()));

    // Prepare buffer.
    let req_buf = rpc.alloc_msgbuf(16);
    let mut resp_buf = rpc.alloc_msgbuf(16);

    // Send request, and make some progress to send out the request.
    let _request = sess.request(RPC_HELLO, &req_buf, &mut resp_buf);
    for _ in 0..10 {
        rpc.progress();
    }

    // Let the test panic in the same way as the server thread.
    if let Err(e) = handle.join() {
        if let Some(e) = e.downcast_ref::<&'static str>() {
            panic!("server panicked: {}", e);
        }
    }
}
