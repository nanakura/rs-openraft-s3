use std::fs::File;
use std::io;
use std::io::{Chain, Read, Write};
use anyhow::Context;
use ntex::web;
use ntex::web::HttpResponse;
use serde::{Deserialize, Serialize};
use crate::pkg::err::AppError;
use serde_xml_rs::to_string;
use crate::pkg::err::AppError::BadRequest;
use futures::{future::ok, stream::once, StreamExt, TryStreamExt};
use ntex::util::{Bytes, BytesMut};
use ntex_files::NamedFile;
use ntex_multipart::Multipart;
use tokio::io::{BufReader, BufStream};
use zstd::decode_all;
use crate::pkg::fs::{multi_decompressed_reader, ReadStream};

#[derive(Serialize)]
struct Person {
    name: String,
    age: u32,
}

type HandlerResponse = Result<HttpResponse, AppError>;
#[web::get("/")]
async fn get() -> HandlerResponse {
    let person = Person {
        name: "John Doe".to_owned(),
        age: 30,
    };
    let xml = to_string(&person).context("")?;
    let res = HttpResponse::Ok().content_type("application/xml")
        .body(xml);
    Ok(res)
}

#[web::post("/upload")]
async fn upload_file(mut payload: Multipart) -> HandlerResponse {
    // iterate over multipart stream
    while let Ok(Some(mut field)) = payload.try_next().await {
        // let content_type = field.content_disposition().unwrap();
        // let filename = content_type.get_filename().unwrap();
        let filename = "somename";
        let filepath = format!("./tmp/{}", filename);
        // File::create is blocking operation, use threadpool
        let mut f = web::block(|| std::fs::File::create(filepath))
            .await
            .unwrap();
        // Field in turn is stream of *Bytes* object
        while let Some(chunk) = field.next().await {
            let data = chunk.unwrap();
            // filesystem operations are blocking, we have to use threadpool
            f = web::block(move || f.write_all(&data).map(|_| f)).await
                .context("文件保存失败")?;
        }
    }

    Ok(web::HttpResponse::Ok().finish())
}

#[web::get("/download/{filename}*")]
async fn download_file(req: web::HttpRequest) -> HandlerResponse {
    let filename: String = req.match_info().query("filename")
        .parse()
        .map_err(|_|BadRequest)?;
    let readers = multi_decompressed_reader(&[String::from("")]).await?;
    let content_disposition = format!("attachment; filename=\"{}\"", filename);
    let body = ReadStream::new(readers);
    Ok(web::HttpResponse::Ok()
        .header("Content-Type", "application/octet-stream")
        .header("Content-Disposition", content_disposition)
        .streaming(body))
}