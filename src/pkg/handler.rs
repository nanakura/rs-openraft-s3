use std::path::PathBuf;
use std::time::SystemTime;
use anyhow::Context;
use ntex::web;
use ntex::web::HttpResponse;
use serde::{Serialize};
use crate::pkg::err::AppError;
use serde_xml_rs::to_string;
use crate::pkg::err::AppError::BadRequest;
use futures::{StreamExt};
use ntex::util::{BytesMut, Stream};
use crate::pkg::fs;
use crate::pkg::fs::{Metadata, multi_decompressed_reader, ReadStream, save_file, sum_15bit_sha256};

#[derive(Serialize)]
struct Person {
    name: String,
    age: u32,
}

type HandlerResponse = Result<HttpResponse, AppError>;
pub(crate) async fn list_bucket() -> HandlerResponse {
    let person = Person {
        name: "John Doe".to_owned(),
        age: 30,
    };
    let xml = to_string(&person).context("")?;
    let res = HttpResponse::Ok().content_type("application/xml")
        .body(xml);
    Ok(res)
}

pub(crate) async fn get_bucket() -> HandlerResponse {
    println!("get bucket");
    Ok(HttpResponse::Ok().content_type("application/xml").finish())
}
pub(crate) async fn head_bucket() -> HandlerResponse {
    Ok(HttpResponse::Ok().content_type("application/xml").finish())
}
pub(crate) async fn create_bucket() -> HandlerResponse {
    Ok(HttpResponse::Ok().content_type("application/xml").finish())
}
pub(crate) async fn delete_bucket() -> HandlerResponse {
    Ok(HttpResponse::Ok().content_type("application/xml").finish())
}

pub(crate) async fn init_chunk_or_combine_chunk() -> HandlerResponse {
    Ok(HttpResponse::Ok().content_type("application/xml").finish())
}
pub(crate) async fn head_object() -> HandlerResponse {
    Ok(HttpResponse::Ok().content_type("application/xml").finish())
}
pub(crate) async fn upload_file_or_upload_chunk() -> HandlerResponse {
    println!("short path");
    Ok(HttpResponse::Ok().content_type("application/xml").finish())
}
pub(crate) async fn delete_file() -> HandlerResponse {
    Ok(HttpResponse::Ok().content_type("application/xml").finish())
}
async fn upload_file(mut body: web::types::Payload) -> HandlerResponse {
    let data_dir = "path/to/data/dir";
    let basic_path_suffix = "basic/path/suffix";
    let bucket_name = "example_bucket";
    let object_key = "example_object_key";
    let file_path = PathBuf::from(data_dir)
        .join(basic_path_suffix)
        .join(bucket_name)
        .join(object_key);
    let mut metainfo_file_path = file_path.clone().to_string_lossy().to_string();
    metainfo_file_path.push_str(".meta");
    let file_name = file_path.file_name().context("解析文件名失败")?.to_string_lossy().to_string();
    let (file_size, _) = body.size_hint();
    let mut bytes = BytesMut::new();
    while let Some(item) = body.next().await {
        let item = item.context("")?;
        bytes.extend_from_slice(&item);
    }
    let chunks = bytes.chunks(8 << 20);
    let mut hashcodes: Vec<String> = Vec::new();
    for chunk in chunks {
       let sha = sum_15bit_sha256(chunk);
        hashcodes.push(sha.clone());
        if !fs::is_path_exist(&sha) {
            let compressed_chunk = fs::compress_chunk(chunk)?;
            save_file(&sha, &compressed_chunk)?;
        }
    }
    let metainfo = Metadata{
        name: file_name,
        size: file_size,
        file_type: "".to_string(),
        time: SystemTime::now(),
        chunks: hashcodes
    };
    fs::save_metadata(&metainfo_file_path, &metainfo)?;
    Ok(web::HttpResponse::Ok().content_type("application/xml").finish())
}

pub(crate) async fn get_suffix(req: web::HttpRequest) -> HandlerResponse {
    println!("get suffix");
    Ok(web::HttpResponse::Ok().content_type("application/xml").finish())
}
pub(crate) async fn head_object_longpath(req: web::HttpRequest) -> HandlerResponse {
    Ok(web::HttpResponse::Ok().content_type("application/xml").finish())
}
pub(crate) async fn upload_file_or_upload_chunk_longpath(req: web::HttpRequest) -> HandlerResponse {
    println!("longpath");
    Ok(web::HttpResponse::Ok().content_type("application/xml").finish())
}
pub(crate) async fn delete_file_longpath(req: web::HttpRequest) -> HandlerResponse {
    Ok(web::HttpResponse::Ok().content_type("application/xml").finish())
}
pub(crate) async fn get_suffix_longpath(req: web::HttpRequest) -> HandlerResponse {
    println!("longpath");
    Ok(web::HttpResponse::Ok().content_type("application/xml").finish())
}
pub(crate) async fn download_file(req: web::HttpRequest) -> HandlerResponse {
    let data_dir = "path/to/data/dir";
    let basic_path_suffix = "basic/path/suffix";
    let bucket_name = "example_bucket";
    let object_key = "example_object_key";
    let file_path = PathBuf::from(data_dir)
        .join(basic_path_suffix)
        .join(bucket_name)
        .join(object_key);
    let mut metainfo_file_path = file_path.clone().to_string_lossy().to_string();
    metainfo_file_path.push_str(".meta");
    let meta_info = fs::load_metadata(&metainfo_file_path)?;
    let filename: String = req.match_info().query("filename")
        .parse()
        .map_err(|_|BadRequest)?;
    let readers = multi_decompressed_reader(&meta_info.chunks[..]).await?;
    let content_disposition = format!("attachment; filename=\"{}\"", filename);
    let body = ReadStream::new(readers);
    Ok(web::HttpResponse::Ok()
        .header("Content-Type", "application/octet-stream")
        .header("Content-Length", meta_info.size)
        .header("Content-Disposition", content_disposition)
        .streaming(body))
}