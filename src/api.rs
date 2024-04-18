use crate::err::AppError;
use crate::err::AppError::{Anyhow, BadRequest};
use crate::fs::{save_metadata, split_file_ann_save, Metadata, UncompressStream};
use crate::model::{
    Bucket, BucketWrapper, CompleteMultipartUpload, CompleteMultipartUploadResult, Content,
    HeadNotFoundResp, InitiateMultipartUploadResult, ListBucketResp, ListBucketResult, Owner,
};
use crate::stream::PayloadAsyncReader;
use crate::util::cry;
use crate::util::date::date_format_to_second;
use crate::{fs, HandlerResponse};
use anyhow::{anyhow, Context};
use chrono::Utc;
use futures::future::ok;
use futures::stream::once;
use futures::StreamExt;
use log::info;
use mime_guess::MimeGuess;
use ntex::util::{Bytes, BytesMut};
use ntex::web;
use ntex::web::types::Query;
use ntex::web::HttpResponse;
use quick_xml::se::to_string;
use rayon::prelude::*;
use serde::Deserialize;
use std::fs::read_dir;
use std::path::{Path, PathBuf};
use uuid::Uuid;
use zstd::zstd_safe::WriteBuf;

const DATA_DIR: &str = "data";
const BASIC_PATH_SUFFIX: &str = "buckets";

pub fn rest(cfg: &mut web::ServiceConfig) {
    cfg
        // 应用日志记录中间件，记录请求和响应。
        // 定义路由和相应的处理函数。
        .route(
            "/health",
            web::get().to(|| async { web::HttpResponse::Ok().body("ok") }),
        )
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
            if file.file_type().unwrap().is_dir() {
                // TODO: Handle directories
            } else if !file.file_type().unwrap().is_dir()
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
pub async fn create_bucket(req: web::HttpRequest) -> HandlerResponse {
    let bucket_name: String = get_path_param(&req, "bucket")?;
    let file_path = PathBuf::from(DATA_DIR)
        .join(BASIC_PATH_SUFFIX)
        .join(bucket_name);
    std::fs::create_dir_all(file_path).context("创建桶失败")?;
    Ok(HttpResponse::Ok().content_type("application/xml").finish())
}

// 删除桶
pub async fn delete_bucket(req: web::HttpRequest) -> HandlerResponse {
    let bucket_name: String = get_path_param(&req, "bucket")?;
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
    #[serde(rename = "uploadId")]
    pub upload_id: Option<String>,
}

// 初始化分片上传 & 完成分片上传
pub async fn init_chunk_or_combine_chunk(
    req: web::HttpRequest,
    mut body: web::types::Payload,
    Query(query): Query<InitChunkOrCombineQuery>,
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
        combine_chunk(&bucket_name, &object_name, &upload_id, cmu).await
    } else {
        init_chunk(bucket_name, object_name).await
    }
}

// 对长路径的初始化分片上传或完成分片上传
pub async fn init_chunk_or_combine_chunk_longpath(
    req: web::HttpRequest,
    mut body: web::types::Payload,
    Query(query): Query<InitChunkOrCombineQuery>,
) -> HandlerResponse {
    let bucket_name: String = get_path_param(&req, "bucket")?;
    let object_name: String = get_path_param(&req, "object")?;
    let object_suffix: String = get_path_param(&req, "objectSuffix")?;
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
        let object_key = PathBuf::from(&object_name)
            .join(&object_suffix)
            .to_string_lossy()
            .to_string();
        combine_chunk(&bucket_name, &object_key, &upload_id, cmu).await
    } else {
        init_chunk(
            bucket_name,
            PathBuf::from(object_name)
                .join(object_suffix)
                .to_string_lossy()
                .to_string(),
        )
        .await
    }
}

// 初始化分片上传
async fn init_chunk(bucket: String, object_key: String) -> HandlerResponse {
    let guid = Uuid::new_v4();
    let upload_id = guid.to_string();
    let file_size_dir = PathBuf::from(DATA_DIR).join("tmp").join(&upload_id);
    let extension = &format!(".meta.{}", &upload_id);
    let mut tmp_dir = PathBuf::from(DATA_DIR)
        .join(BASIC_PATH_SUFFIX)
        .join(&bucket)
        .join(&object_key)
        .to_string_lossy()
        .to_string();
    tmp_dir.push_str(extension);
    std::fs::create_dir_all(file_size_dir).map_err(|err| anyhow!(err))?;
    let file_name = Path::new(&object_key)
        .file_name()
        .context("解析文件名失败")?
        .to_string_lossy()
        .to_string();
    let file_type = MimeGuess::from_path(Path::new(&file_name))
        .first_or_text_plain()
        .to_string();
    let meta_info = Metadata {
        name: file_name,
        size: 0,
        file_type,
        time: Default::default(),
        chunks: vec![],
    };
    save_metadata(&tmp_dir, &meta_info)?;
    let resp = InitiateMultipartUploadResult {
        bucket,
        object_key,
        upload_id,
    };
    let xml = to_string(&resp).map_err(|err| anyhow!(err))?;
    Ok(HttpResponse::Ok().content_type("application/xml").body(xml))
}

