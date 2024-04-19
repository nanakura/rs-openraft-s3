use crate::err::AppError;
use crate::err::AppError::BadRequest;
use crate::fs::UncompressStream;
use crate::model::{
    Bucket, BucketWrapper, CompleteMultipartUpload, Content, HeadNotFoundResp, ListBucketResp,
    ListBucketResult, Owner,
};
use crate::raft::app::App;
use crate::raft::store::Request::{
    CombineChunk, CopyFile, CreateBucket, DeleteBucket, DeleteFile, InitChunk, UploadChunk,
    UploadFile,
};
use crate::util::date::date_format_to_second;
use crate::{fs, HandlerResponse};
use anyhow::{anyhow, Context};
use futures::future::ok;
use futures::stream::once;
use futures::StreamExt;
use log::info;
use ntex::util::{Bytes, BytesMut};
use ntex::web;
use ntex::web::types::Query;
use ntex::web::HttpResponse;
use quick_xml::se::to_string;
use rayon::prelude::*;
use serde::Deserialize;
use std::fs::read_dir;
use std::path::PathBuf;
use zstd::zstd_safe::WriteBuf;

pub(crate) const DATA_DIR: &str = "data";
pub(crate) const BASIC_PATH_SUFFIX: &str = "buckets";

pub fn rest(cfg: &mut web::ServiceConfig) {
    cfg
        // 应用日志记录中间件，记录请求和响应。
        // 定义路由和相应的处理函数。
        .route("/api", web::get().to(list_bucket))
        .route("/api/{bucket}", web::get().to(get_bucket))
        .route("/api/{bucket}", web::head().to(head_bucket))
        .route("/api/{bucket}", web::put().to(create_bucket))
        .route("/api/{bucket}", web::delete().to(delete_bucket))
        .route("/api/{bucket}/", web::get().to(get_bucket))
        .route("/api/{bucket}/", web::head().to(head_bucket))
        .route("/api/{bucket}/", web::put().to(create_bucket))
        .route("/api/{bucket}/", web::delete().to(delete_bucket))
        .route(
            "/api/{bucket}/{object}",
            web::post().to(init_chunk_or_combine_chunk),
        )
        .route("/api/{bucket}/{object}", web::head().to(head_object))
        .route(
            "/api/{bucket}/{object}",
            web::put().to(upload_file_or_upload_chunk),
        )
        .route("/api/{bucket}/{object}", web::delete().to(delete_file))
        .route("/api/{bucket}/{object}", web::get().to(download_file))
        .route(
            "/api/{bucket}/{object}/{objectSuffix}*",
            web::post().to(init_chunk_or_combine_chunk_longpath),
        )
        .route(
            "/api/{bucket}/{object}/{objectSuffix}*",
            web::head().to(head_object_longpath),
        )
        .route(
            "/api/{bucket}/{object}/{objectSuffix}*",
            web::put().to(upload_file_or_upload_chunk_longpath),
        )
        .route(
            "/api/{bucket}/{object}/{objectSuffix}*",
            web::delete().to(delete_file_longpath),
        )
        .route(
            "/api/{bucket}/{object}/{objectSuffix}*",
            web::get().to(download_file_longpath),
        );
}

// 从uri path中获取参数
fn get_path_param(req: &web::HttpRequest, name: &str) -> Result<String, AppError> {
    let param: String = req
        .match_info()
        .query(name)
        .parse()
        .map_err(|_| BadRequest)?;
    Ok(param)
}

// 获取所有桶的列表
pub async fn list_bucket() -> HandlerResponse {
    let dir_path = PathBuf::from(DATA_DIR).join(BASIC_PATH_SUFFIX);
    let dir_path = dir_path.as_path();
    if dir_path.is_dir() {
        let mut res = Vec::new();
        if let Ok(dir) = read_dir(dir_path) {
            for entry in dir.flatten() {
                if entry.file_type().unwrap().is_dir() {
                    let metadata = entry.metadata().context("转换失败")?;
                    let mod_time = metadata.modified().context("转换失败")?;
                    let bucket = Bucket {
                        name: entry.file_name().to_string_lossy().to_string(),
                        creation_date: date_format_to_second(mod_time.into()),
                    };
                    res.push(bucket);
                }
            }
        }
        let mut buckets = Vec::new();
        for bucket in res {
            buckets.push(bucket);
        }
        let list_res = ListBucketResp {
            id: "20230529".to_string(),
            owner: Owner {
                display_name: "minioadmin".to_string(),
            },
            buckets: BucketWrapper { bucket: buckets },
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
            buckets: BucketWrapper { bucket: buckets },
        };
        let xml = to_string(&list_res).context("序列化失败")?;
        Ok(HttpResponse::Ok().content_type("application/xml").body(xml))
    }
}

