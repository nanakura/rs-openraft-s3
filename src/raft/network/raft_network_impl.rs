use log::debug;
use std::fmt::Display;
use std::net::SocketAddr;

use crate::raft::Node;
use crate::raft::NodeId;
use crate::raft::TypeConfig;
use openraft::error::InstallSnapshotError;
use openraft::error::NetworkError;
use openraft::error::RPCError;
use openraft::error::RaftError;
use openraft::network::RPCOption;
use openraft::network::RaftNetwork;
use openraft::network::RaftNetworkFactory;
use openraft::raft::AppendEntriesRequest;
use openraft::raft::AppendEntriesResponse;
use openraft::raft::InstallSnapshotRequest;
use openraft::raft::InstallSnapshotResponse;
use openraft::raft::VoteRequest;
use openraft::raft::VoteResponse;
use serde::de::DeserializeOwned;
use volo_gen::rpc::raft::RaftRequest;
use volo_thrift::ClientError;

pub struct Network {}

// NOTE: This could be implemented also on `Arc<ExampleNetwork>`, but since it's empty, implemented
// directly.
impl RaftNetworkFactory<TypeConfig> for Network {
    type Network = NetworkConnection;

    async fn new_client(&mut self, target: NodeId, node: &Node) -> Self::Network {
        let addr: SocketAddr = node.rpc_addr.parse().unwrap();

        let client = volo_gen::rpc::raft::RaftServiceClientBuilder::new("raft-service")
            .address(addr)
            .build();

        NetworkConnection { client, target }
    }
}

pub struct NetworkConnection {
    client: volo_gen::rpc::raft::RaftServiceClient,
    target: NodeId,
}
impl NetworkConnection {
    async fn c<E: std::error::Error + DeserializeOwned>(
        &mut self,
    ) -> Result<&volo_gen::rpc::raft::RaftServiceClient, RPCError<NodeId, Node, E>> {
        Ok(&self.client)
    }
}

#[derive(Debug)]
struct ErrWrap(Box<dyn std::error::Error>);

impl Display for ErrWrap {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl std::error::Error for ErrWrap {}

fn to_error<E: std::error::Error + 'static + Clone>(
    e: ClientError,
    _target: NodeId,
) -> RPCError<NodeId, Node, E> {
    RPCError::Network(NetworkError::new(&e))
}

// With nightly-2023-12-20, and `err(Debug)` in the instrument macro, this gives the following lint
// warning. Without `err(Debug)` it is OK. Suppress it with `#[allow(clippy::blocks_in_conditions)]`
//
// warning: in a `match` scrutinee, avoid complex blocks or closures with blocks; instead, move the
// block or closure higher and bind it with a `let`
//
//    --> src/network/raft_network_impl.rs:99:91
//     |
// 99  |       ) -> Result<AppendEntriesResponse<NodeId>, RPCError<NodeId, Node, RaftError<NodeId>>>
// {
//     |  ___________________________________________________________________________________________^
// 100 | |         tracing::debug!(req = debug(&req), "append_entries");
// 101 | |
// 102 | |         let c = self.c().await?;
// ...   |
// 108 | |         raft.append(req).await.map_err(|e| to_error(e, self.target))
// 109 | |     }
//     | |_____^
//     |
//     = help: for further information visit https://rust-lang.github.io/rust-clippy/master/index.html#blocks_in_conditions
//     = note: `#[warn(clippy::blocks_in_conditions)]` on by default
#[allow(clippy::blocks_in_conditions)]
impl RaftNetwork<TypeConfig> for NetworkConnection {
    async fn append_entries(
        &mut self,
        req: AppendEntriesRequest<TypeConfig>,
        _option: RPCOption,
    ) -> Result<AppendEntriesResponse<NodeId>, RPCError<NodeId, Node, RaftError<NodeId>>> {
        debug!("append_entries");

        let c = self.c().await?;
        debug!("got connection");

        let req = postcard::to_stdvec(&req).unwrap();
        let req = String::from_utf8(req).unwrap();
        let x = c
            .append(RaftRequest {
                data: req.parse().unwrap(),
            })
            .await
            .map_err(|e| to_error(e, self.target))?;

        let resp = postcard::from_bytes(x.data.as_bytes()).unwrap();
        Ok(resp)
    }

    async fn install_snapshot(
        &mut self,
        req: InstallSnapshotRequest<TypeConfig>,
        _option: RPCOption,
    ) -> Result<
        InstallSnapshotResponse<NodeId>,
        RPCError<NodeId, Node, RaftError<NodeId, InstallSnapshotError>>,
    > {
        debug!("install_snapshot");
        let req = postcard::to_stdvec(&req).unwrap();
        let req = String::from_utf8(req).unwrap();
        let x = self
            .c()
            .await?
            .snapshot(RaftRequest {
                data: req.parse().unwrap(),
            })
            .await
            .map_err(|e| to_error(e, self.target))?;
        let resp = postcard::from_bytes(x.data.as_bytes()).unwrap();
        Ok(resp)
    }

    async fn vote(
        &mut self,
        req: VoteRequest<NodeId>,
        _option: RPCOption,
    ) -> Result<VoteResponse<NodeId>, RPCError<NodeId, Node, RaftError<NodeId>>> {
        debug!("vote");
        let req = postcard::to_stdvec(&req).unwrap();
        let req = String::from_utf8(req).unwrap();
        let x = self
            .c()
            .await?
            .vote(RaftRequest {
                data: req.parse().unwrap(),
            })
            .await
            .map_err(|e| to_error(e, self.target))?;
        let resp = postcard::from_bytes(x.data.as_bytes()).unwrap();
        Ok(resp)
    }
}
