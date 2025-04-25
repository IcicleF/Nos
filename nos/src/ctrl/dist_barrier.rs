use std::io::prelude::*;
use std::net::*;

use crate::ctrl::Cluster;

/// Distributed barrier.
///
/// Synchronize all processes in the cluster.
pub enum DistBarrier {}

impl DistBarrier {
    /// Wait for all processes in the cluster to reach this point of the code
    /// using the given TCP port.
    ///
    /// ## Synchronization scheme
    ///
    /// The process with rank 0 will listen on the given port. All other
    /// processes will try to connect to the process with rank 0. Once the
    /// rank 0 process has received all connections, it will send a byte to
    /// all other processes to let them proceed.
    pub fn wait_on_port(cluster: &Cluster, port: u16) {
        if cluster.rank() == 0 {
            let inaddr_any = SocketAddr::new(
                match cluster[0].ip() {
                    IpAddr::V4(_) => IpAddr::V4(Ipv4Addr::UNSPECIFIED),
                    IpAddr::V6(_) => IpAddr::V6(Ipv6Addr::UNSPECIFIED),
                },
                port,
            );
            let listener = TcpListener::bind(inaddr_any).unwrap();

            let mut streams = vec![];
            for _ in 1..cluster.size() {
                streams.push(listener.accept().unwrap().0);
            }

            let buf = [0; 1];
            for mut stream in streams {
                stream.write_all(&buf).unwrap();
            }
        } else {
            let server_addr = SocketAddr::new(cluster[0].ip(), port);
            let mut stream = loop {
                if let Ok(stream) = TcpStream::connect(server_addr) {
                    break stream;
                }
                std::thread::sleep(std::time::Duration::from_millis(100));
            };

            let mut buf = [0; 1];
            stream.read_exact(&mut buf).unwrap();
        }
    }

    /// Wait for all processes in the cluster to reach this point of the code
    /// using the default TCP port `13373`.
    pub fn wait(cluster: &Cluster) {
        const PORT: u16 = 13373;
        Self::wait_on_port(cluster, PORT);
    }
}
