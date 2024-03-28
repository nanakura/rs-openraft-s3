use crate::pkg::fs;
use std::path::{Path, PathBuf};

#[allow(dead_code)]
pub fn get_file_type(file_path: &str) -> Option<&str> {
    let path = Path::new(file_path);
    path.extension().and_then(|ext| ext.to_str())
}

pub fn file_type_from_meta_info(file_path: &str) -> anyhow::Result<String> {
    let metainfo_path = PathBuf::from(file_path).with_extension(".meta");
    let metadata = fs::load_metadata(&metainfo_path.to_string_lossy().to_string())?;
    Ok(metadata.file_type)
}
