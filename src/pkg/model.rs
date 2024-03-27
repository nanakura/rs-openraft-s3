use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct Bucket {
    name: String,
    creation_date: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct CompleteMultipartUpload {
    #[serde(rename = "PartETags")]
    part_etags: Vec<PartETag>,
}

#[derive(Debug, Serialize, Deserialize)]
struct CompleteMultipartUploadResult {
    bucket_name: String,
    object_key: String,
    #[serde(rename = "ETag")]
    etag: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct InitiateMultipartUploadResult {
    bucket: String,
    object_key: String,
    upload_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct ListBucketsResult {
    buckets: Vec<Bucket>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ObjectMetadata {
    content_type: String,
    content_length: i64,
    last_modified: String,
    file_name: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct PartETag {
    part_number: i32,
    #[serde(rename = "ETag")]
    etag: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct S3Object {
    bucket_name: String,
    key: String,
    metadata: ObjectMetadata,
}

#[derive(Debug, Serialize, Deserialize)]
struct UploadFileResp {
    #[serde(rename = "ETag")]
    etag: String,
    last_modified: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct BucketWrapper {
    bucket: Bucket,
}

#[derive(Debug, Serialize, Deserialize)]
struct Owner {
    display_name: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct ListBucketResp {
    id: String,
    owner: Owner,
    buckets: Vec<BucketWrapper>,
}