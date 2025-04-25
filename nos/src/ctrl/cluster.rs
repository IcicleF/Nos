use std::fmt;
use std::fs::File;
use std::io::{self, prelude::*};
use std::net::*;
use std::ops::Index;
use std::path::{Path, PathBuf};
use std::sync::{atomic::*, Arc};
use std::thread;
use std::time::Duration;

use anyhow::Result;
use local_ip_address::list_afinet_netifas;
use net2::TcpBuilder;
use uuid::Uuid;

fn is_my_ip(ip: IpAddr) -> bool {
    let my_ips = list_afinet_netifas().unwrap();
    my_ips.iter().any(|(_, if_ip)| *if_ip == ip)
}

/// Role of a process.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    /// I am a server with a UUID.
    Server(Uuid),

    /// I am a client.
    Client,
}

/// Peer information.
#[derive(Debug)]
pub struct Peer {
    /// The URI of the peer.
    uri: SocketAddr,

    /// CPU core binding offset.
    cpu: usize,

    /// Index of this peer within its physical machine.
    index: usize,

    /// Whether this peer has (logically) failed.
    failed: AtomicBool,
}

impl Clone for Peer {
    fn clone(&self) -> Self {
        Self {
            uri: self.uri,
            cpu: self.cpu,
            index: self.index,
            failed: AtomicBool::new(self.failed.load(Ordering::Relaxed)),
        }
    }
}

impl PartialEq for Peer {
    fn eq(&self, other: &Self) -> bool {
        self.uri == other.uri && self.cpu == other.cpu
    }
}

impl Eq for Peer {}

impl Ord for Peer {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.uri.cmp(&other.uri).then(self.cpu.cmp(&other.cpu))
    }
}

impl PartialOrd for Peer {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Peer {
    /// Create a new peer with the given URI, CPU core binding offset, and index within
    /// its residing machine.
    pub fn new(uri: SocketAddr, cpu: usize, index: usize) -> Self {
        Self {
            uri,
            cpu,
            index,
            failed: AtomicBool::new(false),
        }
    }

    /// Create a new peer with the given IP address, port, and CPU core binding offset.
    pub fn new_with_ip_port(ip: IpAddr, port: u16, cpu: usize, index: usize) -> Self {
        Self::new(SocketAddr::new(ip, port), cpu, index)
    }

    /// Get the URI of the peer.
    #[inline]
    pub fn uri(&self) -> SocketAddr {
        self.uri
    }

    /// Get the IP address of the peer.
    #[inline]
    pub fn ip(&self) -> IpAddr {
        self.uri.ip()
    }

    /// Get the RPC SM UDP port of the peer.
    #[inline]
    pub fn port(&self) -> u16 {
        self.uri.port()
    }

    /// Get the starting core ID of the peer.
    #[inline]
    pub fn cpu(&self) -> usize {
        self.cpu
    }

    /// Get the index of this peer within its node.
    #[inline]
    pub fn index(&self) -> usize {
        self.index
    }

    /// Get the (logical) liveness of the peer.
    #[inline]
    pub fn is_alive(&self) -> bool {
        !self.failed.load(Ordering::Relaxed)
    }

    /// Mark the peer as (logically) failed.
    #[inline]
    pub fn make_failed(&self) {
        self.failed.store(true, Ordering::SeqCst);
    }

    /// Mark the peer as (logically) alive.
    #[inline]
    pub fn make_alive(&self) {
        self.failed.store(false, Ordering::SeqCst);
    }
}

struct RankAssigner {
    uuid: Uuid,
    halt: Arc<AtomicBool>,
    handle: Option<thread::JoinHandle<()>>,
}

impl RankAssigner {
    const DEFAULT_PORT: u16 = 11451;

    /// Get a temporary file path for the given UUID.
    fn tmp_file(uuid: Uuid) -> PathBuf {
        Path::new("/tmp").join(format!("nos_{}", uuid))
    }

