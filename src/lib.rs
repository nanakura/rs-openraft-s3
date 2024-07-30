use crate::err::AppError;
use crate::middleware::CredentialsV4;
use crate::raft::app::App;
use crate::raft::network::raft::Raft;
use crate::raft::network::Network;
use crate::raft::store::new_storage;
use crate::raft::NodeId;
use log::info;
use ntex::web;
use ntex::web::HttpResponse;
use ntex_cors::Cors;
use openraft::Config;
use raft::app::NodeDesc;
use std::collections::BTreeSet;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::Mutex;

pub mod api;
mod err;
pub mod fs;
pub mod management;
pub mod middleware;
pub mod model;
mod raft;
mod stream;
pub mod util;
pub type HandlerResponse = Result<HttpResponse, AppError>;

pub async fn start_example_raft_node<P>(
    node_id: NodeId,
    dir: P,
    http_addr: String,
    rpc_addr: String,
    fs_root: String,
    access_key: String,
    secret_key: String,
    leader_http_addr: Option<String>,
) -> std::io::Result<()>
where
    P: AsRef<Path>,
{
    // Create a configuration for the raft instance.
    let config = Config {
        heartbeat_interval: 250,
        election_timeout_min: 299,
        ..Default::default()
    };

    let config = Arc::new(config.validate().unwrap());

    let (log_store, state_machine_store) = new_storage(&dir).await;

    let kvs = state_machine_store.data.kvs.clone();

    // Create the network layer that will connect and communicate the raft instances and
    // will be used in conjunction with the store created above.
    let network = Network {};

    // Create a local raft instance.
    let raft = openraft::Raft::new(
        node_id,
        config.clone(),
        network,
        log_store,
        state_machine_store,
    )
    .await
    .unwrap();

    let mut set = BTreeSet::new();
    let node_desc = NodeDesc{
        node_id,
        api_addr: http_addr.clone(),
        rpc_addr: rpc_addr.clone(),
    };
    set.insert(node_desc);
    let mut set2 = BTreeSet::new();
    set2.insert(node_id);
    let app = App {
        id: node_id,
        api_addr: http_addr.clone(),
        rpc_addr: rpc_addr.clone(),
        raft,
        key_values: kvs,
        config,
        nodes: Arc::new(Mutex::new(set2)),
        node_descs: Arc::new(Mutex::new(set)),
    };

    let addr: SocketAddr = rpc_addr.parse().unwrap();
    let raft_node = Raft::new(Arc::new(app.clone()));
    tokio::spawn(async move {
        let addr = volo::net::Address::from(addr);

        info!("websocket server");
        volo_gen::rpc::raft::RaftServiceServer::new(raft_node)
            .run(addr)
            .await
            .unwrap();
    });

    // Create an application that will store all the instances created above, this will
    // be later used on the actix-web services.
    api::DATA_DIR
        .get_or_init(|| async {
            PathBuf::from(fs_root.clone())
                .join("data")
                .to_string_lossy()
                .to_string()
        })
        .await;
    let server_start = web::HttpServer::new(move || {
        info!("web server");
        let app = app.clone();
        web::App::new()
            .state(app)
            .wrap(ntex::web::middleware::Logger::default())
            .wrap(Cors::default())
            // 应用 AWS 签名版本 4 的认证中间件。
            .wrap(CredentialsV4::new(access_key.clone(), secret_key.clone()))
            .configure(management::rest)
            .configure(api::rest)
    })
    .bind(&http_addr)
    .unwrap()
    .run();

    let client = reqwest::Client::new();
    if let Some(addr) = leader_http_addr {
        let response = client
            .post(format!("http://{}/cluster/add-learner", addr))
            .body(format!(
                "[{}, \"{}\", \"{}\"]",
                node_id, http_addr, rpc_addr
            ))
            .send()
            .await
            .unwrap();
        info!("cluster add learner resp status {}", response.status());
        let response = client
            .post(format!("http://{}/cluster/change-membership", addr))
            .send()
            .await
            .unwrap();
        info!(
            "cluster change membership resp status {}",
            response.status()
        );
    } else {
        let response = client
            .post(format!("http://{}/cluster/init", http_addr))
            .body("{}")
            .send()
            .await
            .unwrap();
        info!("cluster init resp status {}", response.status());
    }
    server_start.await?;
    Ok(())
}
