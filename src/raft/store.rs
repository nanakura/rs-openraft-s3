use anyhow::{anyhow, Context};
use std::collections::BTreeMap;
use std::fmt::Debug;
use std::io::Cursor;
use std::ops::RangeBounds;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::api::{BASIC_PATH_SUFFIX, DATA_DIR};
use crate::fs;
use crate::fs::{save_metadata, split_file_ann_save, Metadata};
use crate::model::CompleteMultipartUpload;
use byteorder::BigEndian;
use byteorder::ReadBytesExt;
use byteorder::WriteBytesExt;
use chrono::Utc;
use futures::StreamExt;
use log::{debug, info};
use mime_guess::MimeGuess;
use openraft::storage::LogFlushed;
use openraft::storage::LogState;
use openraft::storage::RaftLogStorage;
use openraft::storage::RaftStateMachine;
use openraft::storage::Snapshot;
use openraft::AnyError;
use openraft::Entry;
use openraft::EntryPayload;
use openraft::ErrorSubject;
use openraft::ErrorVerb;
use openraft::LogId;
use openraft::OptionalSend;
use openraft::RaftLogReader;
use openraft::RaftSnapshotBuilder;
use openraft::SnapshotMeta;
use openraft::StorageError;
use openraft::StorageIOError;
use openraft::StoredMembership;
use openraft::Vote;
use rocksdb::ColumnFamily;
use rocksdb::ColumnFamilyDescriptor;
use rocksdb::Direction;
use rocksdb::Options;
use rocksdb::DB;
use serde::Deserialize;
use serde::Serialize;
use tokio::sync::RwLock;

use crate::raft::typ;
use crate::raft::Node;
use crate::raft::NodeId;
use crate::raft::SnapshotData;
use crate::raft::TypeConfig;
use rayon::prelude::*;

/**
 * Here you will set the types of request that will interact with the raft nodes.
 * For example the `Set` will be used to write data (key and value) to the raft database.
 * The `AddNode` will append a new node to the current existing shared list of nodes.
 * You will want to add any request that can write data in all nodes here.
 */
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Request {
    CreateBucket {
        bucket_name: String,
    },
    DeleteBucket {
        bucket_name: String,
    },
    InitChunk {
        bucket_name: String,
        object_key: String,
        upload_id: String,
    },
    UploadChunk {
        part_number: String,
        upload_id: String,
        hash: String,
        body: Vec<u8>,
    },
    UploadFile {
        file_path: String,
        body: Vec<u8>,
    },
    CombineChunk {
        bucket_name: String,
        object_key: String,
        upload_id: String,
        cmu: String,
    },
    DeleteFile {
        file_path: String,
    },
    CopyFile {
        copy_source: String,
        dest_bucket: String,
        dest_object: String,
    },
}