    /// Create a rank assigner server.
    fn new(uuid: Uuid, peers: Vec<Peer>) -> Option<Self> {
        let tmp_file = Self::tmp_file(uuid);

        // Use a temporary file as a lock to sync with other peers on the same
        // node with me. This file will be deleted on drop.
        let file = File::options().write(true).create_new(true).open(tmp_file);
        match file {
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => return None,
            Err(e) => panic!("failed to create temporary file: {}", e),
            _ => (),
        };

        let halt = Arc::new(AtomicBool::new(false));
        let handle = {
            let halt = halt.clone();
            let handle = thread::spawn(move || Self::assigner(halt, peers));
            Some(handle)
        };
        Some(Self { uuid, halt, handle })
    }

    /// Assigner thread.
    fn assigner(halt: Arc<AtomicBool>, peers: Vec<Peer>) {
        let inaddr_any = SocketAddr::new(
            match peers[0].ip() {
                IpAddr::V4(_) => IpAddr::V4(Ipv4Addr::UNSPECIFIED),
                IpAddr::V6(_) => IpAddr::V6(Ipv6Addr::UNSPECIFIED),
            },
            Self::DEFAULT_PORT,
        );
        let listener = TcpListener::bind(inaddr_any).unwrap();

        let mut peers = peers
            .into_iter()
            .map(|peer| (peer, false))
            .collect::<Vec<_>>();
        peers[0].1 = true;

        while !halt.load(Ordering::Relaxed) && peers.iter().any(|(_, assigned)| !assigned) {
            let (mut stream, remote) = listener.accept().unwrap();
            let remote = remote.ip();

            let id = peers
                .iter()
                .position(|(peer, assigned)| peer.ip() == remote && !assigned)
                .unwrap_or_else(|| panic!("no assignable peer for IP {}", remote));

            // Assign rank
            peers[id].1 = true;
            let mut buf = [0u8; 8];
            buf[0..8].copy_from_slice(&(id as u64).to_le_bytes());
            stream.write_all(&buf).unwrap();
        }

        log::info!("rank assigner has successfully assigned all ranks");
    }

    /// Request a rank from the rank assigner server.
    fn request(cli: IpAddr, svr: IpAddr) -> usize {
        let svr = SocketAddr::new(svr, Self::DEFAULT_PORT);
        let mut stream = loop {
            let socket = match cli {
                IpAddr::V4(_) => TcpBuilder::new_v4(),
                IpAddr::V6(_) => TcpBuilder::new_v6(),
            }
            .expect("cannot create TCP socket");
            if let Ok(stream) = socket.connect(svr) {
                break stream;
            }
            thread::sleep(Duration::from_millis(100));
        };

        let mut buf = [0u8; 8];
        stream.read_exact(&mut buf).unwrap();
        usize::from_le_bytes(buf[0..8].try_into().unwrap())
    }
}

impl Drop for RankAssigner {
    fn drop(&mut self) {
        self.halt.store(true, Ordering::SeqCst);
        if let Some(handle) = self.handle.take() {
            handle.join().unwrap();
        }
        std::fs::remove_file(Self::tmp_file(self.uuid)).expect("failed to remove temporary file");
    }
}

/// Cluster information.
///
/// While the cluster can be cloned, they share the same view of peers via an [`Arc`].
/// Marking a peer as failed or alive will affect all clones.
#[derive(Clone)]
pub struct Cluster {
    peers: Arc<Vec<Peer>>,
    id: isize,

    /// Rank assigner for server nodes.
    #[allow(dead_code)]
    ra: Option<Arc<RankAssigner>>,
}

impl fmt::Debug for Cluster {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Cluster")
            .field("peers", &self.peers)
            .field("id", &self.id)
            .finish()
    }
}

impl Cluster {
    const DEFAULT_RPC_SM_PORT: u16 = 31850;

