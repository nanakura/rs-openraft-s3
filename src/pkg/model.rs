use serde::{Deserialize, Serialize};
use std::time::SystemTime;

#[derive(Debug, Serialize, Deserialize)]
pub struct Bucket {
    pub name: String,
    pub creation_date: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CompleteMultipartUpload {
    #[serde(rename = "PartETags")]
    pub part_etags: Vec<PartETag>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CompleteMultipartUploadResult {
    pub bucket_name: String,
    pub object_key: String,
    #[serde(rename = "ETag")]
    pub etag: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InitiateMultipartUploadResult {
    pub bucket: String,
    pub object_key: String,
    pub upload_id: String,
}

// #[derive(Debug, Serialize, Deserialize)]
// pub struct ListBucketsResult {
//     pub buckets: Vec<Bucket>,
// }
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
    #[serde(skip_serializing_if = "Vec::is_empty")]
    // #[serde(rename = "CommonPrefixes")]
    // pub common_prefixes: Vec<String>,
    #[serde(rename = "Contents")]
    pub contents: Vec<Content>,
}

#[derive(Debug, Serialize)]
pub struct Content {
    #[serde(rename = "Key")]
    pub key: String,
    #[serde(rename = "LastModified")]
    pub last_modified: SystemTime,
    #[serde(rename = "Size")]
    pub size: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ObjectMetadata {
    pub content_type: String,
    pub content_length: i64,
    pub last_modified: String,
    pub file_name: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PartETag {
    pub part_number: i32,
    #[serde(rename = "ETag")]
    pub etag: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct S3Object {
    pub bucket_name: String,
    pub key: String,
    pub metadata: ObjectMetadata,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UploadFileResp {
    #[serde(rename = "ETag")]
    pub etag: String,
    pub last_modified: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BucketWrapper {
    pub bucket: Bucket,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Owner {
    pub display_name: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ListBucketResp {
    pub id: String,
    pub owner: Owner,
    pub buckets: Vec<BucketWrapper>,
}
