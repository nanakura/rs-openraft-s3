use anyhow::{anyhow, Context};
use chrono::{DateTime, Utc};
use futures::Stream;
use hex::ToHex;
use ntex::util::{Bytes};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::fs::File;
use std::io::{self, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use zstd::stream::read::Decoder;
use zstd::stream::write::Encoder;

#[derive(Serialize, Deserialize)]
pub struct Metadata {
    pub name: String,
    pub size: usize,
    #[serde(rename = "type")]
    pub file_type: String,
    //#[serde(serialize_with = "serialize_date", deserialize_with = "deserialize_date")]
    pub time: DateTime<Utc>,
    pub chunks: Vec<String>,
}

static PATH_PREFIX: &str = "data/file";

pub(crate) fn path_from_hash(hash: &str) -> PathBuf {
    let hash_prefix = &hash[0..1];
    let hash_subprefix = &hash[1..3];
    let hash_suffix = &hash[3..];

    PathBuf::from(PATH_PREFIX)
        .join(hash_prefix)
        .join(hash_subprefix)
        .join(hash_suffix)
}

pub(crate) fn save_file(hash_code: &str, data: &[u8]) -> anyhow::Result<()> {
    let file_path = path_from_hash(hash_code);

    fs::create_dir_all(file_path.parent().unwrap())?;

    fs::write(file_path, data)?;

    Ok(())
}

fn get_sha256(data: &[u8]) -> Vec<u8> {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let result = hasher.finalize();
    result.to_vec()
}

fn get_sha256_string(hash: &[u8]) -> String {
    let hash_string: String = hash.encode_hex();
    hash_string[..15].to_uppercase()
}

pub(crate) fn sum_15bit_sha256(data: &[u8]) -> String {
    let sha256 = get_sha256(data);
    get_sha256_string(&sha256)
}

pub(crate) fn compress_chunk(chunk: &[u8]) -> anyhow::Result<Vec<u8>> {
    let mut encoder = Encoder::new(Vec::new(), 0)?;
    encoder.write_all(chunk)?;
    let result = encoder.finish()?;
    Ok(result)
}

#[allow(dead_code)]
fn decompress_chunk(chunk_path: &str) -> anyhow::Result<Vec<u8>> {
    let chunk_file = fs::read(chunk_path)?;
    let mut decoder = Decoder::new(chunk_file.as_slice())?;
    let mut result = Vec::new();
    decoder.read_to_end(&mut result)?;
    Ok(result)
}

pub(crate) fn save_metadata(meta_file_path: &str, metadata: &Metadata) -> anyhow::Result<()> {
    let meta_bytes = serde_json::to_vec(metadata)?;
    fs::create_dir_all(Path::new(meta_file_path).parent().unwrap())?;
    fs::write(meta_file_path, meta_bytes)?;
    Ok(())
}

pub(crate) fn load_metadata(meta_file_path: &str) -> anyhow::Result<Metadata> {
    let metadata_bytes = fs::read(meta_file_path).context("元数据地址不存在")?;
    let metadata = serde_json::from_slice(&metadata_bytes).map_err(|err| anyhow!(err))?;
    Ok(metadata)
}

pub(crate) struct ReadStream {
    readers: Vec<Box<dyn Read + Send + Unpin>>,
}

impl ReadStream {
    pub(crate) fn new(readers: Vec<Box<dyn Read + Send + Unpin>>) -> Self {
        ReadStream { readers }
    }
}

impl Stream for ReadStream {
    type Item = io::Result<Bytes>;

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        let mut buffer = vec![0; 1024];

        let mut total_bytes_read = 0;
        for reader in &mut self.readers {
            match reader.read(&mut buffer).map(|bytes_read| {
                total_bytes_read += bytes_read;
                bytes_read
            }) {
                Ok(bytes_read) => {
                    if bytes_read == 0 {
                        break;
                    }
                }
                Err(error) => return std::task::Poll::Ready(Some(Err(error))),
            }
        }

        if total_bytes_read > 0 {
            buffer.resize(total_bytes_read, 0);
            std::task::Poll::Ready(Some(Ok(Bytes::from(buffer))))
        } else {
            std::task::Poll::Ready(None)
        }
    }
}

pub(crate) async fn multi_decompressed_reader(
    file_paths: &[String],
) -> anyhow::Result<Vec<Box<dyn io::Read + Send + Unpin>>> {
    let mut readers = Vec::new();
    for file_path in file_paths {
        let file = fs::File::open(path_from_hash(file_path))?;
        let decoder = Decoder::new(file)?;

        readers.push(Box::new(decoder) as Box<dyn io::Read + Send + Unpin>);
    }

    Ok(readers)
}

pub(crate) fn is_path_exist(hash: &str) -> bool {
    let path = path_from_hash(hash);
    path.exists()
}

#[allow(dead_code)]
pub(crate) fn split_file(
    mut reader: BufReader<File>,
    chunk_size: usize,
) -> anyhow::Result<Vec<String>> {
    let mut chunks = Vec::new();
    loop {
        let mut buffer = vec![0; chunk_size];
        let read = reader.read(&mut buffer)?;

        if read > 0 {
            let chunk = &buffer[..read];
            let hash_code = sum_15bit_sha256(chunk);
            chunks.push(hash_code.clone());

            if !is_path_exist(&hash_code) {
                let compressed_chunk = compress_chunk(chunk)?;
                save_file(&hash_code, &compressed_chunk)?;
            }
        }

        if read == 0 {
            break;
        }
    }
    Ok(chunks)
}

#[cfg(test)]
mod test {

    #[test]
    fn test1() {}
}