// 完成分片上传
async fn combine_chunk(
    bucket_name: &str,
    object_key: &str,
    upload_id: &str,
    cmu: CompleteMultipartUpload,
) -> HandlerResponse {
    info!("合并分片，uploadId: {}", upload_id);
    let mut part_etags = cmu.part_etags;

    let mut check = true;
    let mut total_len: u64 = 0;

    let extension = &format!(".meta.{}", &upload_id);
    let mut tmp_metadata_dir = PathBuf::from(DATA_DIR)
        .join(BASIC_PATH_SUFFIX)
        .join(bucket_name)
        .join(object_key)
        .to_string_lossy()
        .to_string();
    tmp_metadata_dir.push_str(extension);
    let tmp_metadata_dir = PathBuf::from(tmp_metadata_dir);
    if !tmp_metadata_dir.as_path().exists() {
        info!("未初始化");
        return Err(Anyhow(anyhow!("未初始化".to_string())));
    }

    for part_etag in &part_etags {
        if !fs::is_path_exist(&part_etag.etag) {
            check = false;
            break;
        }
        let len_path = PathBuf::from(DATA_DIR)
            .join("tmp")
            .join(upload_id)
            .join(&format!("{}", part_etag.part_number));
        let len: u64 = std::fs::read_to_string(len_path)
            .context("读取长度文件失败")?
            .parse()
            .context("解析长度文件失败")?;
        total_len += len;
    }

    if !check {
        info!("分片不完整");
        return Err(Anyhow(anyhow!("分片不完整".to_string())));
    }
    part_etags.sort_by_key(|p| p.part_number);
    let chunks: Vec<String> = part_etags.par_iter().map(|p| p.etag.clone()).collect();
    let mut metadata = fs::load_metadata(tmp_metadata_dir.to_string_lossy().as_ref())?;
    info!("读取临时元数据成功");
    metadata.size = total_len;
    metadata.chunks = chunks;
    metadata.time = Utc::now();

    let mut metadata_dir = PathBuf::from(DATA_DIR)
        .join(BASIC_PATH_SUFFIX)
        .join(bucket_name)
        .join(object_key)
        .to_string_lossy()
        .to_string();
    metadata_dir.push_str(".meta");
    save_metadata(&metadata_dir, &metadata)?;
    info!("保存新元数据成功");
    std::fs::remove_file(tmp_metadata_dir).context("删除临时元数据失败")?;
    std::fs::remove_dir_all(PathBuf::from(DATA_DIR).join("tmp").join(upload_id))
        .context("删除临时文件夹失败")?;

    let e_tag = cry::encrypt_by_md5(&format!("{}/{}", bucket_name, object_key));
    let res = CompleteMultipartUploadResult {
        bucket_name: bucket_name.to_string(),
        object_key: object_key.to_string(),
        etag: e_tag,
    };
    let xml = to_string(&res).map_err(|err| anyhow!(err))?;
    Ok(HttpResponse::Ok().content_type("application/xml").body(xml))
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
    body: web::types::Payload,
    Query(query): Query<UploadFileOrChunkQuery>,
) -> HandlerResponse {
    let bucket_name: String = get_path_param(&req, "bucket")?;
    let object_name: String = get_path_param(&req, "object")?;
    let file_path = PathBuf::from(DATA_DIR)
        .join(BASIC_PATH_SUFFIX)
        .join(&bucket_name)
        .join(&object_name);
    match (query.upload_id, query.part_number) {
        (Some(upload_id), Some(part_number)) => {
            upload_chunk(&bucket_name, &object_name, &part_number, &upload_id, body).await
        }
        _ => {
            if let Some(copy_source) = req.headers().get("x-amz-copy-source") {
                copy_object(
                    copy_source.to_str().map_err(|_| BadRequest)?,
                    &bucket_name,
                    &object_name,
                )
                .await
            } else {
                upload_file(file_path, body).await
            }
        }
    }
}