    /// Create a new cluster with the given peers.
    /// Detect the rank of this node automatically.
    ///
    /// If you specify the role as [`Role::Client`], the rank of this node will
    /// be `-1`.
    ///
    /// # Panics
    ///
    /// Panic if the specified role is [`Role::Server`] but the current node's
    /// IP is not in the cluster.
    pub fn new(peers: Vec<Peer>, role: Role) -> Self {
        let uuid = match role {
            Role::Server(uuid) => uuid,
            Role::Client => {
                return Cluster {
                    peers: Arc::new(peers),
                    id: -1,
                    ra: None,
                };
            }
        };
        assert_ne!(peers.len(), 0, "empty cluster");

        let presume_id = peers
            .iter()
            .position(|peer| is_my_ip(peer.ip()))
            .expect("cannot locate self in cluster");

        let mut id = 0;
        let mut ra = None;
        if presume_id == 0 {
            ra = RankAssigner::new(uuid, peers.clone());
        }
        if ra.is_none() {
            id = RankAssigner::request(peers[presume_id].ip(), peers[0].ip());
        }

        Self {
            peers: Arc::new(peers),
            id: id as isize,
            ra: ra.map(Arc::new),
        }
    }

    /// Load TOML cluster configuration.
    ///
    /// All the configuration entries should be placed in the `cluster` table.
    /// Irrelevant fields will be ignored, and you can put the configuration
    /// in your own mixed TOML configuration.
    ///
    /// # Configuration format
    ///
    /// The TOML configuration can be in two forms: `manual` and `auto`.
    /// This should be specified by the key `mode`, but if you skip this key,
    /// the configuration will be treated as `manual`.
    ///
    /// ## Manual mode
    ///
    /// In this mode you must manually list all peers in the `peers` array.
    /// Each element in the array can be a string or a table.
    /// The table should contain three fields:
    ///
    /// - `ip`:   A string, the IPv4/IPv6 address.
    /// - `port`: An integer, the RPC SM UDP port to use.
    /// - `cpu`:  An integer or an integer array, the CPU core binding offset.
    ///           Threads in this peer will try to bind cores starting from the
    ///           given CPU ID. Specifying multiple CPUs will result in the table
    ///           entry representing the same number of peers, and the `port`
    ///           field will now become a base port that automatically increments
    ///           for each peer.
    ///
    /// On top of this, you may merge the IP and the port into a single URI
    /// string with a colon between them. You may omit the `port` field and/or
    /// the `cpu` field, in which case they will be the default values.
    ///
    /// The default port and CPU binding offset can be specified in the `[cluster]`
    /// table with `default-port` and `default-cpu` fields. You can also omit
    /// any of these two fields, in which case they will fallback to `31850` and `0`.
    ///
    /// See the following example:
    ///
    /// ```rust
    /// # use nos::ctrl::*;
    /// let toml = r#"
    ///     [cluster]
    ///     default-port = 31999
    ///
    ///     [[cluster.peers]]
    ///     ip = "10.0.2.1"
    ///     cpu = [0, 18, 36, 54]
    ///
    ///     [[cluster.peers]]
    ///     ip = "10.0.2.2:49999"
    ///     cpu = 0
    /// "#;
    /// let cluster = Cluster::load_toml(toml, Role::Client).unwrap();
    /// assert_eq!(
    ///     cluster.peers(),
    ///     &vec![
    ///         Peer::new("10.0.2.1:31999".parse().unwrap(), 0, 0),
    ///         Peer::new("10.0.2.1:32000".parse().unwrap(), 18, 0),
    ///         Peer::new("10.0.2.1:32001".parse().unwrap(), 36, 0),
    ///         Peer::new("10.0.2.1:32002".parse().unwrap(), 54, 0),
    ///         Peer::new("10.0.2.2:49999".parse().unwrap(), 0, 0),
    ///     ]
    /// );
    /// ```
    ///
    /// If you omit or merge both `port` and `cpu`, you may use a string directly
    /// to represent a peer. The following example shows a minimum 3-node cluster:
    ///
    /// ```rust
    /// # use nos::ctrl::*;
    /// let toml = r#"
    ///     [cluster]
    ///     peers = ["10.0.2.1:31850", "10.0.2.1:31851", "10.0.2.3"]
    /// "#;
    /// let cluster = Cluster::load_toml(toml, Role::Client).unwrap();
    /// assert_eq!(
    ///     cluster.peers(),
    ///     &vec![
    ///         Peer::new("10.0.2.1:31850".parse().unwrap(), 0, 0),
    ///         Peer::new("10.0.2.1:31851".parse().unwrap(), 0, 1),
    ///         Peer::new("10.0.2.3:31850".parse().unwrap(), 0, 0),
    ///     ]
    /// );
    /// ```
    ///
    /// Be aware that all peers will appear in the same order as you list them.
    /// No sorting will be performed.
    ///
    /// ## Auto mode
    ///
    /// Remember that you must specify `mode = auto` to use auto mode.
    /// Auto mode has more constraints but  specify the following fields:
    ///
    /// - `num`:      The number of peers.
    /// - `start-ip`: The IPv4 address of the first node. Each subsequent node
    ///               will add one on its last segment and **will error if it
    ///               reaches 255**.
    /// - `cpu`:      An integer or an integer array of CPU core binding offsets.
    /// - `port` (optional): The base RPC SM UDP port to use. Default to 31850.
    ///
    /// The following example shows a 5-peer cluster with 2 NUMA nodes on 3 nodes.
    ///
    /// ```rust
    /// # use nos::ctrl::*;
    /// let toml = r#"
    ///     [cluster]
    ///     mode = "auto"
    ///
    ///     num = 5
    ///     start-ip = "10.0.2.1"
    ///     cpu = [0, 18]
    ///     port = 31999
    /// "#;
    /// let cluster = Cluster::load_toml(toml, Role::Client).unwrap();
    /// assert_eq!(
    ///     cluster.peers(),
    ///     &vec![
    ///         Peer::new("10.0.2.1:31999".parse().unwrap(), 0, 0),
    ///         Peer::new("10.0.2.1:32000".parse().unwrap(), 18, 1),
    ///         Peer::new("10.0.2.2:31999".parse().unwrap(), 0, 0),
    ///         Peer::new("10.0.2.2:32000".parse().unwrap(), 18, 1),
    ///         Peer::new("10.0.2.3:31999".parse().unwrap(), 0, 0),
    ///     ]
    /// );
    /// ```
    ///
    /// The `default-port` and `default-cpu` fields will be ignored if you use
    /// auto mode.
    #[cold]
    pub fn load_toml(toml: &str, role: Role) -> Result<Self> {
        let toml = toml::from_str::<toml::Value>(toml)?;
        let conf = toml
            .get("cluster")
            .ok_or_else(|| anyhow::anyhow!("bad configuration: no cluster config found"))?
            .as_table()
            .ok_or_else(|| anyhow::anyhow!("bad configuration: cluster config must be a table"))?;

        // Determine configuration mode, default to manual
        let is_manual = match conf
            .get("mode")
            .map(|v| v.as_str())
            .unwrap_or(Some("manual"))
        {
            Some(s) if s.eq_ignore_ascii_case("manual") => true,
            Some(s) if s.eq_ignore_ascii_case("auto") => false,
            _ => {
                return Err(anyhow::anyhow!(
                    "bad configuration: mode must be either \"manual\" or \"auto\""
                ))
            }
        };

        let peers = if is_manual {
            let default_port = conf
                .get("default-port")
                .map(|x| {
                    x.as_integer()
                        .ok_or_else(|| {
                            anyhow::anyhow!("bad configuration: `default-port` must be an integer")
                        })
                        .map(|x| x as u16)
                })
                .unwrap_or(Ok(Self::DEFAULT_RPC_SM_PORT))?; // Default to 31850

            let default_cpu = conf
                .get("default-cpu")
                .map(|cpus| {
                    if let Some(cpu) = cpus.as_integer() {
                        Ok(vec![cpu as usize])
                    } else if let Some(cpus) = cpus.as_array() {
                        cpus
                            .iter()
                            .map(|x| {
                                x.as_integer()
                                    .ok_or_else(|| {
                                        anyhow::anyhow!(
                                            "bad configuration: default CPUs must be identified by integers"
                                        )
                                    })
                                    .map(|x| x as usize)
                            })
                            .collect::<Result<Vec<_>>>()
                    } else {
                        Err(anyhow::anyhow!(
                            "bad configuration: `default-cpu` should be either an integer or an array"
                        ))
                    }
                })
                .unwrap_or(Ok(vec![0]))?; // Default to 0

            let conf_peers = conf
                .get("peers")
                .ok_or_else(|| {
                    anyhow::anyhow!("bad configuration: `peers` must be specified (manual mode)")
                })?
                .as_array()
                .ok_or_else(|| anyhow::anyhow!("bad configuration: `peers` must be an array"))?;

            let mut peers = Vec::new();
            for peer in conf_peers {
                // Try parse as table
                if let Some(peer_table) = peer.as_table() {
                    let Some(ip) = peer_table.get("ip") else {
                        return Err(anyhow::anyhow!(
                            "bad configuration: peer table must contain `ip`"
                        ));
                    };
                    let Some(ip) = ip.as_str() else {
                        return Err(anyhow::anyhow!("bad configuration: `ip` must be a string"));
                    };

                    // Determine URI based on whether there is a colon
                    let uri = if ip.find(':').is_some() {
                        let uri = ip.parse::<SocketAddr>()?;
                        if let Some(port) = peer_table.get("port") {
                            if let Some(port) = port.as_integer() {
                                if port as u16 != uri.port() {
                                    return Err(anyhow::anyhow!(
                                        "bad configuration: port in URI and port field mismatch"
                                    ));
                                } else {
                                    log::warn!("specified port for peer {} in both IP and a separate field", uri);
                                }
                            } else {
                                return Err(anyhow::anyhow!(
                                    "bad configuration: `port` should be an integer"
                                ));
                            }
                        }
                        uri
                    } else {
                        let port = match peer_table.get("port") {
                            Some(port) => port.as_integer().ok_or_else(|| {
                                anyhow::anyhow!("bad configuration: `port` should be an integer")
                            })? as u16,
                            None => default_port,
                        };
                        SocketAddr::new(ip.parse()?, port)
                    };

                    // Determine CPUs
                    let cpus = if let Some(cpus) = peer_table.get("cpu") {
                        if let Some(cpu) = cpus.as_integer().map(|x| x as usize) {
                            vec![cpu]
                        } else if let Some(cpus) = cpus.as_array() {
                            cpus
                                .iter()
                                .map(|x| {
                                    x.as_integer()
                                        .ok_or_else(|| {
                                            anyhow::anyhow!(
                                                "bad configuration: CPUs must be identified by integers"
                                            )
                                        })
                                        .map(|x| x as usize)
                                })
                                .collect::<Result<Vec<_>>>()?
                        } else {
                            return Err(anyhow::anyhow!(
                                "bad configuration: `cpu` should be either an integer or an array"
                            ));
                        }
                    } else {
                        default_cpu.clone()
                    };

                    for (i, cpu) in cpus.into_iter().enumerate() {
                        let mut uri = uri;
                        uri.set_port(uri.port() + i as u16);

                        peers.push(Peer::new(uri, cpu, i));
                    }
                } else if let Some(peer_str) = peer.as_str() {
                    // Try parse as URI
                    if let Ok(uri) = peer_str.parse::<SocketAddr>() {
                        for (i, cpu) in default_cpu.iter().enumerate() {
                            peers.push(Peer::new(uri, *cpu, i));
                        }
                    } else {
                        // Try parse as IP
                        let ip = peer_str.parse::<IpAddr>()?;
                        for (i, cpu) in default_cpu.iter().enumerate() {
                            peers.push(Peer::new_with_ip_port(ip, default_port, *cpu, i));
                        }
                    }
                } else {
                    return Err(anyhow::anyhow!(
                        "bad configuration: peer must be either a string or a table"
                    ));
                }
            }
            peers
        } else {
            let num = conf
                .get("num")
                .ok_or_else(|| {
                    anyhow::anyhow!("bad configuration: `num` must be specified (auto mode)")
                })?
                .as_integer()
                .ok_or_else(|| anyhow::anyhow!("bad configuration: `num` must be an integer"))?
                as usize;
            let start_ip = conf
                .get("start-ip")
                .ok_or_else(|| {
                    anyhow::anyhow!("bad configuration: `start-ip` must be specified (auto mode)")
                })?
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("bad configuration: `start-ip` must be a string"))?
                .parse::<Ipv4Addr>()?;

            let cpus = conf
                .get("cpu")
                .map(|x| {
                    if let Some(cpu) = x.as_integer().map(|x| x as usize) {
                        Ok(vec![cpu])
                    } else if let Some(cpus) = x.as_array() {
                        cpus.iter()
                            .map(|x| {
                                x.as_integer()
                                    .ok_or_else(|| {
                                        anyhow::anyhow!(
                                            "bad configuration: CPUs must be identified by integers"
                                        )
                                    })
                                    .map(|x| x as usize)
                            })
                            .collect::<Result<Vec<_>>>()
                    } else {
                        Err(anyhow::anyhow!(
                            "bad configuration: `cpu` should be either an integer or an array"
                        ))
                    }
                })
                .unwrap_or(Ok(vec![1]))?; // Default to 1
            let port = conf
                .get("port")
                .map(|x| {
                    x.as_integer()
                        .ok_or_else(|| {
                            anyhow::anyhow!("bad configuration: `port` must be an integer")
                        })
                        .map(|x| x as u16)
                })
                .unwrap_or(Ok(Self::DEFAULT_RPC_SM_PORT))?; // Default to 31850

            if num > cpus.len() * 255 {
                return Err(anyhow::anyhow!("bad configuration: too many peers"));
            }

            let mut peers = Vec::with_capacity(num);
            for i in 0..num {
                let offset = i / cpus.len();
                let index = i % cpus.len();

                let mut ip = start_ip.octets();
                ip[3] += offset as u8;
                if ip[3] == 255 {
                    return Err(anyhow::anyhow!(
                        "bad configuration: cannot assign `.255` IPv4 address for peer {}",
                        i
                    ));
                }
                let ip = Ipv4Addr::from(ip);

                peers.push(Peer::new_with_ip_port(
                    ip.into(),
                    port + index as u16,
                    cpus[index],
                    index,
                ));
            }
            peers
        };

