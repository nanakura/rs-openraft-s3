#[global_allocator]
static ALLOC: snmalloc_rs::SnMalloc = snmalloc_rs::SnMalloc;
mod pkg;

use ntex::web;
use ntex_cors::Cors;
use ntex::web::middleware;
use crate::pkg::handler::get;

#[ntex::main]
async fn main() ->anyhow::Result<()>{
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));
    web::HttpServer::new(move || {
        let cors = Cors::default();
        let app = web::App::new()
            .wrap(cors)
            .wrap(middleware::Logger::default())
            .service(
                web::resource("/s3")
                    .route(web::get().to(get))
            );

        app
    })
        .bind(("0.0.0.0", 8080))?
        .run()
        .await?;
    Ok(())
}
