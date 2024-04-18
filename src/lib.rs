use crate::err::AppError;
use crate::middleware::CredentialsV4;
use crate::raft::app::App;
use crate::raft::network::Network;
use crate::raft::store::new_storage;
use crate::raft::{network, NodeId};
use log::info;
use ntex::web;
use ntex::web::HttpResponse;
use ntex_cors::Cors;
use openraft::Config;
use std::path::Path;
use std::sync::Arc;
use tokio::net::TcpListener;

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

    let app = App {
        id: node_id,
        api_addr: http_addr.clone(),
        rpc_addr: rpc_addr.clone(),
        raft,
        key_values: kvs,
        config,
    };

    let echo_service = Arc::new(network::raft::Raft::new(Arc::new(app.clone())));

    let server = toy_rpc::Server::builder().register(echo_service).build();

    let listener = TcpListener::bind(rpc_addr).await.unwrap();
    tokio::spawn(async move {
        server.accept_websocket(listener).await.unwrap();
        info!("websocket server");
    });

    // Create an application that will store all the instances created above, this will
    // be later used on the actix-web services.
    let _ = web::HttpServer::new(move || {
        info!("web server");
        let app = app.clone();
        web::App::new()
            .state(app)
            .wrap(ntex::web::middleware::Logger::default())
            .configure(management::rest)
            .wrap(Cors::default())
            // 应用 AWS 签名版本 4 的认证中间件。
            .wrap(CredentialsV4)
            .configure(api::rest)
    })
    .bind(http_addr)
    .unwrap()
    .run()
    .await;
    Ok(())
}
