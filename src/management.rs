use anyhow::Context;
use log::info;
use ntex::util::BytesMut;
use ntex::web;
use ntex::web::types::Payload;
use ntex::web::HttpResponse;
use std::collections::BTreeMap;

use crate::raft::app::NodeDesc;
use crate::HandlerResponse;
use openraft::error::Infallible;
use openraft::RaftMetrics;

use crate::raft::app::App;
use crate::raft::Node;
use crate::raft::NodeId;

// --- Cluster management

pub fn rest(cfg: &mut web::ServiceConfig) {
    cfg.route(
        "/health",
        web::get().to(|| async { web::HttpResponse::Ok().body("ok") }),
    )
    .route("/cluster/add-learner", web::post().to(add_learner))
    .route(
        "/cluster/change-membership",
        web::post().to(change_membership),
    )
    .route("/cluster/init", web::post().to(init))
    .route("/cluster/metrics", web::get().to(metrics));
}

/// Add a node as **Learner**.
///
/// A Learner receives log replication from the leader but does not vote.
/// This should be done before adding a node as a member into the cluster
/// (by calling `change-membership`)
pub async fn add_learner(mut payload: Payload, state: web::types::State<App>) -> HandlerResponse {
    let mut bytes = BytesMut::new();
    while let Some(item) = ntex::util::stream_recv(&mut payload).await {
        bytes.extend_from_slice(&item.unwrap());
    }
    let (node_id, api_addr, rpc_addr): (NodeId, String, String) =
        serde_json::from_slice(&bytes).context("deserialize json failed")?;
    let node = Node { rpc_addr: rpc_addr.clone(), api_addr: api_addr.clone() };

    let node_desc = NodeDesc{
        node_id,
        api_addr,
        rpc_addr,
    };
    state.nodes.lock().await.insert(node_id);
    state.node_descs.lock().await.insert(node_desc);
    let res = state.raft.add_learner(node_id, node, true).await;
    Ok(HttpResponse::Ok().json(&res))
}

/// Changes specified learners to members, or remove members.
pub async fn change_membership(state: web::types::State<App>) -> HandlerResponse {
    let x = state.nodes.lock().await;
    let body = (*x).clone();
    let res = state.raft.change_membership(body, false).await;
    Ok(HttpResponse::Ok().json(&res))
}

/// Initialize a single-node cluster.
pub async fn init(state: web::types::State<App>) -> HandlerResponse {
    info!("start init");
    let mut nodes = BTreeMap::new();
    let node = Node {
        api_addr: state.api_addr.clone(),
        rpc_addr: state.rpc_addr.clone(),
    };

    nodes.insert(state.id, node);
    let res = state.raft.initialize(nodes).await;

    info!("get res: {:?}", res);
    Ok(HttpResponse::Ok().json(&res))
}

/// Get the latest metrics of the cluster
pub async fn metrics(state: web::types::State<App>) -> HandlerResponse {
    let metrics = state.raft.metrics().borrow().clone();

    let res: Result<RaftMetrics<NodeId, Node>, Infallible> = Ok(metrics);
    Ok(HttpResponse::Ok().json(&res))
}