// 桶间拷贝对象数据
async fn copy_object(copy_source: &str, dest_bucket: &str, dest_object: &str) -> HandlerResponse {
    let mut copy_source = copy_source.to_string();
    if copy_source.contains('?') {
        copy_source = copy_source.split('?').next().unwrap().to_string();
    }

    let copy_list: Vec<&str> = copy_source.split('/').collect();
    let copy_list = &copy_list[..copy_list.len() - 1];

    let mut src_bucket_name = String::new();
    for &it in copy_list {
        if !it.is_empty() {
            src_bucket_name = it.to_string();
            break;
        }
    }

    let mut res = String::new();
    for i in copy_list.iter().skip(1) {
        res.push_str(i);
        res.push('/');
    }
    let src_object = &res;
    let src_metadata_path = PathBuf::from(DATA_DIR)
        .join(BASIC_PATH_SUFFIX)
        .join(src_bucket_name)
        .join(src_object);
    let dest_metadata_path = PathBuf::from(DATA_DIR)
        .join(BASIC_PATH_SUFFIX)
        .join(dest_bucket)
        .join(dest_object);
    std::fs::copy(src_metadata_path, dest_metadata_path).map_err(|err| anyhow!(err))?;

    Ok(HttpResponse::Ok().finish())
}

// 上传分片
pub(crate) async fn upload_chunk(
    _bucket_name: &str,
    _object_key: &str,
    part_number: &str,
    upload_id: &str,
    mut body: web::types::Payload,
) -> HandlerResponse {
    let mut bytes = Vec::new();
    bytes.reserve_exact(8 << 20);
    while let Some(item) = body.next().await {
        let item = item.map_err(|err| anyhow!(err.to_string()))?;
        bytes.extend_from_slice(&item);
    }
    let hash = fs::sum_sha256(&bytes).await;
    let path = fs::path_from_hash(&hash);
    if fs::is_path_exist(&path.to_string_lossy().to_string()) {
        return Ok(HttpResponse::Ok().header("ETag", &hash).body(&hash));
    }
    let hash_clone = hash.clone();
    let len = bytes.len();
    let part_path = PathBuf::from(DATA_DIR)
        .join("tmp")
        .join(upload_id)
        .join(part_number);
    tokio::fs::write(part_path, format!("{}", len))
        .await
        .map_err(|err| anyhow!(err.to_string()))?;
    let body = fs::compress_chunk(std::io::Cursor::new(&bytes))?;
    fs::save_file(&hash_clone, body)?;
    Ok(HttpResponse::Ok().header("ETag", &hash).finish())
}

// 删除文件
pub async fn delete_file(req: web::HttpRequest) -> HandlerResponse {
    let bucket_name: String = get_path_param(&req, "bucket")?;
    let object_name: String = get_path_param(&req, "object")?;
    let file_path = PathBuf::from(DATA_DIR)
        .join(BASIC_PATH_SUFFIX)
        .join(bucket_name)
        .join(object_name);
    do_delete_file(file_path).await
}

// 删除文件逻辑
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

// 上传文件
async fn upload_file(file_path: PathBuf, body: web::types::Payload) -> HandlerResponse {
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

    let reader = PayloadAsyncReader::new(body);
    let (file_size, hashcodes) = split_file_ann_save(Box::new(reader), 8 << 20).await?;
    let metainfo = Metadata {
        name: file_name,
        size: file_size as u64,
        file_type: file_type.to_string(),
        time: Utc::now(),
        chunks: hashcodes,
    };
    fs::save_metadata(&metainfo_file_path, &metainfo)?;
    Ok(web::HttpResponse::Ok()
        .content_type("application/xml")
        .finish())
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
    body: web::types::Payload,
    Query(query): Query<UploadFileOrChunkQuery>,
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
            upload_chunk(&bucket_name, &object_key, &part_number, &upload_id, body).await
        }
        _ => {
            if let Some(copy_source) = req.headers().get("x-amz-copy-source") {
                copy_object(
                    copy_source.to_str().map_err(|_| BadRequest)?,
                    &bucket_name,
                    &object_key,
                )
                .await
            } else {
                upload_file(file_path, body).await
            }
        }
    }
}

// 长路径删除文件
pub async fn delete_file_longpath(req: web::HttpRequest) -> HandlerResponse {
    let bucket_name: String = get_path_param(&req, "bucket")?;
    let object_name: String = get_path_param(&req, "object")?;
    let object_suffix: String = get_path_param(&req, "objectSuffix")?;
    let file_path = PathBuf::from(DATA_DIR)
        .join(BASIC_PATH_SUFFIX)
        .join(bucket_name)
        .join(object_name)
        .join(object_suffix);
    do_delete_file(file_path).await
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
