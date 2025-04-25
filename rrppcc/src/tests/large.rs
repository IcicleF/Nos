//! Simple tests for large RPCs.

use futures::join;

use super::*;

const HELLO_WORLD: &str = "hello, world!";
const RPC_HELLO: ReqType = 42;

const LARGE_MSG_LEN: usize = 16384;

/// Test a single RPC with large request.
#[test]
fn large_req() {
    let cli_port = next_port();
    let svr_port = next_port();

    let (tx, rx) = mpsc::channel();
    let (tx2, rx2) = mpsc::channel();

    // Request length, intentionally some magic to verify it.
    let req_byte = rand::thread_rng().gen::<u8>();

    let handle = thread::spawn(move || {
        let nx = Nexus::new(("127.0.0.1", svr_port));
        let mut rpc = Rpc::new(&nx, 2, NIC_NAME, 1);
        rpc.set_handler(RPC_HELLO, move |req| async move {
            assert_eq!(req.req_buf().len(), LARGE_MSG_LEN);
            let payload = unsafe { req.req_buf().as_slice() };
            assert!(payload.iter().all(|&b| b == req_byte));

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

    for _ in 0..10000 {
        // Prepare buffer.
        let mut req_buf = rpc.alloc_msgbuf(LARGE_MSG_LEN);
        let mut resp_buf = rpc.alloc_msgbuf(16);

        // Send request.
        unsafe { ptr::write_bytes(req_buf.as_mut_ptr(), req_byte, req_buf.len()) };
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

/// Test a single RPC with large response.
#[test]
fn large_resp() {
    let cli_port = next_port();
    let svr_port = next_port();

    let (tx, rx) = mpsc::channel();
    let (tx2, rx2) = mpsc::channel();

    // Response length, intentionally some magic to verify it.
    let resp_byte = rand::thread_rng().gen::<u8>();

    let handle = thread::spawn(move || {
        let nx = Nexus::new(("127.0.0.1", svr_port));
        let mut rpc = Rpc::new(&nx, 2, NIC_NAME, 1);
        rpc.set_handler(RPC_HELLO, move |req| async move {
            let mut resp_buf = req.rpc().alloc_msgbuf(LARGE_MSG_LEN);
            unsafe { ptr::write_bytes(resp_buf.as_mut_ptr(), resp_byte, LARGE_MSG_LEN) };
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

    for _ in 0..100000 {
        // Prepare buffer.
        let mut req_buf = rpc.alloc_msgbuf(16);
        let mut resp_buf = rpc.alloc_msgbuf(50000);

        // Send request.
        unsafe { ptr::write_bytes(req_buf.as_mut_ptr(), 0, req_buf.len()) };
        let request = sess.request(RPC_HELLO, &req_buf, &mut resp_buf);
        block_on(request);

        // Validation.
        assert!(resp_buf.len() == LARGE_MSG_LEN);
        let payload = unsafe { resp_buf.as_slice() };
        assert!(payload.iter().all(|&b| b == resp_byte));
    }

    tx.send(()).unwrap();
    handle.join().unwrap();
}

/// Test nested large RPCs.
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
                let nest_send_buf = req.rpc().alloc_msgbuf(LARGE_MSG_LEN);
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
    let mut req_buf = rpc.alloc_msgbuf(LARGE_MSG_LEN);
    unsafe { ptr::write_bytes(req_buf.as_mut_ptr(), 0, LARGE_MSG_LEN) };

    let mut resp_buf = rpc.alloc_msgbuf(64);

    // Send request.
    let expected = HELLO_WORLD.to_owned() + HELLO_WORLD;
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
        assert_eq!(payload, expected);
    }

    stop_flag.store(true, Ordering::SeqCst);
    svr1_handle.join().unwrap();
    svr2_handle.join().unwrap();
}
