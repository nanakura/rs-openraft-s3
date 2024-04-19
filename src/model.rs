use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// 定义数据结构

// 桶数据
#[derive(Debug, Serialize, Deserialize)]
pub struct Bucket {
    #[serde(rename = "Name")]
    pub name: String,
    #[serde(rename = "CreationDate")]
    pub creation_date: String,
}

// 完成上传请求体
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompleteMultipartUpload {
    #[serde(rename = "Part")]
    pub part_etags: Vec<PartETag>,
}

// 完成上传返回结果
#[derive(Debug, Serialize, Deserialize)]
pub struct CompleteMultipartUploadResult {
    #[serde(rename = "BucketName")]
    pub bucket_name: String,
    #[serde(rename = "ObjectKey")]
    pub object_key: String,
    #[serde(rename = "ETag")]
    pub etag: String,
}

// 初始化分片上传请求结果
#[derive(Debug, Serialize, Deserialize)]
pub struct InitiateMultipartUploadResult {
    #[serde(rename = "Bucket")]
    pub bucket: String,
    #[serde(rename = "ObjectKey")]
    pub object_key: String,
    #[serde(rename = "UploadId")]
    pub upload_id: String,
}

// #[derive(Debug, Serialize, Deserialize)]
// pub struct ListBucketsResult {
//     pub buckets: Vec<Bucket>,
// }

// 文件列表
#[derive(Debug, Serialize)]
#[serde(rename = "ListBucketResult")]
pub struct ListBucketResult {
    #[serde(rename = "Name")]
    pub name: String,
    #[serde(rename = "Prefix")]
    pub prefix: String,
    #[serde(rename = "MaxKeys")]
    pub max_keys: u32,
    #[serde(rename = "IsTruncated")]
    pub is_truncated: bool,
    #[serde(rename = "Contents")]
    pub contents: Vec<Content>,
}

// 文件列表数据实体
#[derive(Debug, Serialize)]
pub struct Content {
    #[serde(rename = "Key")]
    pub key: String,
    #[serde(rename = "LastModified")]
    //#[serde(serialize_with = "serialize_date", deserialize_with = "deserialize_date")]
    pub last_modified: DateTime<Utc>,
    #[serde(rename = "Size")]
    pub size: i64,
}

// 判断是否存在请求结果
#[derive(Debug, Serialize, Deserialize)]
pub struct HeadNotFoundResp {
    #[serde(rename = "NoExist")]
    pub no_exist: String,
}

// 元数据
#[derive(Debug, Serialize, Deserialize)]
pub struct ObjectMetadata {
    pub content_type: String,
    pub content_length: i64,
    pub last_modified: String,
    pub file_name: String,
}

// 分片上传tag
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartETag {
    #[serde(rename = "PartNumber")]
    pub part_number: i32,
    #[serde(rename = "ETag")]
    pub etag: String,
}

// S3对象
#[derive(Debug, Serialize, Deserialize)]
pub struct S3Object {
    #[serde(rename = "BucketName")]
    pub bucket_name: String,
    #[serde(rename = "Key")]
    pub key: String,
    #[serde(rename = "MetaData")]
    pub metadata: ObjectMetadata,
}

// 上传文件请求结果
#[derive(Debug, Serialize, Deserialize)]
pub struct UploadFileResp {
    #[serde(rename = "ETag")]
    pub etag: String,
    #[serde(rename = "LastModified")]
    pub last_modified: String,
}

// 桶信息实体
#[derive(Debug, Serialize, Deserialize)]
pub struct BucketWrapper {
    #[serde(rename = "Bucket")]
    pub bucket: Vec<Bucket>,
}

// 桶拥有者实体
#[derive(Debug, Serialize, Deserialize)]
pub struct Owner {
    #[serde(rename = "DisplayName")]
    pub display_name: String,
}

// 桶列表请求结果

#[derive(Debug, Serialize, Deserialize)]
pub struct ListBucketResp {
    #[serde(rename = "Id")]
    pub id: String,
    #[serde(rename = "Owner")]
    pub owner: Owner,
    #[serde(rename = "Buckets")]
    pub buckets: BucketWrapper,
}
