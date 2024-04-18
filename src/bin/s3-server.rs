#![feature(fn_traits, unboxed_closures)]
#[global_allocator]
static ALLOC: MiMalloc = MiMalloc;

use clap::Parser;
use mimalloc::MiMalloc;
use rs_s3_local::start_example_raft_node;

#[derive(Parser, Clone, Debug)]
#[clap(author, version, about, long_about = None)]
pub struct Opt {
    #[clap(long)]
    pub id: u64,

    #[clap(long)]
    pub http_addr: String,

    #[clap(long)]
    pub rpc_addr: String,
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
    ).await?;
    Ok(())
}
