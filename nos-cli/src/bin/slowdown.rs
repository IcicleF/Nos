use std::{env, error, mem, ptr};

use futures::executor::block_on;
use local_ip_address::local_ip;
use nos::ctrl::{Cluster, Role, NIC_NAME, NIC_PHYPORT};
use rrppcc::*;

const RPC_PORT: u16 = 31849;

fn main() -> Result<(), Box<dyn error::Error>> {
    let args = env::args().collect::<Vec<_>>();
    if args.len() < 3 {
        eprintln!("Usage: {} <path-to-config> <nodes>", args[0]);
        eprintln!("Specify +x to slow a node down and -x to recover");
        return Err("invalid arguments".into());
    }

    let mut conf_file = args[1].clone();
    if !conf_file.ends_with(".toml") {
        eprintln!("warning: inferring unprovided .toml suffix");
        conf_file.push_str(".toml");
    }

    let nodes = args[2..]
        .iter()
        .map(|s| {
            let mut s = s.chars();
            let sign = s.next().unwrap();
            let num = s.collect::<String>().parse::<usize>().unwrap();

            let slowdown = match sign {
                '+' => true,
                '-' => false,
                _ => panic!("invalid sign"),
            };
            (slowdown, num)
        })
        .collect::<Vec<_>>();

    {
        let to_slowdown = nodes
            .iter()
            .filter(|(slowdown, _)| *slowdown)
            .map(|(_, node)| node)
            .collect::<Vec<_>>();
        let to_recover = nodes
            .iter()
            .filter(|(slowdown, _)| !*slowdown)
            .map(|(_, node)| node)
            .collect::<Vec<_>>();
        println!("+ slowdown: {:?}", to_slowdown);
        println!("- slowdown: {:?}", to_recover);
    }

    let cluster = Cluster::load_toml_file(&conf_file, Role::Client)?;
    let nexus = Nexus::new((local_ip().unwrap(), RPC_PORT));
    let rpc = Rpc::new(&nexus, 1, NIC_NAME, NIC_PHYPORT);
    let sessions = cluster
        .peers()
        .iter()
        .map(|peer| {
            let sess = rpc.create_session(peer.uri(), 1);
            assert!(block_on(sess.connect()));
            sess
        })
        .collect::<Vec<_>>();

    let mut req_buf = rpc.alloc_msgbuf(mem::size_of::<u64>());
    let mut resp_buf = rpc.alloc_msgbuf(8);

    for sess in &sessions {
        for &(slowdown, node) in &nodes {
            let ty = if slowdown {
                nos::rpc_types::RPC_MAKE_SLOWED
            } else {
                nos::rpc_types::RPC_MAKE_SLOW_RECOVERED
            };
            unsafe { ptr::write(req_buf.as_mut_ptr() as *mut u64, node as u64) };
            let req = sess.request(ty, &req_buf, &mut resp_buf);
            block_on(req);
        }
    }

    println!("ok");
    Ok(())
}
