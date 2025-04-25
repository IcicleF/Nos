use futures::executor::block_on;
use rrppcc::{type_alias::*, *};
use std::{ptr, sync::mpsc, thread};

fn main() {
    const CLI_URI: &'static str = "127.0.0.1:31850";
    const SVR_URI: &'static str = "127.0.0.1:31851";
    const NIC_NAME: &'static str = "mlx5_0";

    const RPC_HELLO: ReqType = 42;
    const HELLO_WORLD: &str = "Hello, world!";

    let (finish_tx, finish_rx) = mpsc::channel();
    let (svr_ready_tx, svr_ready_rx) = mpsc::channel();

    // Server thread.
    let handle = thread::spawn(move || {
        let nx = Nexus::new(SVR_URI);
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

        svr_ready_tx.send(()).unwrap();
        while let Err(_) = finish_rx.try_recv() {
            rpc.progress();
        }
    });

    // Client thread.
    let nx = Nexus::new(CLI_URI);
    let rpc = Rpc::new(&nx, 1, NIC_NAME, 1);

    svr_ready_rx.recv().unwrap();
    let sess = rpc.create_session(SVR_URI, 2);
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

    finish_tx.send(()).unwrap();
    handle.join().unwrap();
}