/**
 * Here you will defined what type of answer you expect from reading the data of a node.
 * In this example it will return a optional value from a given key in
 * the `ExampleRequest.Set`.
 *
 *
 */
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Response {
    pub value: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct StoredSnapshot {
    pub meta: SnapshotMeta<NodeId, Node>,

    /// The data of the state machine at the time of this snapshot.
    pub data: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct StateMachineStore {
    pub data: StateMachineData,

    /// snapshot index is not persisted in this example.
    ///
    /// It is only used as a suffix of snapshot id, and should be globally unique.
    /// In practice, using a timestamp in micro-second would be good enough.
    snapshot_idx: u64,

    /// State machine stores snapshot in db.
    db: Arc<DB>,
}

#[derive(Debug, Clone)]
pub struct StateMachineData {
    pub last_applied_log_id: Option<LogId<NodeId>>,

    pub last_membership: StoredMembership<NodeId, Node>,

    /// State built from applying the raft logs
    pub kvs: Arc<RwLock<BTreeMap<String, String>>>,
}

impl RaftSnapshotBuilder<TypeConfig> for StateMachineStore {
    async fn build_snapshot(&mut self) -> Result<Snapshot<TypeConfig>, StorageError<NodeId>> {
        let last_applied_log = self.data.last_applied_log_id;
        let last_membership = self.data.last_membership.clone();

        let kv_json = {
            let kvs = self.data.kvs.read().await;
            serde_json::to_vec(&*kvs).map_err(|e| StorageIOError::read_state_machine(&e))?
        };

        let snapshot_id = if let Some(last) = last_applied_log {
            format!("{}-{}-{}", last.leader_id, last.index, self.snapshot_idx)
        } else {
            format!("--{}", self.snapshot_idx)
        };

        let meta = SnapshotMeta {
            last_log_id: last_applied_log,
            last_membership,
            snapshot_id,
        };

        let snapshot = StoredSnapshot {
            meta: meta.clone(),
            data: kv_json.clone(),
        };

        self.set_current_snapshot_(snapshot)?;

        Ok(Snapshot {
            meta,
            snapshot: Box::new(Cursor::new(kv_json)),
        })
    }
}

impl StateMachineStore {
    async fn new(db: Arc<DB>) -> Result<StateMachineStore, StorageError<NodeId>> {
        let mut sm = Self {
            data: StateMachineData {
                last_applied_log_id: None,
                last_membership: Default::default(),
                kvs: Arc::new(Default::default()),
            },
            snapshot_idx: 0,
            db,
        };

        let snapshot = sm.get_current_snapshot_()?;
        if let Some(snap) = snapshot {
            sm.update_state_machine_(snap).await?;
        }

        Ok(sm)
    }

    async fn update_state_machine_(
        &mut self,
        snapshot: StoredSnapshot,
    ) -> Result<(), StorageError<NodeId>> {
        let kvs: BTreeMap<String, String> = serde_json::from_slice(&snapshot.data)
            .map_err(|e| StorageIOError::read_snapshot(Some(snapshot.meta.signature()), &e))?;

        self.data.last_applied_log_id = snapshot.meta.last_log_id;
        self.data.last_membership = snapshot.meta.last_membership.clone();
        let mut x = self.data.kvs.write().await;
        *x = kvs;

        Ok(())
    }

    fn get_current_snapshot_(&self) -> StorageResult<Option<StoredSnapshot>> {
        Ok(self
            .db
            .get_cf(self.store(), b"snapshot")
            .map_err(|e| StorageError::IO {
                source: StorageIOError::read(&e),
            })?
            .and_then(|v| serde_json::from_slice(&v).ok()))
    }

    fn set_current_snapshot_(&self, snap: StoredSnapshot) -> StorageResult<()> {
        self.db
            .put_cf(
                self.store(),
                b"snapshot",
                serde_json::to_vec(&snap).unwrap().as_slice(),
            )
            .map_err(|e| StorageError::IO {
                source: StorageIOError::write_snapshot(Some(snap.meta.signature()), &e),
            })?;
        self.flush(
            ErrorSubject::Snapshot(Some(snap.meta.signature())),
            ErrorVerb::Write,
        )?;
        Ok(())
    }

    fn flush(
        &self,
        subject: ErrorSubject<NodeId>,
        verb: ErrorVerb,
    ) -> Result<(), StorageIOError<NodeId>> {
        self.db
            .flush_wal(true)
            .map_err(|e| StorageIOError::new(subject, verb, AnyError::new(&e)))?;
        Ok(())
    }

    fn store(&self) -> &ColumnFamily {
        self.db.cf_handle("store").unwrap()
    }
}

impl RaftStateMachine<TypeConfig> for StateMachineStore {
    type SnapshotBuilder = Self;

    async fn applied_state(
        &mut self,
    ) -> Result<(Option<LogId<NodeId>>, StoredMembership<NodeId, Node>), StorageError<NodeId>> {
        Ok((
            self.data.last_applied_log_id,
            self.data.last_membership.clone(),
        ))
    }

    async fn apply<I>(&mut self, entries: I) -> Result<Vec<Response>, StorageError<NodeId>>
    where
        I: IntoIterator<Item = typ::Entry> + OptionalSend,
        I::IntoIter: OptionalSend,
    {
        let entries = entries.into_iter();
        let mut replies = Vec::with_capacity(entries.size_hint().0);

        for ent in entries {
            self.data.last_applied_log_id = Some(ent.log_id);

            let resp_value = None;

            match ent.payload {
                EntryPayload::Blank => {}
                EntryPayload::Normal(req) => match req {
                    Request::CreateBucket { bucket_name } => {
                        std::fs::create_dir_all(bucket_name)
                            .context("创建桶失败")
                            .unwrap();
                    }
                    Request::DeleteBucket { bucket_name } => {
                        if std::fs::metadata(&bucket_name).is_ok() {
                            std::fs::remove_dir_all(&bucket_name)
                                .context("删除桶失败")
                                .unwrap();
                        }
                    }
                    // Request::Set { key, value } => {
                    //     resp_value = Some(value.clone());
                    //
                    //     let mut st = self.data.kvs.write().await;
                    //     st.insert(key, value);
                    // }
                    Request::InitChunk {
                        bucket_name,
                        object_key,
                        upload_id,
                    } => {
                        let _ = init_chunk(bucket_name, object_key, upload_id).await;
                    }
                    Request::UploadChunk {
                        part_number,
                        upload_id,
                        hash,
                        body,
                    } => {
                        let _ = upload_chunk(&part_number, &upload_id, &hash, body).await;
                    }
                    Request::UploadFile { file_path, body } => {
                        let _ = upload_file(file_path, body).await;
                    }
                    Request::CombineChunk {
                        bucket_name,
                        object_key,
                        upload_id,
                        cmu,
                    } => {
                        let cmu: CompleteMultipartUpload = quick_xml::de::from_str(&cmu).unwrap();
                        let _ = combine_chunk(&bucket_name, &object_key, &upload_id, cmu).await;
                    }
                    Request::DeleteFile { file_path } => {
                        let _ = do_delete_file(file_path).await;
                    }
                    Request::CopyFile {
                        copy_source,
                        dest_bucket,
                        dest_object,
                    } => {
                        let _ = copy_object(&copy_source, &dest_bucket, &dest_object).await;
                    }
                },
                EntryPayload::Membership(mem) => {
                    self.data.last_membership = StoredMembership::new(Some(ent.log_id), mem);
                }
            }

            replies.push(Response { value: resp_value });
        }
        Ok(replies)
    }

    async fn get_snapshot_builder(&mut self) -> Self::SnapshotBuilder {
        self.snapshot_idx += 1;
        self.clone()
    }

    async fn begin_receiving_snapshot(
        &mut self,
    ) -> Result<Box<Cursor<Vec<u8>>>, StorageError<NodeId>> {
        Ok(Box::new(Cursor::new(Vec::new())))
    }

    async fn install_snapshot(
        &mut self,
        meta: &SnapshotMeta<NodeId, Node>,
        snapshot: Box<SnapshotData>,
    ) -> Result<(), StorageError<NodeId>> {
        let new_snapshot = StoredSnapshot {
            meta: meta.clone(),
            data: snapshot.into_inner(),
        };

        self.update_state_machine_(new_snapshot.clone()).await?;

        self.set_current_snapshot_(new_snapshot)?;

        Ok(())
    }

    async fn get_current_snapshot(
        &mut self,
    ) -> Result<Option<Snapshot<TypeConfig>>, StorageError<NodeId>> {
        let x = self.get_current_snapshot_()?;
        Ok(x.map(|s| Snapshot {
            meta: s.meta.clone(),
            snapshot: Box::new(Cursor::new(s.data.clone())),
        }))
    }
}

// 上传文件
async fn upload_file(metainfo_file_path: String, body: Vec<u8>) -> anyhow::Result<()> {
    let file_name = PathBuf::from(&metainfo_file_path)
        .file_name()
        .context("解析文件名失败")?
        .to_string_lossy()
        .to_string();
    let tmp_filename = file_name.clone();
    let file_type = MimeGuess::from_path(Path::new(&tmp_filename))
        .first_or_text_plain()
        .to_string();

    let (file_size, hashcodes) = split_file_ann_save(body, 8 << 20).await?;
    let metainfo = Metadata {
        name: file_name,
        size: file_size as u64,
        file_type: file_type.to_string(),
        time: Utc::now(),
        chunks: hashcodes,
    };
    fs::save_metadata(&metainfo_file_path, &metainfo)?;
    Ok(())
}

// 桶间拷贝对象数据
async fn copy_object(
    copy_source: &str,
    dest_bucket: &str,
    dest_object: &str,
) -> anyhow::Result<()> {
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

    Ok(())
}

// 上传分片
pub(crate) async fn upload_chunk(
    part_number: &str,
    upload_id: &str,
    hash: &str,
    body: Vec<u8>,
) -> anyhow::Result<()> {
    let path = fs::path_from_hash(&hash);
    if fs::is_path_exist(&path.to_string_lossy().to_string()) {
        return Ok(());
    }
    let hash_clone = hash;
    let len = body.len();
    let part_path = PathBuf::from(DATA_DIR)
        .join("tmp")
        .join(upload_id)
        .join(part_number);
    tokio::fs::write(part_path, format!("{}", len))
        .await
        .map_err(|err| anyhow!(err.to_string()))?;
    let body = fs::compress_chunk(std::io::Cursor::new(&body))?;
    fs::save_file(&hash_clone, body)?;
    Ok(())
}

// 初始化分片上传
async fn init_chunk(bucket: String, object_key: String, upload_id: String) -> anyhow::Result<()> {
    let file_size_dir = PathBuf::from(crate::api::DATA_DIR)
        .join("tmp")
        .join(&upload_id);
    let extension = &format!(".meta.{}", &upload_id);
    let mut tmp_dir = PathBuf::from(crate::api::DATA_DIR)
        .join(crate::api::BASIC_PATH_SUFFIX)
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
    Ok(())
}

// 完成分片上传
async fn combine_chunk(
    bucket_name: &str,
    object_key: &str,
    upload_id: &str,
    cmu: CompleteMultipartUpload,
) -> anyhow::Result<()> {
    info!("合并分片，uploadId: {}", upload_id);
    let mut part_etags = cmu.part_etags;

    let mut check = true;
    let mut total_len: u64 = 0;

    let extension = &format!(".meta.{}", &upload_id);
    let mut tmp_metadata_dir = PathBuf::from(crate::api::DATA_DIR)
        .join(crate::api::BASIC_PATH_SUFFIX)
        .join(bucket_name)
        .join(object_key)
        .to_string_lossy()
        .to_string();
    tmp_metadata_dir.push_str(extension);
    let tmp_metadata_dir = PathBuf::from(tmp_metadata_dir);
    if !tmp_metadata_dir.as_path().exists() {
        info!("未初始化");
        return Err(anyhow!("未初始化".to_string()));
    }

    for part_etag in &part_etags {
        if !fs::is_path_exist(&part_etag.etag) {
            check = false;
            break;
        }
        let len_path = PathBuf::from(crate::api::DATA_DIR)
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
        return Err(anyhow!("分片不完整".to_string()));
    }
    part_etags.sort_by_key(|p| p.part_number);
    let chunks: Vec<String> = part_etags.par_iter().map(|p| p.etag.clone()).collect();
    let mut metadata = fs::load_metadata(tmp_metadata_dir.to_string_lossy().as_ref())?;
    info!("读取临时元数据成功");
    metadata.size = total_len;
    metadata.chunks = chunks;
    metadata.time = Utc::now();

    let mut metadata_dir = PathBuf::from(crate::api::DATA_DIR)
        .join(crate::api::BASIC_PATH_SUFFIX)
        .join(bucket_name)
        .join(object_key)
        .to_string_lossy()
        .to_string();
    metadata_dir.push_str(".meta");
    save_metadata(&metadata_dir, &metadata)?;
    info!("保存新元数据成功");
    std::fs::remove_file(tmp_metadata_dir).context("删除临时元数据失败")?;
    std::fs::remove_dir_all(
        PathBuf::from(crate::api::DATA_DIR)
            .join("tmp")
            .join(upload_id),
    )
    .context("删除临时文件夹失败")?;
    Ok(())
}

// 删除文件逻辑
async fn do_delete_file(metainfo_file_path: String) -> anyhow::Result<()> {
    if std::fs::metadata(&metainfo_file_path).is_ok() {
        std::fs::remove_file(&metainfo_file_path).context("删除文件失败")?;
    }
    Ok(())
}

#[derive(Debug, Clone)]
pub struct LogStore {
    db: Arc<DB>,
}
type StorageResult<T> = Result<T, StorageError<NodeId>>;

/// converts an id to a byte vector for storing in the database.
/// Note that we're using big endian encoding to ensure correct sorting of keys
fn id_to_bin(id: u64) -> Vec<u8> {
    let mut buf = Vec::with_capacity(8);
    buf.write_u64::<BigEndian>(id).unwrap();
    buf
}

fn bin_to_id(buf: &[u8]) -> u64 {
    (&buf[0..8]).read_u64::<BigEndian>().unwrap()
}

impl LogStore {
    fn store(&self) -> &ColumnFamily {
        self.db.cf_handle("store").unwrap()
    }

    fn logs(&self) -> &ColumnFamily {
        self.db.cf_handle("logs").unwrap()
    }

    fn flush(
        &self,
        subject: ErrorSubject<NodeId>,
        verb: ErrorVerb,
    ) -> Result<(), StorageIOError<NodeId>> {
        self.db
            .flush_wal(true)
            .map_err(|e| StorageIOError::new(subject, verb, AnyError::new(&e)))?;
        Ok(())
    }

    fn get_last_purged_(&self) -> StorageResult<Option<LogId<u64>>> {
        Ok(self
            .db
            .get_cf(self.store(), b"last_purged_log_id")
            .map_err(|e| StorageIOError::read(&e))?
            .and_then(|v| serde_json::from_slice(&v).ok()))
    }

    fn set_last_purged_(&self, log_id: LogId<u64>) -> StorageResult<()> {
        self.db
            .put_cf(
                self.store(),
                b"last_purged_log_id",
                serde_json::to_vec(&log_id).unwrap().as_slice(),
            )
            .map_err(|e| StorageIOError::write(&e))?;

        self.flush(ErrorSubject::Store, ErrorVerb::Write)?;
        Ok(())
    }

    fn set_committed_(
        &self,
        committed: &Option<LogId<NodeId>>,
    ) -> Result<(), StorageIOError<NodeId>> {
        let json = serde_json::to_vec(committed).unwrap();

        self.db
            .put_cf(self.store(), b"committed", json)
            .map_err(|e| StorageIOError::write(&e))?;

        self.flush(ErrorSubject::Store, ErrorVerb::Write)?;
        Ok(())
    }

    fn get_committed_(&self) -> StorageResult<Option<LogId<NodeId>>> {
        Ok(self
            .db
            .get_cf(self.store(), b"committed")
            .map_err(|e| StorageError::IO {
                source: StorageIOError::read(&e),
            })?
            .and_then(|v| serde_json::from_slice(&v).ok()))
    }

    fn set_vote_(&self, vote: &Vote<NodeId>) -> StorageResult<()> {
        self.db
            .put_cf(self.store(), b"vote", serde_json::to_vec(vote).unwrap())
            .map_err(|e| StorageError::IO {
                source: StorageIOError::write_vote(&e),
            })?;

        self.flush(ErrorSubject::Vote, ErrorVerb::Write)?;
        Ok(())
    }

    fn get_vote_(&self) -> StorageResult<Option<Vote<NodeId>>> {
        Ok(self
            .db
            .get_cf(self.store(), b"vote")
            .map_err(|e| StorageError::IO {
                source: StorageIOError::write_vote(&e),
            })?
            .and_then(|v| serde_json::from_slice(&v).ok()))
    }
}

impl RaftLogReader<TypeConfig> for LogStore {
    async fn try_get_log_entries<RB: RangeBounds<u64> + Clone + Debug + OptionalSend>(
        &mut self,
        range: RB,
    ) -> StorageResult<Vec<Entry<TypeConfig>>> {
        let start = match range.start_bound() {
            std::ops::Bound::Included(x) => id_to_bin(*x),
            std::ops::Bound::Excluded(x) => id_to_bin(*x + 1),
            std::ops::Bound::Unbounded => id_to_bin(0),
        };
        self.db
            .iterator_cf(
                self.logs(),
                rocksdb::IteratorMode::From(&start, Direction::Forward),
            )
            .map(|res| {
                let (id, val) = res.unwrap();
                let entry: StorageResult<Entry<_>> =
                    serde_json::from_slice(&val).map_err(|e| StorageError::IO {
                        source: StorageIOError::read_logs(&e),
                    });
                let id = bin_to_id(&id);

                assert_eq!(Ok(id), entry.as_ref().map(|e| e.log_id.index));
                (id, entry)
            })
            .take_while(|(id, _)| range.contains(id))
            .map(|x| x.1)
            .collect()
    }
}

impl RaftLogStorage<TypeConfig> for LogStore {
    type LogReader = Self;

    async fn get_log_state(&mut self) -> StorageResult<LogState<TypeConfig>> {
        let last = self
            .db
            .iterator_cf(self.logs(), rocksdb::IteratorMode::End)
            .next()
            .and_then(|res| {
                let (_, ent) = res.unwrap();
                Some(
                    serde_json::from_slice::<Entry<TypeConfig>>(&ent)
                        .ok()?
                        .log_id,
                )
            });

        let last_purged_log_id = self.get_last_purged_()?;

        let last_log_id = match last {
            None => last_purged_log_id,
            Some(x) => Some(x),
        };
        Ok(LogState {
            last_purged_log_id,
            last_log_id,
        })
    }

    async fn save_committed(
        &mut self,
        _committed: Option<LogId<NodeId>>,
    ) -> Result<(), StorageError<NodeId>> {
        self.set_committed_(&_committed)?;
        Ok(())
    }

    async fn read_committed(&mut self) -> Result<Option<LogId<NodeId>>, StorageError<NodeId>> {
        let c = self.get_committed_()?;
        Ok(c)
    }

    async fn save_vote(&mut self, vote: &Vote<NodeId>) -> Result<(), StorageError<NodeId>> {
        self.set_vote_(vote)
    }

    async fn read_vote(&mut self) -> Result<Option<Vote<NodeId>>, StorageError<NodeId>> {
        self.get_vote_()
    }

    async fn append<I>(&mut self, entries: I, callback: LogFlushed<TypeConfig>) -> StorageResult<()>
    where
        I: IntoIterator<Item = Entry<TypeConfig>> + Send,
        I::IntoIter: Send,
    {
        for entry in entries {
            let id = id_to_bin(entry.log_id.index);
            assert_eq!(bin_to_id(&id), entry.log_id.index);
            self.db
                .put_cf(
                    self.logs(),
                    id,
                    serde_json::to_vec(&entry).map_err(|e| StorageIOError::write_logs(&e))?,
                )
                .map_err(|e| StorageIOError::write_logs(&e))?;
        }

        callback.log_io_completed(Ok(()));

        Ok(())
    }

    async fn truncate(&mut self, log_id: LogId<NodeId>) -> StorageResult<()> {
        debug!("delete_log: [{:?}, +oo)", log_id);

        let from = id_to_bin(log_id.index);
        let to = id_to_bin(0xff_ff_ff_ff_ff_ff_ff_ff);
        self.db
            .delete_range_cf(self.logs(), &from, &to)
            .map_err(|e| StorageIOError::write_logs(&e).into())
    }

    async fn purge(&mut self, log_id: LogId<NodeId>) -> Result<(), StorageError<NodeId>> {
        debug!("delete_log: [0, {:?}]", log_id);

        self.set_last_purged_(log_id)?;
        let from = id_to_bin(0);
        let to = id_to_bin(log_id.index + 1);
        self.db
            .delete_range_cf(self.logs(), &from, &to)
            .map_err(|e| StorageIOError::write_logs(&e).into())
    }

    async fn get_log_reader(&mut self) -> Self::LogReader {
        self.clone()
    }
}

pub(crate) async fn new_storage<P: AsRef<Path>>(db_path: P) -> (LogStore, StateMachineStore) {
    let mut db_opts = Options::default();
    db_opts.create_missing_column_families(true);
    db_opts.create_if_missing(true);

    let store = ColumnFamilyDescriptor::new("store", Options::default());
    let logs = ColumnFamilyDescriptor::new("logs", Options::default());

    let db = DB::open_cf_descriptors(&db_opts, db_path, vec![store, logs]).unwrap();
    let db = Arc::new(db);

    let log_store = LogStore { db: db.clone() };
    let sm_store = StateMachineStore::new(db).await.unwrap();

    (log_store, sm_store)
}
