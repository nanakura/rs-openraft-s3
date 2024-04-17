#![feature(fn_traits, unboxed_closures)]
#[global_allocator]
static ALLOC: MiMalloc = MiMalloc;

use mimalloc::MiMalloc;
use ntex::web;
use ntex::web::middleware;
use ntex_cors::Cors;
use rs_s3_local::handler::{
    create_bucket, delete_bucket, delete_file, delete_file_longpath, download_file,
    download_file_longpath, get_bucket, head_bucket, head_object, head_object_longpath,
    init_chunk_or_combine_chunk, init_chunk_or_combine_chunk_longpath, list_bucket,
    upload_file_or_upload_chunk, upload_file_or_upload_chunk_longpath,
};
use rs_s3_local::middleware::CredentialsV4;

#[ntex::main]
async fn main() -> anyhow::Result<()> {
    // 初始化环境日志记录器
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));
    // 创建一个新的 HTTP 服务器实例。
    web::HttpServer::new(move || {
        web::App::new()
            // 应用 CORS 中间件，默认配置。
            .wrap(Cors::default())
            // 应用 AWS 签名版本 4 的认证中间件。
            .wrap(CredentialsV4)
            // 应用日志记录中间件，记录请求和响应。
            .wrap(middleware::Logger::default())
            // 定义路由和相应的处理函数。
            .route("/", web::get().to(list_bucket))
            .route("/{bucket}", web::get().to(get_bucket))
            .route("/{bucket}", web::head().to(head_bucket))
            .route("/{bucket}", web::put().to(create_bucket))
            .route("/{bucket}", web::delete().to(delete_bucket))
            .route("/{bucket}/", web::get().to(get_bucket))
            .route("/{bucket}/", web::head().to(head_bucket))
            .route("/{bucket}/", web::put().to(create_bucket))
            .route("/{bucket}/", web::delete().to(delete_bucket))
            .route(
                "/{bucket}/{object}",
                web::post().to(init_chunk_or_combine_chunk),
            )
            .route("/{bucket}/{object}", web::head().to(head_object))
            .route(
                "/{bucket}/{object}",
                web::put().to(upload_file_or_upload_chunk),
            )
            .route("/{bucket}/{object}", web::delete().to(delete_file))
            .route("/{bucket}/{object}", web::get().to(download_file))
            .route(
                "/{bucket}/{object}/{objectSuffix}*",
                web::post().to(init_chunk_or_combine_chunk_longpath),
            )
            .route(
                "/{bucket}/{object}/{objectSuffix}*",
                web::head().to(head_object_longpath),
            )
            .route(
                "/{bucket}/{object}/{objectSuffix}*",
                web::put().to(upload_file_or_upload_chunk_longpath),
            )
            .route(
                "/{bucket}/{object}/{objectSuffix}*",
                web::delete().to(delete_file_longpath),
            )
            .route(
                "/{bucket}/{object}/{objectSuffix}*",
                web::get().to(download_file_longpath),
            )
    })
    .bind(("0.0.0.0", 9000))?
    .run()
    .await?;
    Ok(())
}
