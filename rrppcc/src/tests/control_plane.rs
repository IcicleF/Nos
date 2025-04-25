//! Control-plane functionalities, including creating RPCs and sessions.

use super::*;

/// Test if `Rpc`s can be created.
#[test]
fn create_rpcs() {
    let nexus = Nexus::new(("127.0.0.1", next_port()));
    let handles = (1..=16).map(|i| {
        let nexus = nexus.clone();
        thread::spawn(move || {
            let _ = Rpc::new(&nexus, i, NIC_NAME, 1);
        })
    });
    for handle in handles {
        handle.join().unwrap();
    }
}

/// Test if `Session`s can be created and connected.
#[test]
fn connect_rpcs() {
    let cli_port = next_port();
    let svr_port = next_port();

    let (tx, rx) = mpsc::channel();
    let handle = thread::spawn(move || {
        let nx = Nexus::new(("127.0.0.1", svr_port));
        let rpc = Rpc::new(&nx, 3, NIC_NAME, 1);
        while let Err(_) = rx.try_recv() {
            rpc.progress();
        }
    });

    let nx = Nexus::new(("127.0.0.1", cli_port));
    let r1 = Rpc::new(&nx, 1, NIC_NAME, 1);
    let r2 = Rpc::new(&nx, 2, NIC_NAME, 1);

    (0..10).for_each(|i| {
        let sess = r1.create_session(("127.0.0.1", svr_port), 3);
        assert_eq!(sess.id(), i);
        assert_eq!(sess.is_connected(), false);

        assert!(block_on(async { sess.connect().await }));
        assert!(sess.is_connected());
    });
    (0..10).for_each(|i| {
        let sess = r2.create_session(("127.0.0.1", svr_port), 3);
        assert_eq!(sess.id(), i);
        assert_eq!(sess.is_connected(), false);

        assert!(block_on(async { sess.connect().await }));
        assert!(sess.is_connected());
    });

    tx.send(()).unwrap();
    handle.join().unwrap();
}
