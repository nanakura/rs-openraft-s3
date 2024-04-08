use crate::util::cry;
use anyhow::Context;
use chrono::{DateTime, Utc};
use futures::Stream;
use hex::ToHex;
use ntex::util::{Bytes, BytesMut};
use rkyv::{Archive, Deserialize, Infallible, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::io::{self, Cursor, Read};
use std::path::{Path, PathBuf};
use tokio::io::AsyncReadExt;
use zstd::stream::read::Decoder;
use zstd::zstd_safe::WriteBuf;

#[derive(Archive, Deserialize, Serialize, Debug, PartialEq)]
#[archive(compare(PartialEq), check_bytes)]
#[archive_attr(derive(Debug))]
pub struct Metadata {
    pub name: String,
    pub size: u64,
    pub file_type: String,
    pub time: DateTime<Utc>,
    pub chunks: Vec<String>,
}

const PATH_PREFIX: &str = "data/file";

pub(crate) fn path_from_hash(hash: &str) -> PathBuf {
    let hash_prefix = &hash[0..1];
    let hash_subprefix = &hash[1..3];
    let hash_suffix = &hash[3..];

    PathBuf::from(PATH_PREFIX)
        .join(hash_prefix)
        .join(hash_subprefix)
        .join(hash_suffix)
}

pub(crate) fn save_file(hash_code: &str, mut reader: impl std::io::Read) -> anyhow::Result<()> {
    let file_path = path_from_hash(hash_code);
    fs::create_dir_all(file_path.parent().unwrap())?;
    //tokio::fs::write(file_path, data).await?;
    let mut file = std::fs::File::create(file_path)?;
    std::io::copy(&mut reader, &mut file)?;
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
    //hash_string[..15].to_uppercase()
    hash_string.to_uppercase()
}

pub(crate) async fn sum_sha256(data: &[u8]) -> String {
    let sha256 = get_sha256(data);
    get_sha256_string(&sha256)
}

pub(crate) fn compress_chunk(mut reader: impl std::io::Read) -> anyhow::Result<impl std::io::Read> {
    //let mut encoder = Encoder::new(Vec::new(), 0)?;
    let mut res = Vec::new();
    //std::io::copy(&mut reader, &mut encoder)?;
    zstd::stream::copy_encode(&mut reader, &mut res, 0)?;
    //let result = encoder.finish()?;
    Ok(Cursor::new(res))
}

fn decompress_chunk(chunk_path: &str) -> anyhow::Result<Vec<u8>> {
    let chunk_file = fs::read(chunk_path)?;
    let mut decoder = Decoder::new(chunk_file.as_slice())?;
    let mut result = Vec::new();
    decoder.read_to_end(&mut result)?;
    Ok(result)
}

pub(crate) fn save_metadata(meta_file_path: &str, metadata: &Metadata) -> anyhow::Result<()> {
    let meta_data = rkyv::to_bytes::<_, 256>(metadata)?;
    let meta_data = meta_data.as_slice();
    fs::create_dir_all(Path::new(meta_file_path).parent().unwrap())?;
    let meta_bytes = cry::aes_256_cbc_encrypt(meta_data)?;
    fs::write(meta_file_path, &meta_bytes)?;
    Ok(())
}

pub(crate) fn load_metadata(meta_file_path: &str) -> anyhow::Result<Metadata> {
    let metadata_bytes = fs::read(meta_file_path).context("元数据地址不存在")?;
    let metadata_bytes = cry::aes_256_cbc_decrypt(&metadata_bytes)?;
    let archived = rkyv::check_archived_root::<Metadata>(&metadata_bytes[..]).unwrap();
    let res: Metadata = archived.deserialize(&mut Infallible)?;
    //let res = <Metadata as Deserialize<Metadata, Infallible>>::deserialize(archived, &mut rkyv::Infallible).unwrap();
    Ok(res)
}

pub(crate) struct UncompressStream {
    hashes: Vec<String>,
    idx: usize,
}

impl UncompressStream {
    pub(crate) fn new(hashes: Vec<String>) -> Self {
        UncompressStream { hashes, idx: 0 }
    }
}

impl Stream for UncompressStream {
    type Item = io::Result<Bytes>;

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        if self.idx >= self.hashes.len() {
            std::task::Poll::Ready(None)
        } else {
            let x = &self.hashes[self.idx];
            let path = path_from_hash(x).to_string_lossy().to_string();
            if let Ok(res) = decompress_chunk(&path) {
                self.idx += 1;
                std::task::Poll::Ready(Some(Ok(Bytes::from(res))))
            } else {
                std::task::Poll::Ready(None)
            }
        }
        // if let Some(it) = self.hashes.next() {
        //     let res = decompress_chunk(&it).unwrap();
        //     std::task::Poll::Ready(Some(Ok(Bytes::from(res))))
        // } else {
        // }
    }
}

#[allow(dead_code)]
pub(crate) async fn multi_decompressed_reader(
    file_paths: &[String],
) -> anyhow::Result<Vec<Box<dyn io::Read + Send + Unpin>>> {
    let mut readers = Vec::new();
    for file_path in file_paths {
        let file = fs::File::open(path_from_hash(file_path))?;
        let decoder = Decoder::new(file)?;

        readers.push(Box::new(decoder) as Box<dyn io::Read + Send + Unpin>);
    }
    //let x = readers.into_iter();
    Ok(readers)
}

pub(crate) fn is_path_exist(hash: &str) -> bool {
    let path = path_from_hash(hash);
    path.exists()
}

pub(crate) async fn split_file_ann_save(
    mut reader: Box<dyn tokio::io::AsyncRead + Unpin>,
    chunk_size: usize,
) -> anyhow::Result<(usize, Vec<String>)> {
    let mut chunks = Vec::new();
    let mut size = 0;
    let mut buffer = BytesMut::new();
    loop {
        let read = reader.read_buf(&mut buffer).await?;
        if read > 0 {
            size += read;
            if buffer.len() >= chunk_size {
                let chunk = buffer.split_to(chunk_size);
                let hash_code = sum_sha256(chunk.as_slice()).await;
                chunks.push(hash_code.clone());

                if !is_path_exist(&hash_code) {
                    let compressed_chunk = compress_chunk(std::io::Cursor::new(chunk)).unwrap();
                    save_file(&hash_code, compressed_chunk).unwrap();
                }
            }
        }

        if read == 0 {
            if !buffer.is_empty() {
                let chunk = buffer.as_slice();
                let hash_code = sum_sha256(chunk).await;
                chunks.push(hash_code.clone());

                if !is_path_exist(&hash_code) {
                    let compressed_chunk = compress_chunk(std::io::Cursor::new(chunk)).unwrap();
                    save_file(&hash_code, compressed_chunk).unwrap();
                }
            }
            break;
        }
    }
    Ok((size, chunks))
}

#[cfg(test)]
mod test {
    use crate::fs::Metadata;
    use rkyv::{Deserialize, Infallible};

    #[test]
    fn test1() {
        let m = Metadata {
            name: "xxx".to_string(),
            size: 10,
            file_type: "xxxxx".to_string(),
            time: Default::default(),
            chunks: vec![],
        };

        let bytes = rkyv::to_bytes::<_, 256>(&m).unwrap();
        let bytes = bytes.as_slice();
        let archived = rkyv::check_archived_root::<Metadata>(&bytes[..]).unwrap();
        let res: Metadata = archived.deserialize(&mut Infallible).unwrap();
        assert_eq!(m, res)
    }

    #[test]
    fn test2() {}
}