        Ok(Self::new(peers, role))
    }

    /// Load cluster configuration from a TOML file.
    #[cold]
    pub fn load_toml_file(path: impl AsRef<Path>, role: Role) -> Result<Self> {
        let mut file = File::open(path)?;
        let mut toml_str = String::new();
        file.read_to_string(&mut toml_str)?;

        Self::load_toml(&toml_str, role)
    }

    /// Get the IP addresses of all nodes in the cluster.
    #[inline]
    pub fn peers(&self) -> &Vec<Peer> {
        &self.peers
    }

    /// Get the rank of this node in the cluster.
    #[inline]
    pub fn rank(&self) -> usize {
        #[cold]
        #[inline(never)]
        fn client_failure() {
            panic!("cannot get self as a peer in the cluster because I am a client");
        }

        if self.id < 0 {
            client_failure();
        }
        self.id as usize
    }

    /// Get the peer representing this node.
    #[inline]
    pub fn me(&self) -> &Peer {
        &self.peers[self.rank()]
    }

    /// Get the number of participants in the cluster.
    #[inline]
    pub fn size(&self) -> usize {
        self.peers.len()
    }

    /// Determine whether a specific peer is alive.
    #[inline]
    pub fn is_peer_alive(&self, id: usize) -> bool {
        self.peers[id].is_alive()
    }
}

impl Index<usize> for Cluster {
    type Output = Peer;

    fn index(&self, index: usize) -> &Self::Output {
        &self.peers[index]
    }
}
