use std::collections::BTreeSet;
use std::sync::Arc;
use std::sync::Mutex;

use openraft::error::NetworkError;
use openraft::error::RPCError;
use openraft::error::RemoteError;
use openraft::error::Unreachable;
use openraft::RaftMetrics;
use openraft::TryAsRef;
use reqwest::Client;
use serde::de::DeserializeOwned;
use serde::Deserialize;
use serde::Serialize;

use crate::raft::typ;
use crate::raft::Node;
use crate::raft::NodeId;
use crate::raft::Request;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Empty {}

pub struct ExampleClient {
    /// The leader node to send request to.
    ///
    /// All traffic should be sent to the leader in a cluster.
    pub leader: Arc<Mutex<(NodeId, String)>>,

    pub inner: Client,
}

#[derive(Debug, Clone, Copy)]
enum Method {
    GET,
    POST,
    PUT,
    DELETE,
}

impl ExampleClient {
    /// Create a client with a leader node id and a node manager to get node address by node id.
    pub fn new(leader_id: NodeId, leader_addr: String) -> Self {
        Self {
            leader: Arc::new(Mutex::new((leader_id, leader_addr))),
            inner: Client::new(),
        }
    }

    // --- Application API
    // TODO Application API

    /// Submit a write request to the raft cluster.
    ///
    /// The request will be processed by raft protocol: it will be replicated to a quorum and then
    /// will be applied to state machine.
    ///
    /// The result of applying the request will be returned.
    pub async fn create_bucket(
        &self,
        req: &Request,
    ) -> Result<typ::ClientWriteResponse, typ::RPCError<typ::ClientWriteError>> {
        self.send_rpc_to_leader("api/write", Some(req), Method::PUT)
            .await
    }

    pub async fn delete_bucket(
        &self,
        req: &Request,
    ) -> Result<typ::ClientWriteResponse, typ::RPCError<typ::ClientWriteError>> {
        self.send_rpc_to_leader("api/write", Some(req), Method::DELETE)
            .await
    }

    // --- Cluster management API

    /// Initialize a cluster of only the node that receives this request.
    ///
    /// This is the first step to initialize a cluster.
    /// With a initialized cluster, new node can be added with [`write`].
    /// Then setup replication with [`add_learner`].
    /// Then make the new node a member with [`change_membership`].
    pub async fn init(&self) -> Result<(), typ::RPCError<typ::InitializeError>> {
        self.do_send_rpc_to_leader("cluster/init", Some(&Empty {}), Method::POST)
            .await
    }

    /// Add a node as learner.
    ///
    /// The node to add has to exist, i.e., being added with `write(ExampleRequest::AddNode{})`
    pub async fn add_learner(
        &self,
        req: (NodeId, String, String),
    ) -> Result<typ::ClientWriteResponse, typ::RPCError<typ::ClientWriteError>> {
        self.send_rpc_to_leader("cluster/add-learner", Some(&req), Method::POST)
            .await
    }

    /// Change membership to the specified set of nodes.
    ///
    /// All nodes in `req` have to be already added as learner with [`add_learner`],
    /// or an error [`LearnerNotFound`] will be returned.
    pub async fn change_membership(
        &self,
        req: &BTreeSet<NodeId>,
    ) -> Result<typ::ClientWriteResponse, typ::RPCError<typ::ClientWriteError>> {
        self.send_rpc_to_leader("cluster/change-membership", Some(req), Method::POST)
            .await
    }

    /// Get the metrics about the cluster.
    ///
    /// Metrics contains various information about the cluster, such as current leader,
    /// membership config, replication status etc.
    /// See [`RaftMetrics`].
    pub async fn metrics(&self) -> Result<RaftMetrics<NodeId, Node>, typ::RPCError> {
        self.do_send_rpc_to_leader("cluster/metrics", None::<&()>, Method::POST)
            .await
    }

    // --- Internal methods

    /// Send RPC to specified node.
    ///
    /// It sends out a POST request if `req` is Some. Otherwise a GET request.
    /// The remote endpoint must respond a reply in form of `Result<T, E>`.
    /// An `Err` happened on remote will be wrapped in an [`RPCError::RemoteError`].
    async fn do_send_rpc_to_leader<Req, Resp, Err>(
        &self,
        uri: &str,
        req: Option<&Req>,
        method: Method,
    ) -> Result<Resp, RPCError<NodeId, Node, Err>>
    where
        Req: Serialize + 'static,
        Resp: Serialize + DeserializeOwned,
        Err: std::error::Error + Serialize + DeserializeOwned,
    {
        let (leader_id, url) = {
            let t = self.leader.lock().unwrap();
            let target_addr = &t.1;
            (t.0, format!("http://{}/{}", target_addr, uri))
        };
        let resp = if let Some(r) = req {
            println!(
                ">>> client send request to {}: {}",
                url,
                serde_json::to_string_pretty(&r).unwrap()
            );
            match method {
                Method::GET => self.inner.get(url.clone()),
                Method::POST => self.inner.post(url.clone()).json(r),
                Method::PUT => self.inner.put(url.clone()).json(r),
                Method::DELETE => self.inner.delete(url.clone()),
            }
        } else {
            println!(">>> client send request to {}", url,);
            self.inner.get(url.clone())
        }
        .send()
        .await
        .map_err(|e| {
            if e.is_connect() {
                // `Unreachable` informs the caller to backoff for a short while to avoid error log flush.
                return RPCError::Unreachable(Unreachable::new(&e));
            }
            RPCError::Network(NetworkError::new(&e))
        })?;

        let res: Result<Resp, Err> = resp
            .json()
            .await
            .map_err(|e| RPCError::Network(NetworkError::new(&e)))?;
        println!(
            "<<< client recv reply from {}: {}",
            url,
            serde_json::to_string_pretty(&res).unwrap()
        );

        res.map_err(|e| RPCError::RemoteError(RemoteError::new(leader_id, e)))
    }

    /// Try the best to send a request to the leader.
    ///
    /// If the target node is not a leader, a `ForwardToLeader` error will be
    /// returned and this client will retry at most 3 times to contact the updated leader.
    async fn send_rpc_to_leader<Req, Resp, Err>(
        &self,
        uri: &str,
        req: Option<&Req>,
        method: Method,
    ) -> Result<Resp, typ::RPCError<Err>>
    where
        Req: Serialize + 'static,
        Resp: Serialize + DeserializeOwned,
        Err: std::error::Error
            + Serialize
            + DeserializeOwned
            + TryAsRef<typ::ForwardToLeader>
            + Clone,
    {
        // Retry at most 3 times to find a valid leader.
        let mut n_retry = 3;

        loop {
            let res: Result<Resp, typ::RPCError<Err>> =
                self.do_send_rpc_to_leader(uri, req, method).await;

            let rpc_err = match res {
                Ok(x) => return Ok(x),
                Err(rpc_err) => rpc_err,
            };

            if let RPCError::RemoteError(remote_err) = &rpc_err {
                let raft_err: &typ::RaftError<_> = &remote_err.source;

                if let Some(typ::ForwardToLeader {
                    leader_id: Some(leader_id),
                    leader_node: Some(leader_node),
                    ..
                }) = raft_err.forward_to_leader()
                {
                    // Update target to the new leader.
                    {
                        let mut t = self.leader.lock().unwrap();
                        let api_addr = leader_node.api_addr.clone();
                        *t = (*leader_id, api_addr);
                    }

                    n_retry -= 1;
                    if n_retry > 0 {
                        continue;
                    }
                }
            }

            return Err(rpc_err);
        }
    }
}
