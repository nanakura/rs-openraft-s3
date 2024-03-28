use std::fmt::Pointer;
use crate::pkg::err::AppError;
use crate::pkg::err::AppError::BadRequest;
use crate::pkg::fs;
use crate::pkg::fs::{
    multi_decompressed_reader, save_file, sum_15bit_sha256, Metadata, ReadStream,
};
use crate::pkg::model::{
    Bucket, BucketWrapper, Content, HeadNotFoundResp, ListBucketResp, ListBucketResult, Owner,
};
use crate::pkg::util::date::date_format_to_second;
use crate::pkg::util::file::get_file_type;
use anyhow::Context;
use chrono::Utc;
use futures::future::ok;
use futures::stream::once;
use futures::StreamExt;
use mime_guess::MimeGuess;
use ntex::util::{Bytes, BytesMut, Stream};
use ntex::web;
use ntex::web::types::Query;
use ntex::web::{HttpResponse, Responder};
use quick_xml::se::to_string;
use serde::Deserialize;
use std::fs::read_dir;
use std::path::{Path, PathBuf};

static DATA_DIR: &str = "data";
static BASIC_PATH_SUFFIX: &str = "buckets";

type HandlerResponse = Result<HttpResponse, AppError>;
pub(crate) async fn list_bucket() -> HandlerResponse {
    let dir_path = PathBuf::from(DATA_DIR).join(BASIC_PATH_SUFFIX);
    let dir_path = dir_path.as_path();
    if dir_path.is_dir() {
        let mut res = Vec::new();
        if let Ok(dir) = read_dir(dir_path) {
            for entry in dir {
                if let Ok(entry) = entry {
                    if entry.file_type().unwrap().is_dir() {
                        let metadata = entry.metadata().context("转换失败")?;
                        let mod_time = metadata.modified().context("转换失败")?;
                        let bucket = Bucket {
                            name: entry.file_name().to_string_lossy().to_string(),
                            creation_date: date_format_to_second(mod_time),
                        };
                        res.push(bucket);
                    }
                }
            }
        }
        let mut buckets = Vec::new();
        for bucket in res {
            let bucket_wrapper = BucketWrapper { bucket };
            buckets.push(bucket_wrapper);
        }
        let list_res = ListBucketResp {
            id: "20230529".to_string(),
            owner: Owner {
                display_name: "minioadmin".to_string(),
            },
            buckets,
        };
        let xml = to_string(&list_res).context("序列化失败")?;
        Ok(HttpResponse::Ok().content_type("application/xml").body(xml))
    } else {
        std::fs::create_dir_all(dir_path).context("创建文件夹失败")?;
        let buckets = Vec::new();
        let list_res = ListBucketResp {
            id: "20230529".to_string(),
            owner: Owner {
                display_name: "minioadmin".to_string(),
            },
            buckets,
        };
        let xml = to_string(&list_res).context("序列化失败")?;
        Ok(HttpResponse::Ok().content_type("application/xml").body(xml))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use serde::Serialize;
    #[test]
    fn test1() {
        let mut buckets = Vec::new();
        buckets.push(BucketWrapper {
            bucket: Bucket {
                name: "xx".to_string(),
                creation_date: "111".to_string(),
            },
        });
        let list_res = ListBucketResp {
            id: "20230529".to_string(),
            owner: Owner {
                display_name: "minioadmin".to_string(),
            },
            buckets,
        };
        let xml = to_string(&list_res);
        assert!(xml.is_ok(), "序列化错误");
    }

    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    struct Person {
        #[serde(rename = "FullName")]
        full_name: String,
        age: u32,
        #[serde(rename = "Address")]
        addresses: Vec<Address>,
    }

    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    struct Address {
        street: String,
        city: String,
        state: String,
    }

    #[test]
    fn test2() {
        let person = Person {
            full_name: "John Doe".to_string(),
            age: 30,
            addresses: vec![
                Address {
                    street: "123 Main St".to_string(),
                    city: "New York".to_string(),
                    state: "NY".to_string(),
                },
                Address {
                    street: "456 Elm St".to_string(),
                    city: "Los Angeles".to_string(),
                    state: "CA".to_string(),
                },
            ],
        };
        // Serialize to XML
        let xml = to_string(&person);
        assert!(xml.is_ok(), "序列化失败");
    }
}

#[derive(Deserialize)]
pub struct GetBucketQueryParams {
    pub prefix: Option<String>,
}
pub(crate) async fn get_bucket(
    req: web::HttpRequest,
    Query(query): Query<GetBucketQueryParams>,
) -> HandlerResponse {
    let bucket_name: String = req
        .match_info()
        .query("bucket")
        .parse()
        .map_err(|_| BadRequest)?;
    let bucket_path = PathBuf::from(DATA_DIR)
        .join(BASIC_PATH_SUFFIX)
        .join(&bucket_name);
    if let Ok(files) = std::fs::read_dir(&bucket_path) {
        let mut contents = Vec::new();
        for file in files {
            if let Ok(file) = file {
                if file.file_type().unwrap().is_dir() {
                    // TODO: Handle directories
                } else if !file.file_type().unwrap().is_dir()
                    && file.file_name().to_string_lossy().ends_with(".meta")
                {
                    let meta_file_path = bucket_path.clone();
                    let meta_file_path = meta_file_path.join(&file.file_name());
                    let meta_file_path = meta_file_path.to_str().unwrap();
                    println!("{}", &meta_file_path);
                    let metadata = fs::load_metadata(&meta_file_path)?;
                    contents.push(Content {
                        size: metadata.size as i64,
                        key: metadata.name,
                        last_modified: metadata.time,
                    });
                }
            }
        }

        let result = ListBucketResult {
            name: bucket_name,
            prefix: query.prefix.unwrap_or_else(|| "".to_string()),
            is_truncated: false,
            max_keys: 100000,
            contents,
        };

        let xml = to_string(&result).context("序列化失败")?;
        Ok(HttpResponse::Ok().content_type("application/xml").body(xml))
    } else {
        Ok(HttpResponse::NotFound().finish())
    }
}
pub(crate) async fn head_bucket(req: web::HttpRequest) -> HandlerResponse {
    let bucket_name: String = req
        .match_info()
        .query("bucket")
        .parse()
        .map_err(|_| BadRequest)?;
    let file_path = PathBuf::from(DATA_DIR)
        .join(BASIC_PATH_SUFFIX)
        .join(bucket_name);
    if file_path.as_path().is_dir() {
        Ok(HttpResponse::Ok().content_type("application/xml").finish())
    } else {
        Ok(HttpResponse::NotFound()
            .content_type("application/xml")
            .finish())
    }
}
pub(crate) async fn create_bucket(req: web::HttpRequest) -> HandlerResponse {
    let bucket_name: String = req
        .match_info()
        .query("bucket")
        .parse()
        .map_err(|_| BadRequest)?;
    let file_path = PathBuf::from(DATA_DIR)
        .join(BASIC_PATH_SUFFIX)
        .join(bucket_name);
    std::fs::create_dir_all(file_path).context("创建桶失败")?;
    Ok(HttpResponse::Ok().content_type("application/xml").finish())
}

pub(crate) async fn delete_bucket(req: web::HttpRequest) -> HandlerResponse {
    let bucket_name: String = req
        .match_info()
        .query("bucket")
        .parse()
        .map_err(|_| BadRequest)?;
    let file_path = PathBuf::from(DATA_DIR)
        .join(BASIC_PATH_SUFFIX)
        .join(bucket_name);
    if std::fs::metadata(&file_path).is_ok() {
        std::fs::remove_dir_all(&file_path).context("删除桶失败")?;
        Ok(HttpResponse::Ok().content_type("application/xml").finish())
    } else {
        Ok(HttpResponse::NotFound()
            .content_type("application/xml")
            .finish())
    }
}

#[derive(Deserialize)]
pub struct InitChunkOrCombineQuery {
    pub upload_id: Option<String>,
}
pub(crate) async fn init_chunk_or_combine_chunk(
    req: web::HttpRequest,
    Query(query): Query<InitChunkOrCombineQuery>,
) -> HandlerResponse {
    let bucket_name: String = req
        .match_info()
        .query("bucket")
        .parse()
        .map_err(|_| BadRequest)?;
    let file_path = PathBuf::from(DATA_DIR)
        .join(BASIC_PATH_SUFFIX)
        .join(bucket_name);
    if let Some(upload_id) = query.upload_id {
        Ok(HttpResponse::Ok().content_type("application/xml").finish())
    } else {
        Ok(HttpResponse::Ok().content_type("application/xml").finish())
    }
}

async fn init_chunk() {}

async fn combine_chunk() {}

pub(crate) async fn head_object(req: web::HttpRequest) -> HandlerResponse {
    let bucket_name: String = req
        .match_info()
        .query("bucket")
        .parse()
        .map_err(|_| BadRequest)?;
    let object_name: String = req
        .match_info()
        .query("object")
        .parse()
        .map_err(|_| BadRequest)?;
    let file_path = PathBuf::from(DATA_DIR)
        .join(BASIC_PATH_SUFFIX)
        .join(bucket_name)
        .join(object_name);

    do_head_object(file_path).await
}

#[derive(Deserialize)]
pub struct UploadFileOrChunkQuery {
    pub upload_id: Option<String>,
    pub part_number: Option<String>,
}
pub(crate) async fn upload_file_or_upload_chunk(
    req: web::HttpRequest,
    body: web::types::Payload,
    Query(query): Query<UploadFileOrChunkQuery>,
) -> HandlerResponse {
    let bucket_name: String = req
        .match_info()
        .query("bucket")
        .parse()
        .map_err(|_| BadRequest)?;
    let object_name: String = req
        .match_info()
        .query("object")
        .parse()
        .map_err(|_| BadRequest)?;
    let file_path = PathBuf::from(DATA_DIR)
        .join(BASIC_PATH_SUFFIX)
        .join(bucket_name)
        .join(object_name);
    match (query.upload_id, query.part_number) {
        (Some(upload_id), Some(part_number)) => {
            println!("short path");
            Ok(HttpResponse::Ok().content_type("application/xml").finish())
        }
        _ => upload_file(file_path, body).await,
    }
}

pub(crate) async fn delete_file(req: web::HttpRequest) -> HandlerResponse {
    let bucket_name: String = req
        .match_info()
        .query("bucket")
        .parse()
        .map_err(|_| BadRequest)?;
    let object_name: String = req
        .match_info()
        .query("object")
        .parse()
        .map_err(|_| BadRequest)?;
    let file_path = PathBuf::from(DATA_DIR)
        .join(BASIC_PATH_SUFFIX)
        .join(bucket_name)
        .join(object_name);
    do_delete_file(file_path).await
}

async fn do_delete_file(file_path: PathBuf) -> HandlerResponse {
    let mut metainfo_file_path = file_path.clone().to_string_lossy().to_string();
    metainfo_file_path.push_str(".meta");
    if std::fs::metadata(&metainfo_file_path).is_ok() {
        std::fs::remove_file(&metainfo_file_path).context("删除文件失败")?;
        return Ok(HttpResponse::Ok().content_type("application/xml").finish());
    }
    Ok(HttpResponse::NotFound()
        .content_type("application/xml")
        .finish())
}
async fn upload_file(file_path: PathBuf, mut body: web::types::Payload) -> HandlerResponse {
    let mut metainfo_file_path = file_path.clone().to_string_lossy().to_string();
    metainfo_file_path.push_str(".meta");
    let file_name = file_path
        .file_name()
        .context("解析文件名失败")?
        .to_string_lossy()
        .to_string();
    let tmp_filename = file_name.clone();
    let file_type = MimeGuess::from_path(Path::new(&tmp_filename))
        .first_or_text_plain()
        .to_string();
    let mut file_size = match body.size_hint() {
        (_, Some(sz)) => sz,
        (sz, None) => sz,
    };
    let mut bytes = BytesMut::new();
    while let Some(item) = body.next().await {
        let item = item.context("")?;
        bytes.extend_from_slice(&item);
    }
    let chunks = bytes.chunks(8 << 20);
    let mut hashcodes: Vec<String> = Vec::new();
    let mut sz_flag = false;
    for chunk in chunks {
        let sha = sum_15bit_sha256(chunk);
        if file_size == 0 || sz_flag {
            if !sz_flag {
            sz_flag = true;
            }
            file_size += chunk.len();
        }
        hashcodes.push(sha.clone());
        if !fs::is_path_exist(&sha) {
            let compressed_chunk = fs::compress_chunk(chunk)?;
            save_file(&sha, &compressed_chunk)?;
        }
    }
    let metainfo = Metadata {
        name: file_name,
        size: file_size,
        file_type: file_type.to_string(),
        time: Utc::now(),
        chunks: hashcodes,
    };
    fs::save_metadata(&metainfo_file_path, &metainfo)?;
    Ok(web::HttpResponse::Ok()
        .content_type("application/xml")
        .finish())
}

pub(crate) async fn head_object_longpath(req: web::HttpRequest) -> HandlerResponse {
    let bucket_name: String = req
        .match_info()
        .query("bucket")
        .parse()
        .map_err(|_| BadRequest)?;
    let object_name: String = req
        .match_info()
        .query("object")
        .parse()
        .map_err(|_| BadRequest)?;
    let object_suffix: String = req
        .match_info()
        .query("objectSuffix")
        .parse()
        .map_err(|_| BadRequest)?;
    let file_path = PathBuf::from(DATA_DIR)
        .join(BASIC_PATH_SUFFIX)
        .join(bucket_name)
        .join(object_name)
        .join(object_suffix);
    do_head_object(file_path).await
}

async fn do_head_object(file_path: PathBuf) -> HandlerResponse {
    let mut metainfo_file_path = file_path.clone().to_string_lossy().to_string();
    metainfo_file_path.push_str(".meta");
    if !std::fs::metadata(&metainfo_file_path).is_ok() {
        let resp = HeadNotFoundResp {
            no_exist: "1".to_string(),
        };
        let xml = to_string(&resp).context("序列化失败")?;
        return Ok(web::HttpResponse::NotFound()
            .content_type("application/xml")
            .body(xml));
    }
    let metainfo = fs::load_metadata(&metainfo_file_path)?;
    Ok(web::HttpResponse::Ok()
        .content_type(metainfo.file_type)
        .header(
            "Content-Disposition",
            format!("attachment; filename=\"{}\"", metainfo.name),
        )
        .header(
            "Last-Modified",
            metainfo.time.format("%Y-%m-%d %H:%M:%S").to_string(),
        )
        .content_length(metainfo.size as u64)
        .body("body"))
}

pub(crate) async fn upload_file_or_upload_chunk_longpath(
    req: web::HttpRequest,
    body: web::types::Payload,
    Query(query): Query<UploadFileOrChunkQuery>,
) -> HandlerResponse {
    let bucket_name: String = req
        .match_info()
        .query("bucket")
        .parse()
        .map_err(|_| BadRequest)?;
    let object_name: String = req
        .match_info()
        .query("object")
        .parse()
        .map_err(|_| BadRequest)?;
    let object_suffix: String = req
        .match_info()
        .query("objectSuffix")
        .parse()
        .map_err(|_| BadRequest)?;
    let file_path = PathBuf::from(DATA_DIR)
        .join(BASIC_PATH_SUFFIX)
        .join(bucket_name)
        .join(object_name)
        .join(object_suffix);
    match (query.upload_id, query.part_number) {
        (Some(upload_id), Some(part_number)) => {
            println!("long path");
            Ok(HttpResponse::Ok().content_type("application/xml").finish())
        }
        _ => upload_file(file_path, body).await,
    }
}
pub(crate) async fn delete_file_longpath(req: web::HttpRequest) -> HandlerResponse {
    let bucket_name: String = req
        .match_info()
        .query("bucket")
        .parse()
        .map_err(|_| BadRequest)?;
    let object_name: String = req
        .match_info()
        .query("object")
        .parse()
        .map_err(|_| BadRequest)?;
    let object_suffix: String = req
        .match_info()
        .query("objectSuffix")
        .parse()
        .map_err(|_| BadRequest)?;
    let file_path = PathBuf::from(DATA_DIR)
        .join(BASIC_PATH_SUFFIX)
        .join(bucket_name)
        .join(object_name)
        .join(object_suffix);
    do_delete_file(file_path).await
}
pub(crate) async fn download_file_longpath(req: web::HttpRequest) -> HandlerResponse {
    let bucket_name: String = req
        .match_info()
        .query("bucket")
        .parse()
        .map_err(|_| BadRequest)?;
    let object_name: String = req
        .match_info()
        .query("object")
        .parse()
        .map_err(|_| BadRequest)?;
    let object_suffix: String = req
        .match_info()
        .query("objectSuffix")
        .parse()
        .map_err(|_| BadRequest)?;
    let file_path = PathBuf::from(DATA_DIR)
        .join(BASIC_PATH_SUFFIX)
        .join(bucket_name)
        .join(object_name)
        .join(object_suffix);
    do_download_file(file_path).await
}
pub(crate) async fn download_file(req: web::HttpRequest) -> HandlerResponse {
    let bucket_name: String = req
        .match_info()
        .query("bucket")
        .parse()
        .map_err(|_| BadRequest)?;
    let object_name: String = req
        .match_info()
        .query("object")
        .parse()
        .map_err(|_| BadRequest)?;
    let file_path = PathBuf::from(DATA_DIR)
        .join(BASIC_PATH_SUFFIX)
        .join(bucket_name)
        .join(object_name);
    do_download_file(file_path).await
}

async fn do_download_file(file_path: PathBuf) -> HandlerResponse {
    let mut metainfo_file_path = file_path.clone().to_string_lossy().to_string();
    metainfo_file_path.push_str(".meta");
    let meta_info = fs::load_metadata(&metainfo_file_path)?;
    let readers = multi_decompressed_reader(&meta_info.chunks[..]).await?;
    let content_disposition = format!("attachment; filename=\"{}\"", meta_info.name);
    let body = ReadStream::new(readers);
    Ok(web::HttpResponse::Ok()
        .header("Content-Type", "application/octet-stream")
        .header("Content-Length", meta_info.size)
        .header("Content-Disposition", content_disposition)
        .streaming(body))
}