#[derive(Deserialize)]
pub struct GetBucketQueryParams {
    pub prefix: Option<String>,
}
// 获取桶的数据
pub async fn get_bucket(
    req: web::HttpRequest,
    Query(query): Query<GetBucketQueryParams>,
) -> HandlerResponse {
    let bucket_name: String = get_path_param(&req, "bucket")?;

    let bucket_path = PathBuf::from(DATA_DIR)
        .join(BASIC_PATH_SUFFIX)
        .join(&bucket_name);
    if let Ok(files) = std::fs::read_dir(&bucket_path) {
        let mut contents = Vec::new();
        for file in files.flatten() {
            if !file.file_type().unwrap().is_dir()
                && file.file_name().to_string_lossy().ends_with(".meta")
            {
                let meta_file_path = bucket_path.clone();
                let meta_file_path = meta_file_path.join(&file.file_name());
                let meta_file_path = meta_file_path.to_str().unwrap();
                info!("{}", &meta_file_path);
                let metadata = fs::load_metadata(meta_file_path)?;
                contents.push(Content {
                    size: metadata.size as i64,
                    key: metadata.name,
                    last_modified: metadata.time,
                });
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

// 查询桶是否存在
pub async fn head_bucket(req: web::HttpRequest) -> HandlerResponse {
    let bucket_name: String = get_path_param(&req, "bucket")?;
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

// 创建桶
pub async fn create_bucket(
    req: web::HttpRequest,
    state: web::types::State<App>,
) -> HandlerResponse {
    let bucket_name: String = get_path_param(&req, "bucket")?;
    let file_path = PathBuf::from(DATA_DIR)
        .join(BASIC_PATH_SUFFIX)
        .join(bucket_name);
    let _ = state.raft.client_write(CreateBucket {
        bucket_name: file_path.to_string_lossy().to_string(),
    });
    //std::fs::create_dir_all(file_path).context("创建桶失败")?;
    Ok(HttpResponse::Ok().finish())
}

// 删除桶
pub async fn delete_bucket(
    req: web::HttpRequest,
    state: web::types::State<App>,
) -> HandlerResponse {
    let bucket_name: String = get_path_param(&req, "bucket")?;
    let file_path = PathBuf::from(DATA_DIR)
        .join(BASIC_PATH_SUFFIX)
        .join(bucket_name);

    let _ = state
        .raft
        .client_write(DeleteBucket {
            bucket_name: file_path.to_string_lossy().to_string(),
        })
        .await;

    Ok(HttpResponse::Ok().finish())
    // if std::fs::metadata(&file_path).is_ok() {
    //     std::fs::remove_dir_all(&file_path).context("删除桶失败")?;
    //     Ok(HttpResponse::Ok().content_type("application/xml").finish())
    // } else {
    //     Ok(HttpResponse::NotFound()
    //         .content_type("application/xml")
    //         .finish())
    // }
}

#[derive(Deserialize)]
pub struct InitChunkOrCombineQuery {
    #[serde(rename = "uploadId")]
    pub upload_id: Option<String>,
}

// 初始化分片上传 & 完成分片上传
pub async fn init_chunk_or_combine_chunk(
    req: web::HttpRequest,
    mut body: web::types::Payload,
    Query(query): Query<InitChunkOrCombineQuery>,
    state: web::types::State<App>,
) -> HandlerResponse {
    let bucket_name: String = get_path_param(&req, "bucket")?;
    let object_name: String = get_path_param(&req, "object")?;
    if let Some(upload_id) = query.upload_id {
        let mut bytes = BytesMut::new();
        while let Some(item) = body.next().await {
            let item = item.context("")?;
            bytes.extend_from_slice(&item);
        }
        let body = std::str::from_utf8(bytes.as_slice()).map_err(|err| anyhow!(err))?;
        let cmu: CompleteMultipartUpload =
            quick_xml::de::from_str(body).map_err(|err| anyhow!(err))?;
        let _ = state
            .raft
            .client_write(CombineChunk {
                bucket_name,
                object_key: object_name,
                upload_id,
                cmu,
            })
            .await;

        Ok(HttpResponse::Ok().finish())
        //combine_chunk(&bucket_name, &object_name, &upload_id, cmu).await
    } else {
        let _ = state
            .raft
            .client_write(InitChunk {
                bucket_name,
                object_key: object_name,
            })
            .await;
        Ok(HttpResponse::Ok().finish())
        //init_chunk(bucket_name, object_name).await
    }
}

// 对长路径的初始化分片上传或完成分片上传
pub async fn init_chunk_or_combine_chunk_longpath(
    req: web::HttpRequest,
    mut body: web::types::Payload,
    Query(query): Query<InitChunkOrCombineQuery>,
    state: web::types::State<App>,
) -> HandlerResponse {
    let bucket_name: String = get_path_param(&req, "bucket")?;
    let object_name: String = get_path_param(&req, "object")?;
    let object_suffix: String = get_path_param(&req, "objectSuffix")?;
    let object_key = PathBuf::from(&object_name)
        .join(&object_suffix)
        .to_string_lossy()
        .to_string();
    if let Some(upload_id) = query.upload_id {
        info!("uploadId: {}", upload_id);
        let mut bytes = BytesMut::new();
        while let Some(item) = body.next().await {
            let item = item.context("")?;
            bytes.extend_from_slice(&item);
        }
        let body = std::str::from_utf8(bytes.as_slice()).map_err(|err| anyhow!(err))?;

        let cmu: CompleteMultipartUpload =
            quick_xml::de::from_str(body).map_err(|err| anyhow!(err))?;
        let _ = state
            .raft
            .client_write(CombineChunk {
                bucket_name,
                object_key,
                upload_id,
                cmu,
            })
            .await;
        Ok(HttpResponse::Ok().finish())
        //combine_chunk(&bucket_name, &object_key, &upload_id, cmu).await
    } else {
        let _ = state
            .raft
            .client_write(InitChunk {
                bucket_name,
                object_key,
            })
            .await;
        // init_chunk(
        //     bucket_name,
        //     PathBuf::from(object_name)
        //         .join(object_suffix)
        //         .to_string_lossy()
        //         .to_string(),
        // )
        // .await

        Ok(HttpResponse::Ok().finish())
    }
}

// 查询对象信息
pub async fn head_object(req: web::HttpRequest) -> HandlerResponse {
    let bucket_name: String = get_path_param(&req, "bucket")?;
    let object_name: String = get_path_param(&req, "object")?;
    let file_path = PathBuf::from(DATA_DIR)
        .join(BASIC_PATH_SUFFIX)
        .join(bucket_name)
        .join(object_name);

    do_head_object(file_path).await
}

#[derive(Deserialize)]
pub struct UploadFileOrChunkQuery {
    #[serde(rename = "uploadId")]
    pub upload_id: Option<String>,
    #[serde(rename = "partNumber")]
    pub part_number: Option<String>,
}

// 上传文件 & 上传文件分片
pub async fn upload_file_or_upload_chunk(
    req: web::HttpRequest,
    mut body: web::types::Payload,
    Query(query): Query<UploadFileOrChunkQuery>,
    state: web::types::State<App>,
) -> HandlerResponse {
    let bucket_name: String = get_path_param(&req, "bucket")?;
    let object_name: String = get_path_param(&req, "object")?;
    let file_path = PathBuf::from(DATA_DIR)
        .join(BASIC_PATH_SUFFIX)
        .join(&bucket_name)
        .join(&object_name);
    match (query.upload_id, query.part_number) {
        (Some(upload_id), Some(part_number)) => {
            let mut bytes = Vec::new();
            bytes.reserve_exact(8 << 20);
            while let Some(item) = body.next().await {
                let item = item.map_err(|err| anyhow!(err.to_string()))?;
                bytes.extend_from_slice(&item);
            }
            let _ = state
                .raft
                .client_write(UploadChunk {
                    part_number,
                    upload_id,
                    body: bytes.to_vec(),
                })
                .await;
            //upload_chunk(&bucket_name, &object_name, &part_number, &upload_id, body).await
            Ok(HttpResponse::Ok().finish())
        }
        _ => {
            if let Some(copy_source) = req.headers().get("x-amz-copy-source") {
                let _ = state
                    .raft
                    .client_write(CopyFile {
                        copy_source: copy_source.to_str().unwrap().to_string(),
                        dest_bucket: bucket_name,
                        dest_object: object_name,
                    })
                    .await;
                // copy_object(
                //     copy_source.to_str().map_err(|_| BadRequest)?,
                //     &bucket_name,
                //     &object_name,
                // )
                // .await
                Ok(HttpResponse::Ok().finish())
            } else {
                let mut bytes = Vec::new();
                bytes.reserve_exact(8 << 20);
                while let Some(item) = body.next().await {
                    let item = item.map_err(|err| anyhow!(err.to_string()))?;
                    bytes.extend_from_slice(&item);
                }

                let mut metainfo_file_path = file_path.clone().to_string_lossy().to_string();
                metainfo_file_path.push_str(".meta");
                let _ = state
                    .raft
                    .client_write(UploadFile {
                        file_path: metainfo_file_path,
                        body: bytes.to_vec(),
                    })
                    .await;
                //upload_file(file_path, body).await
                Ok(HttpResponse::Ok().finish())
            }
        }
    }
}

// 删除文件
pub async fn delete_file(req: web::HttpRequest, state: web::types::State<App>) -> HandlerResponse {
    let bucket_name: String = get_path_param(&req, "bucket")?;
    let object_name: String = get_path_param(&req, "object")?;
    let file_path = PathBuf::from(DATA_DIR)
        .join(BASIC_PATH_SUFFIX)
        .join(bucket_name)
        .join(object_name);
    let mut metainfo_file_path = file_path.clone().to_string_lossy().to_string();
    metainfo_file_path.push_str(".meta");
    let _ = state
        .raft
        .client_write(DeleteFile {
            file_path: metainfo_file_path,
        })
        .await;
    //do_delete_file(file_path).await
    Ok(HttpResponse::Ok().finish())
}

// 长路径获取对象信息
pub async fn head_object_longpath(req: web::HttpRequest) -> HandlerResponse {
    let bucket_name: String = get_path_param(&req, "bucket")?;
    let object_name: String = get_path_param(&req, "object")?;
    let object_suffix: String = get_path_param(&req, "objectSuffix")?;
    let file_path = PathBuf::from(DATA_DIR)
        .join(BASIC_PATH_SUFFIX)
        .join(bucket_name)
        .join(object_name)
        .join(object_suffix);
    do_head_object(file_path).await
}

// 获取对象信息逻辑
async fn do_head_object(file_path: PathBuf) -> HandlerResponse {
    let mut metainfo_file_path = file_path.clone().to_string_lossy().to_string();
    metainfo_file_path.push_str(".meta");
    info!("{}", metainfo_file_path);
    if std::fs::metadata(&metainfo_file_path).is_err() {
        let resp = HeadNotFoundResp {
            no_exist: "1".to_string(),
        };
        let xml = to_string(&resp).context("序列化失败")?;
        return Ok(web::HttpResponse::NotFound()
            .content_type("application/xml")
            .body(xml));
    }
    let metainfo = fs::load_metadata(&metainfo_file_path)?;

    let body = once(ok::<_, web::Error>(Bytes::new()));
    let last_modified = date_format_to_second(metainfo.time);
    Ok(web::HttpResponse::Ok()
        .content_type(metainfo.file_type)
        .header(
            "Content-Disposition",
            format!("attachment; filename=\"{}\"", metainfo.name),
        )
        .header("Last-Modified", last_modified)
        .content_length(metainfo.size as u64)
        .no_chunking()
        .streaming(body))
}

// 长路径上传文件 & 上传文件分片
pub async fn upload_file_or_upload_chunk_longpath(
    req: web::HttpRequest,
    mut body: web::types::Payload,
    Query(query): Query<UploadFileOrChunkQuery>,
    state: web::types::State<App>,
) -> HandlerResponse {
    let bucket_name: String = get_path_param(&req, "bucket")?;
    let object_name: String = get_path_param(&req, "object")?;
    let object_suffix: String = get_path_param(&req, "objectSuffix")?;
    let file_path = PathBuf::from(DATA_DIR)
        .join(BASIC_PATH_SUFFIX)
        .join(&bucket_name)
        .join(&object_name)
        .join(&object_suffix);
    let object_key = PathBuf::from(&object_name)
        .join(&object_suffix)
        .to_string_lossy()
        .to_string();
    match (query.upload_id, query.part_number) {
        (Some(upload_id), Some(part_number)) => {
            info!("long path");
            let mut bytes = Vec::new();
            bytes.reserve_exact(8 << 20);
            while let Some(item) = body.next().await {
                let item = item.map_err(|err| anyhow!(err.to_string()))?;
                bytes.extend_from_slice(&item);
            }
            let _ = state
                .raft
                .client_write(UploadChunk {
                    part_number,
                    upload_id,
                    body: bytes.to_vec(),
                })
                .await;
            //upload_chunk(&bucket_name, &object_key, &part_number, &upload_id, body).await
            Ok(HttpResponse::Ok().finish())
        }
        _ => {
            if let Some(copy_source) = req.headers().get("x-amz-copy-source") {
                let _ = state
                    .raft
                    .client_write(CopyFile {
                        copy_source: copy_source.to_str().unwrap().to_string(),
                        dest_bucket: bucket_name,
                        dest_object: object_key,
                    })
                    .await;
                // copy_object(
                //     copy_source.to_str().map_err(|_| BadRequest)?,
                //     &bucket_name,
                //     &object_key,
                // )
                // .await
                Ok(HttpResponse::Ok().finish())
            } else {
                let mut bytes = Vec::new();
                bytes.reserve_exact(8 << 20);
                while let Some(item) = body.next().await {
                    let item = item.map_err(|err| anyhow!(err.to_string()))?;
                    bytes.extend_from_slice(&item);
                }
                let mut metainfo_file_path = file_path.clone().to_string_lossy().to_string();
                metainfo_file_path.push_str(".meta");
                let _ = state
                    .raft
                    .client_write(UploadFile {
                        file_path: metainfo_file_path,
                        body: bytes.to_vec(),
                    })
                    .await;
                //upload_file(file_path, body).await
                Ok(HttpResponse::Ok().finish())
            }
        }
    }
}

// 长路径删除文件
pub async fn delete_file_longpath(
    req: web::HttpRequest,
    state: web::types::State<App>,
) -> HandlerResponse {
    let bucket_name: String = get_path_param(&req, "bucket")?;
    let object_name: String = get_path_param(&req, "object")?;
    let object_suffix: String = get_path_param(&req, "objectSuffix")?;
    let file_path = PathBuf::from(DATA_DIR)
        .join(BASIC_PATH_SUFFIX)
        .join(bucket_name)
        .join(object_name)
        .join(object_suffix);
    let _ = state
        .raft
        .client_write(DeleteFile {
            file_path: file_path.to_string_lossy().to_string(),
        })
        .await;
    //do_delete_file(file_path).await
    Ok(HttpResponse::Ok().finish())
}

// 产路径删除文件
pub async fn download_file_longpath(req: web::HttpRequest) -> HandlerResponse {
    let bucket_name: String = get_path_param(&req, "bucket")?;
    let object_name: String = get_path_param(&req, "object")?;
    let object_suffix: String = get_path_param(&req, "objectSuffix")?;
    let file_path = PathBuf::from(DATA_DIR)
        .join(BASIC_PATH_SUFFIX)
        .join(bucket_name)
        .join(object_name)
        .join(object_suffix);
    do_download_file(file_path).await
}

// 下载文件
pub async fn download_file(req: web::HttpRequest) -> HandlerResponse {
    let bucket_name: String = get_path_param(&req, "bucket")?;
    let object_name: String = get_path_param(&req, "object")?;
    let file_path = PathBuf::from(DATA_DIR)
        .join(BASIC_PATH_SUFFIX)
        .join(bucket_name)
        .join(object_name);
    do_download_file(file_path).await
}

// 下载文件逻辑
async fn do_download_file(file_path: PathBuf) -> HandlerResponse {
    let mut metainfo_file_path = file_path.clone().to_string_lossy().to_string();
    metainfo_file_path.push_str(".meta");
    let meta_info = fs::load_metadata(&metainfo_file_path)?;
    // let readers = multi_decompressed_reader(&meta_info.chunks[..]).await?;
    let body = UncompressStream::new(meta_info.chunks);
    let content_disposition = format!("attachment; filename=\"{}\"", meta_info.name);
    Ok(web::HttpResponse::Ok()
        .header("Content-Type", "application/octet-stream")
        .header("Content-Length", meta_info.size)
        .header("Last-Modified", date_format_to_second(meta_info.time))
        .header("Content-Disposition", content_disposition)
        .streaming(body))
}
