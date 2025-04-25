use crate::ctrl::Cluster;
use rrppcc::{type_alias::*, *};

/// Resource pool that allows `ObjStore::rpc_handler` to send RPCs.
///
/// It contains:
/// - RPC sessions to remote nodes;
/// - message buffers.
pub struct RpcRes {
    /// RPC sessions to remote nodes.
    sessions: Vec<SessId>,
}

impl RpcRes {
    /// Create a new resource pool on the given [`Rpc`] instance.
    pub async fn new(rpc: &Rpc, cluster: &Cluster, rpc_id: RpcId) -> Self {
        let n = cluster.size();
        let rank = cluster.rank();

        let mut sessions = Vec::with_capacity(n);
        for i in 0..n {
            sessions.push(if i == rank {
                0
            } else {
                let uri = cluster[i].uri();
                let sess = rpc.create_session(uri, rpc_id);
                sess.connect().await;
                assert!(
                    sess.is_connected(),
                    "failed to connect to {} Rpc #{}",
                    uri,
                    rpc_id
                );
                sess.id()
            });
        }
        Self { sessions }
    }

    /// Get the session to the specified remote node.
    pub fn session_to<'a>(&'a self, rpc: &'a Rpc, rank: usize) -> Session<'a> {
        rpc.get_session(self.sessions[rank]).unwrap()
    }
}
