use std::env;
use std::error;

use futures::executor::block_on;
use local_ip_address::local_ip;
use nos::ctrl::{Cluster, Role, NIC_NAME, NIC_PHYPORT};
use rrppcc::*;

fn main() -> Result<(), Box<dyn error::Error>> {
    let args = env::args().collect::<Vec<_>>();
    if args.len() < 2 {
        eprintln!("Usage: {} [OPTIONS] <path-to-config>", args[0]);
        eprintln!();
        eprintln!("Options:");
        eprintln!("  -n, --no-duplicate-nodes   Don't print duplicate nodes");
        return Err("invalid arguments".into());
    }

    let len = args.len();
    let mut conf_file = args[len - 1].clone();
    if !conf_file.ends_with(".toml") {
        eprintln!("warning: inferring unprovided .toml suffix");
        conf_file.push_str(".toml");
    }

    let cluster = Cluster::load_toml_file(&conf_file, Role::Client)?;
    let peer = &cluster[0];

    let nexus = Nexus::new((local_ip().unwrap(), 31850));
    let rpc = Rpc::new(&nexus, 1, NIC_NAME, NIC_PHYPORT);
    let sess = rpc.create_session(peer.uri(), 1);
    assert!(block_on(sess.connect()));

    let req_buf = rpc.alloc_msgbuf(8);
    let mut resp_buf = rpc.alloc_msgbuf(8);

    let req = sess.request(nos::rpc_types::RPC_REPORT_STATS, &req_buf, &mut resp_buf);
    block_on(req);

    println!("ok");
    Ok(())
}
