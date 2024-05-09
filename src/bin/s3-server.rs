#![feature(fn_traits, unboxed_closures)]
#[global_allocator]
static ALLOC: MiMalloc = MiMalloc;

use clap::Parser;
use mimalloc::MiMalloc;
use rs_s3_local::start_example_raft_node;

#[derive(Parser, Clone, Debug)]
#[clap(author, version, about, long_about = None)]
pub struct Opt {
    #[clap(long, default_value_t = 1)]
    pub id: u64,

    #[clap(long, default_value_t = String::from("127.0.0.1:9000"))]
    pub http_addr: String,

    #[clap(long, default_value_t = String::from("127.0.0.1:32001"))]
    pub rpc_addr: String,

    #[clap(long)]
    pub leader_http_addr: Option<String>,

    #[clap(long, default_value_t = String::from("minioadmin"))]
    pub access_key: String,
    #[clap(long, default_value_t = String::from("minioadmin"))]
    pub secret_key: String,
}

#[ntex::main]
async fn main() -> anyhow::Result<()> {
    // 初始化环境日志记录器
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));
    // 创建一个新的 HTTP 服务器实例。
    // Parse the parameters passed by arguments.
    let options = Opt::parse();

    start_example_raft_node(
        options.id,
        format!("{}-db", options.id),
        options.http_addr,
        options.rpc_addr,
        options.access_key,
        options.secret_key,
        options.leader_http_addr,
    )
    .await?;
    Ok(())
}
